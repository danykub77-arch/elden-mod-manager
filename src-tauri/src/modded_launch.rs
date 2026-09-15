use crate::elden_ring;
use crate::engines::me3;
use crate::paths;
use crate::runtime::{ModContentType, UnifiedProfile};

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(target_os = "linux")]
use std::ffi::CString;
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::thread;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::time::Duration;

fn profile_directory(profile_id: &str) -> Result<PathBuf, String> {
    Ok(paths::profiles_dir()?.join(profile_id))
}

fn unified_profile_path(profile_id: &str) -> Result<PathBuf, String> {
    Ok(profile_directory(profile_id)?.join("profile.json"))
}

fn generated_me3_profile_path(profile_id: &str) -> Result<PathBuf, String> {
    Ok(profile_directory(profile_id)?.join("elden-mod-manager.me3"))
}

fn load_unified_profile(profile_id: &str) -> Result<UnifiedProfile, String> {
    let path = unified_profile_path(profile_id)?;

    if !path.is_file() {
        return Err(format!("Profile '{}' does not exist.", profile_id));
    }

    let text =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    serde_json::from_str(&text).map_err(|e| format!("Failed to parse {}: {e}", path.display()))
}

fn enabled_randomizer_executable(profile_id: &str) -> Result<Option<PathBuf>, String> {
    let profile = load_unified_profile(profile_id)?;

    let enabled = profile
        .mods
        .iter()
        .any(|installed_mod| installed_mod.enabled && installed_mod.id == "elden-ring-randomizer");

    if !enabled {
        return Ok(None);
    }

    let randomizer_directory = profile_directory(profile_id)?
        .join("tools")
        .join("randomizer");

    let executable = randomizer_directory.join("EldenRingRandomizer.exe");

    if !executable.is_file() {
        return Err(format!(
            "Elden Ring Randomizer is enabled, but its launcher was not found at {}.",
            executable.display()
        ));
    }

    Ok(Some(executable))
}

fn safe_me3_relative_path(value: &str) -> Result<String, String> {
    let normalized = value.replace('\\', "/");

    if normalized.starts_with('/') || normalized.contains("../") || normalized.contains('\'') {
        return Err(format!("Unsafe mod path in profile: {}", value));
    }

    Ok(normalized)
}

fn write_me3_profile(profile_id: &str) -> Result<PathBuf, String> {
    let profile = load_unified_profile(profile_id)?;

    let profile_path = generated_me3_profile_path(profile_id)?;

    let mut contents = String::from(
        "profileVersion = \"v1\"\n\
start_online = false\n\
\n\
[[supports]]\n\
game = \"eldenring\"\n",
    );

    let randomizer_enabled = profile
        .mods
        .iter()
        .any(|installed_mod| installed_mod.enabled && installed_mod.id == "elden-ring-randomizer");

    if randomizer_enabled {
        contents.push_str("\n[[packages]]\npath = 'tools/randomizer'\n");

        let crash_fix = profile_directory(profile_id)?
            .join("tools")
            .join("randomizer")
            .join("dll")
            .join("RandomizerCrashFix.dll");

        if crash_fix.is_file() {
            contents
                .push_str("\n[[natives]]\npath = 'tools/randomizer/dll/RandomizerCrashFix.dll'\n");
        }
    }

    for installed_mod in &profile.mods {
        if !installed_mod.enabled || installed_mod.id == "elden-ring-randomizer" {
            continue;
        }

        for content in &installed_mod.contents {
            let path = safe_me3_relative_path(&content.relative_path)?;

            match &content.content_type {
                ModContentType::Assets => {
                    contents.push_str("\n[[packages]]\n");

                    contents.push_str(&format!("path = '{}'\n", path));
                }

                ModContentType::NativeDll => {
                    contents.push_str("\n[[natives]]\n");

                    contents.push_str(&format!("path = '{}'\n", path));
                }

                ModContentType::Config | ModContentType::Unknown => {}
            }
        }
    }

    // Randomizer must be the final me3 package. Later packages win file conflicts.
    let randomizer_package = "
[[packages]]
path = 'tools/randomizer'
";
    contents = contents.replace(randomizer_package, "");
    if profile
        .mods
        .iter()
        .any(|m| m.enabled && m.id == "elden-ring-randomizer")
    {
        contents.push_str(randomizer_package);
    }

    fs::write(&profile_path, contents).map_err(|e| {
        format!(
            "Failed to create generated me3 profile {}: {e}",
            profile_path.display()
        )
    })?;

    println!("Generated me3 profile: {}", profile_path.display());

    println!(
        "Enabled mods: {}",
        profile
            .mods
            .iter()
            .filter(|installed_mod| installed_mod.enabled,)
            .count()
    );

    Ok(profile_path)
}

fn find_me3_executable() -> Result<PathBuf, String> {
    let engine = me3::engine_directory()?;

    #[cfg(target_os = "linux")]
    {
        let candidates = [engine.join("bin").join("me3"), engine.join("me3")];

        for candidate in candidates {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        return Err("me3 is not installed. Install it from Runtime Backends first.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let candidates = [engine.join("bin").join("me3.exe"), engine.join("me3.exe")];

        for candidate in candidates {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        return Err("me3 is not installed. Install it from Runtime Backends first.".to_string());
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Err("Modded launching is not currently supported on this platform.".to_string())
    }
}

#[cfg(target_os = "linux")]
fn launch_me3(executable: &Path, profile: &Path) -> Result<(), String> {
    let engine = me3::engine_directory()?;

    let windows_binaries = engine.join("bin").join("win64");

    if !windows_binaries.is_dir() {
        return Err(format!(
            "me3 Windows runtime files were not found at {}.",
            windows_binaries.display()
        ));
    }

    println!("Launching modded Elden Ring through me3.");

    println!("me3 executable: {}", executable.display());

    println!("Windows binaries: {}", windows_binaries.display());

    println!("Profile: {}", profile.display());

    Command::new(executable)
        .arg("--windows-binaries-dir")
        .arg(&windows_binaries)
        .arg("launch")
        .arg("-p")
        .arg(profile)
        .current_dir(&engine)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to start me3: {e}"))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_me3(executable: &Path, profile: &Path) -> Result<(), String> {
    let engine = me3::engine_directory()?;

    Command::new(executable)
        .arg("launch")
        .arg("-p")
        .arg(profile)
        .current_dir(&engine)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to start me3: {e}"))?;

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn launch_me3(_executable: &Path, _profile: &Path) -> Result<(), String> {
    Err("Modded launching is not currently supported on this platform.".to_string())
}

#[cfg(target_os = "linux")]
fn start_randomizer_launch_handoff(
    profile_id: String,
    randomizer_executable: &Path,
) -> Result<(), String> {
    let randomizer_directory = randomizer_executable
        .parent()
        .ok_or_else(|| "Randomizer executable has no parent directory.".to_string())?
        .to_path_buf();

    let modengine_launcher = randomizer_directory
        .join("diste")
        .join("ModEngine")
        .join("modengine2_launcher.exe");

    if !modengine_launcher.is_file() {
        return Err(format!(
            "Randomizer Mod Engine launcher was not found at {}.",
            modengine_launcher.display()
        ));
    }

    thread::spawn(move || {
        // Give the Randomizer time to finish starting before arming the handoff.
        thread::sleep(Duration::from_secs(2));

        if let Err(error) = wait_for_randomizer_modengine_open(&modengine_launcher) {
            eprintln!("Randomizer launch handoff watcher failed: {error}");
            return;
        }

        println!("Randomizer Launch Elden Ring pressed; handing off to me3.");

        // The Randomizer has finished generating its files. Its built-in
        // Mod Engine launcher is not used by this app; me3 takes over here.
        // Close the Randomizer before it can display its misleading
        // "Automatic Mod Engine launcher appeared to fail" dialog.
        let _ = Command::new("pkill")
            .arg("-f")
            .arg("EldenRingRandomizer.exe")
            .status();

        let result = (|| -> Result<(), String> {
            let executable = find_me3_executable()?;
            let profile = write_me3_profile(&profile_id)?;
            launch_me3(&executable, &profile)
        })();

        if let Err(error) = result {
            eprintln!("Failed to launch Randomizer profile through me3: {error}");
        }
    });

    Ok(())
}

#[cfg(target_os = "linux")]
fn wait_for_randomizer_modengine_open(path: &Path) -> Result<(), String> {
    use std::os::raw::{c_char, c_int, c_void};

    const IN_OPEN: u32 = 0x0000_0020;
    const O_CLOEXEC: c_int = 0x0008_0000;

    unsafe extern "C" {
        fn inotify_init1(flags: c_int) -> c_int;
        fn inotify_add_watch(fd: c_int, pathname: *const c_char, mask: u32) -> c_int;
        fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
        fn close(fd: c_int) -> c_int;
    }

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| format!("Invalid Randomizer launcher path: {}", path.display()))?;

    let fd = unsafe { inotify_init1(O_CLOEXEC) };
    if fd < 0 {
        return Err(format!(
            "inotify_init1 failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    let watch = unsafe { inotify_add_watch(fd, c_path.as_ptr(), IN_OPEN) };
    if watch < 0 {
        let error = std::io::Error::last_os_error();
        unsafe { close(fd) };
        return Err(format!("Could not watch {}: {error}", path.display()));
    }

    let mut buffer = [0u8; 4096];
    let read_result = unsafe { read(fd, buffer.as_mut_ptr().cast::<c_void>(), buffer.len()) };
    let read_error = if read_result < 0 {
        Some(std::io::Error::last_os_error())
    } else {
        None
    };

    unsafe { close(fd) };

    if let Some(error) = read_error {
        return Err(format!(
            "Failed while waiting for Randomizer launch: {error}"
        ));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn start_randomizer_launch_handoff(
    profile_id: String,
    randomizer_executable: &Path,
) -> Result<(), String> {
    let randomizer_directory = randomizer_executable
        .parent()
        .ok_or_else(|| "Randomizer executable has no parent directory.".to_string())?
        .to_path_buf();

    let modengine_launcher = randomizer_directory
        .join("diste")
        .join("ModEngine")
        .join("modengine2_launcher.exe");

    if !modengine_launcher.is_file() {
        return Err(format!(
            "Randomizer Mod Engine launcher was not found at {}.",
            modengine_launcher.display()
        ));
    }

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));

        let initial_modified = fs::metadata(&modengine_launcher)
            .and_then(|metadata| metadata.modified())
            .ok();

        loop {
            thread::sleep(Duration::from_millis(250));

            let running = Command::new("tasklist")
                .args(["/FI", "IMAGENAME eq modengine2_launcher.exe", "/NH"])
                .output();

            let detected = match running {
                Ok(output) => String::from_utf8_lossy(&output.stdout)
                    .to_ascii_lowercase()
                    .contains("modengine2_launcher.exe"),
                Err(_) => false,
            };

            if detected {
                break;
            }

            if let (Some(before), Ok(metadata)) =
                (initial_modified, fs::metadata(&modengine_launcher))
            {
                if let Ok(after) = metadata.modified() {
                    if after > before {
                        break;
                    }
                }
            }
        }

        println!("Randomizer Launch Elden Ring pressed; handing off to me3.");

        let _ = Command::new("taskkill")
            .args(["/IM", "EldenRingRandomizer.exe", "/F"])
            .status();

        let _ = Command::new("taskkill")
            .args(["/IM", "modengine2_launcher.exe", "/F"])
            .status();

        let result = (|| -> Result<(), String> {
            let executable = find_me3_executable()?;
            let profile = write_me3_profile(&profile_id)?;
            launch_me3(&executable, &profile)
        })();

        if let Err(error) = result {
            eprintln!("Failed to launch Randomizer profile through me3: {error}");
        }
    });

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn start_randomizer_launch_handoff(
    _profile_id: String,
    _randomizer_executable: &Path,
) -> Result<(), String> {
    Err("Randomizer launch handoff is not supported on this platform.".to_string())
}

#[tauri::command]
pub fn launch_elden_ring_modded(profile_id: String) -> Result<(), String> {
    if let Some(randomizer_executable) = enabled_randomizer_executable(&profile_id)? {
        println!(
            "Elden Ring Randomizer is enabled for profile '{}'.",
            profile_id
        );
        println!("Opening Randomizer instead of launching the game immediately.");

        elden_ring::launch_with_elden_ring_proton(&randomizer_executable)?;
        start_randomizer_launch_handoff(profile_id, &randomizer_executable)?;
        return Ok(());
    }

    let executable = find_me3_executable()?;

    let profile = write_me3_profile(&profile_id)?;

    launch_me3(&executable, &profile)
}
