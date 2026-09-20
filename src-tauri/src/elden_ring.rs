use crate::paths;

use serde::Serialize;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const ELDEN_RING_APP_ID: &str = "1245620";

#[derive(Serialize)]
pub struct EldenRingInstallation {
    found: bool,
    steam_path: Option<String>,
    game_path: Option<String>,
    executable_path: Option<String>,
    proton_prefix: Option<String>,
}

pub(crate) fn find_steam_root() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let home = paths::home_dir().ok()?;

        let candidates = [
            home.join(".local/share/Steam"),
            home.join(".steam/steam"),
            home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        ];

        for path in candidates {
            if path.exists() {
                return Some(path);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut candidates = Vec::new();

        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(program_files_x86).join("Steam"));
        }

        if let Ok(program_files) = std::env::var("ProgramFiles") {
            candidates.push(PathBuf::from(program_files).join("Steam"));
        }

        candidates.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
        candidates.push(PathBuf::from(r"C:\Program Files\Steam"));

        for key in [
            r"HKCU\Software\Valve\Steam",
            r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam",
            r"HKLM\SOFTWARE\Valve\Steam",
        ] {
            if let Ok(output) = Command::new("reg")
                .args(["query", key, "/v", "SteamPath"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    for line in stdout.lines() {
                        if !line.contains("SteamPath") {
                            continue;
                        }

                        if let Some(value) = line.split("REG_SZ").nth(1) {
                            let value = value.trim();

                            if !value.is_empty() {
                                candidates.push(PathBuf::from(value));
                            }
                        }
                    }
                }
            }
        }

        for path in candidates {
            if path.join("steam.exe").is_file() {
                return Some(path);
            }
        }
    }

    None
}

fn parse_library_paths(steam_root: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![steam_root.to_path_buf()];

    let vdf_path = steam_root.join("steamapps/libraryfolders.vdf");

    let Ok(contents) = fs::read_to_string(vdf_path) else {
        return libraries;
    };

    for line in contents.lines() {
        let trimmed = line.trim();

        if !trimmed.starts_with("\"path\"") {
            continue;
        }

        let parts: Vec<&str> = trimmed.split('"').collect();

        if parts.len() < 4 {
            continue;
        }

        let path = parts[3].replace("\\\\", "\\");

        let path = PathBuf::from(path);

        if !libraries.contains(&path) {
            libraries.push(path);
        }
    }

    libraries
}

pub(crate) fn find_elden_ring_executable() -> Option<PathBuf> {
    let steam_root = find_steam_root()?;

    for library in parse_library_paths(&steam_root) {
        let executable = library.join("steamapps/common/ELDEN RING/Game/eldenring.exe");
        if executable.is_file() {
            return Some(executable);
        }
    }

    None
}

#[cfg(target_os = "linux")]
pub(crate) fn find_elden_ring_proton_prefix() -> Option<PathBuf> {
    let steam_root = find_steam_root()?;

    for library in parse_library_paths(&steam_root) {
        let game_executable = library.join("steamapps/common/ELDEN RING/Game/eldenring.exe");

        if !game_executable.exists() {
            continue;
        }

        let prefix = library
            .join("steamapps")
            .join("compatdata")
            .join(ELDEN_RING_APP_ID)
            .join("pfx");

        if prefix.exists() {
            return Some(prefix);
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn proton_candidates(steam_root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let compatibility_tools = steam_root.join("compatibilitytools.d");
    if let Ok(entries) = fs::read_dir(&compatibility_tools) {
        for entry in entries.flatten() {
            let proton = entry.path().join("proton");
            if proton.is_file() {
                candidates.push(proton);
            }
        }
    }

    for library in parse_library_paths(steam_root) {
        let common = library.join("steamapps/common");
        let Ok(entries) = fs::read_dir(&common) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };

            if !name.starts_with("Proton") {
                continue;
            }

            let proton = path.join("proton");
            if proton.is_file() {
                candidates.push(proton);
            }
        }
    }

    candidates.sort();
    candidates.reverse();
    candidates
}

#[cfg(target_os = "linux")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "linux")]
fn update_randomizer_config(path: &Path, game_executable: &Path) -> Result<bool, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;

    if !contents.contains("RandomizerCommon.Properties.Settings") {
        return Ok(false);
    }

    let Some(setting_start) = contents.find("<setting name=\"Exe\"") else {
        return Ok(false);
    };
    let Some(value_offset) = contents[setting_start..].find("<value>") else {
        return Ok(false);
    };

    let value_start = setting_start + value_offset + "<value>".len();
    let Some(value_end_offset) = contents[value_start..].find("</value>") else {
        return Ok(false);
    };
    let value_end = value_start + value_end_offset;

    let escaped_path = xml_escape(&game_executable.to_string_lossy());
    let mut updated = String::with_capacity(contents.len() + escaped_path.len());
    updated.push_str(&contents[..value_start]);
    updated.push_str(&escaped_path);
    updated.push_str(&contents[value_end..]);

    if updated != contents {
        fs::write(path, updated)
            .map_err(|error| format!("Failed to update {}: {error}", path.display()))?;
    }

    println!(
        "Configured Randomizer Elden Ring executable: {} -> {}",
        path.display(),
        game_executable.display()
    );
    Ok(true)
}

#[cfg(target_os = "linux")]
fn update_randomizer_configs_recursive(
    directory: &Path,
    game_executable: &Path,
    updated_count: &mut usize,
) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Ok(());
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            update_randomizer_configs_recursive(&path, game_executable, updated_count)?;
            continue;
        }

        let is_user_config = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("user.config"));

        if is_user_config && update_randomizer_config(&path, game_executable)? {
            *updated_count += 1;
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn configure_randomizer_game_executable(
    working_directory: &Path,
    compat_data: &Path,
) -> Result<(), String> {
    let game_executable = find_elden_ring_executable()
        .ok_or_else(|| "Elden Ring was not found in any configured Steam library.".to_string())?;

    let mut updated_count = 0;
    let users_directory = compat_data.join("pfx/drive_c/users");
    update_randomizer_configs_recursive(&users_directory, &game_executable, &mut updated_count)?;

    // On a fresh prefix, user.config may not exist yet. Updating the application's
    // default settings (when present) lets .NET seed the correct value on first run.
    let app_config = working_directory.join("EldenRingRandomizer.exe.config");
    if app_config.is_file() && update_randomizer_config(&app_config, &game_executable)? {
        updated_count += 1;
    }

    if updated_count == 0 {
        println!(
            "Detected Elden Ring at {}, but no Randomizer settings file exists yet.",
            game_executable.display()
        );
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn launch_with_elden_ring_proton(executable: &Path) -> Result<(), String> {
    const DOTNET_DESKTOP_RUNTIME_URL: &str =
        "https://aka.ms/dotnet/8.0/windowsdesktop-runtime-win-x64.exe";

    let steam_root =
        find_steam_root().ok_or_else(|| "Steam installation was not found.".to_string())?;

    let proton = proton_candidates(&steam_root)
        .into_iter()
        .next()
        .ok_or_else(|| {
            "No Proton installation was found in Steam. Install or run a Proton version for Elden Ring first."
                .to_string()
        })?;

    let working_directory = executable
        .parent()
        .ok_or_else(|| format!("Invalid executable path: {}", executable.display()))?;

    // Keep the Proton runtime outside the Randomizer mod root. me3 recursively scans
    // package roots, while Proton prefixes contain symlinks that escape that root.
    let profile_directory = working_directory
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| {
            format!(
                "Could not resolve the profile directory from {}.",
                working_directory.display()
            )
        })?;

    let runtime_directory = profile_directory.join(".runtime").join("randomizer");
    let legacy_runtime_directory = working_directory.join(".linux-runtime");

    if legacy_runtime_directory.exists() {
        if let Some(runtime_parent) = runtime_directory.parent() {
            fs::create_dir_all(runtime_parent).map_err(|error| {
                format!(
                    "Failed to create Randomizer runtime parent {}: {error}",
                    runtime_parent.display()
                )
            })?;
        }

        if runtime_directory.exists() {
            fs::remove_dir_all(&legacy_runtime_directory).map_err(|error| {
                format!(
                    "Failed to remove obsolete Randomizer runtime {}: {error}",
                    legacy_runtime_directory.display()
                )
            })?;
        } else {
            fs::rename(&legacy_runtime_directory, &runtime_directory).map_err(|error| {
                format!(
                    "Failed to move Randomizer runtime from {} to {}: {error}",
                    legacy_runtime_directory.display(),
                    runtime_directory.display()
                )
            })?;

            println!(
                "Moved Randomizer Proton runtime outside the me3 package: {}",
                runtime_directory.display()
            );
        }
    }
    let compat_data = runtime_directory.join("compatdata");
    let dotnet_installer = runtime_directory.join("windowsdesktop-runtime-8-x64.exe");
    let dotnet_marker = runtime_directory.join(".dotnet8-desktop-installed");

    fs::create_dir_all(&compat_data).map_err(|error| {
        format!(
            "Failed to create Randomizer Proton runtime directory {}: {error}",
            compat_data.display()
        )
    })?;

    if !dotnet_marker.is_file() {
        println!("Randomizer .NET 8 Desktop Runtime is not installed yet.");

        if !dotnet_installer.is_file() {
            println!("Downloading Microsoft .NET 8 Desktop Runtime...");

            let status = Command::new("curl")
                .arg("-L")
                .arg("--fail")
                .arg("--show-error")
                .arg("--progress-bar")
                .arg("-o")
                .arg(&dotnet_installer)
                .arg(DOTNET_DESKTOP_RUNTIME_URL)
                .status()
                .map_err(|error| {
                    format!(
                        "Failed to start curl while downloading .NET 8 Desktop Runtime: {error}"
                    )
                })?;

            if !status.success() {
                let _ = fs::remove_file(&dotnet_installer);
                return Err(format!(
                    "Downloading .NET 8 Desktop Runtime failed with status {status}."
                ));
            }
        }

        println!("Installing .NET 8 Desktop Runtime into the Randomizer Proton prefix...");

        let status = Command::new(&proton)
            .arg("run")
            .arg(&dotnet_installer)
            .arg("/install")
            .arg("/quiet")
            .arg("/norestart")
            .current_dir(&runtime_directory)
            .env("STEAM_COMPAT_DATA_PATH", &compat_data)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &steam_root)
            .status()
            .map_err(|error| {
                format!(
                    "Failed to run the .NET 8 Desktop Runtime installer through Proton: {error}"
                )
            })?;

        if !status.success() {
            return Err(format!(
                ".NET 8 Desktop Runtime installation failed with status {status}."
            ));
        }

        fs::write(
            &dotnet_marker,
            b"installed
",
        )
        .map_err(|error| {
            format!(
                ".NET installed, but the launcher could not write marker {}: {error}",
                dotnet_marker.display()
            )
        })?;

        println!(".NET 8 Desktop Runtime installation finished.");
    }

    configure_randomizer_game_executable(working_directory, &compat_data)?;

    println!("Launching Elden Ring Randomizer in its dedicated Proton prefix.");
    println!("Proton: {}", proton.display());
    println!("Compat data: {}", compat_data.display());
    println!("Executable: {}", executable.display());

    Command::new(&proton)
        .arg("run")
        .arg(executable)
        .current_dir(working_directory)
        .env("STEAM_COMPAT_DATA_PATH", &compat_data)
        .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &steam_root)
        .spawn()
        .map_err(|error| {
            format!(
                "Failed to launch {} through Proton: {error}",
                executable.display()
            )
        })?;

    Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn launch_with_elden_ring_proton(executable: &Path) -> Result<(), String> {
    let working_directory = executable
        .parent()
        .ok_or_else(|| format!("Invalid executable path: {}", executable.display()))?;

    Command::new(executable)
        .current_dir(working_directory)
        .spawn()
        .map_err(|error| format!("Failed to launch {}: {error}", executable.display()))?;

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(crate) fn launch_with_elden_ring_proton(_executable: &Path) -> Result<(), String> {
    Err("Launching Windows tools is not supported on this platform.".to_string())
}

#[cfg(target_os = "linux")]
fn find_proton_windows_user(prefix: &Path) -> Result<PathBuf, String> {
    let users = prefix.join("drive_c/users");

    let entries = fs::read_dir(&users).map_err(|error| {
        format!(
            "Could not read Proton users directory {}: {error}",
            users.display(),
        )
    })?;

    /*
     * Prefer Steam's normal Proton user.
     */
    let steamuser = users.join("steamuser");

    if steamuser.exists() {
        return Ok(steamuser);
    }

    /*
     * Fallback for unusual prefixes.
     */
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;

        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name();

        let name = name.to_string_lossy();

        if name.eq_ignore_ascii_case("public") {
            continue;
        }

        if name.eq_ignore_ascii_case("default") {
            continue;
        }

        return Ok(path);
    }

    Err("Could not find the Windows user directory inside Elden Ring's Proton prefix.".to_string())
}

pub fn elden_ring_config_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "linux")]
    {
        let prefix =
        find_elden_ring_proton_prefix()
        .ok_or_else(|| {
            "Elden Ring's Proton prefix was not found. Launch Elden Ring through Steam at least once."
            .to_string()
        })?;

        let windows_user = find_proton_windows_user(&prefix)?;

        return Ok(windows_user
            .join("AppData")
            .join("Roaming")
            .join("EldenRing"));
    }

    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").map_err(|_| "APPDATA is not available.".to_string())?;

        return Ok(PathBuf::from(appdata).join("EldenRing"));
    }

    #[allow(unreachable_code)]
    Err("Elden Ring game settings are not supported on this platform yet.".to_string())
}

#[tauri::command]
pub fn detect_elden_ring() -> EldenRingInstallation {
    let Some(steam_root) = find_steam_root() else {
        return EldenRingInstallation {
            found: false,
            steam_path: None,
            game_path: None,
            executable_path: None,
            proton_prefix: None,
        };
    };

    for library in parse_library_paths(&steam_root) {
        let game_path = library.join("steamapps/common/ELDEN RING/Game");

        let executable = game_path.join("eldenring.exe");

        if !executable.exists() {
            continue;
        }

        #[cfg(target_os = "linux")]
        let proton_prefix = {
            let prefix = library
                .join("steamapps")
                .join("compatdata")
                .join(ELDEN_RING_APP_ID)
                .join("pfx");

            prefix
                .exists()
                .then(|| prefix.to_string_lossy().into_owned())
        };

        #[cfg(not(target_os = "linux"))]
        let proton_prefix: Option<String> = None;

        return EldenRingInstallation {
            found: true,

            steam_path: Some(steam_root.to_string_lossy().into_owned()),

            game_path: Some(game_path.to_string_lossy().into_owned()),

            executable_path: Some(executable.to_string_lossy().into_owned()),

            proton_prefix,
        };
    }

    EldenRingInstallation {
        found: false,

        steam_path: Some(steam_root.to_string_lossy().into_owned()),

        game_path: None,
        executable_path: None,
        proton_prefix: None,
    }
}

#[tauri::command]
pub fn launch_elden_ring_vanilla() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        Command::new("steam")
            .arg(format!("steam://rungameid/{ELDEN_RING_APP_ID}",))
            .spawn()
            .map_err(|error| format!("Failed to launch Steam: {error}",))?;

        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "/C",
                "start",
                "",
                &format!("steam://rungameid/{ELDEN_RING_APP_ID}",),
            ])
            .spawn()
            .map_err(|error| format!("Failed to launch Steam: {error}",))?;

        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Vanilla launching is not implemented on this platform.".to_string())
}
