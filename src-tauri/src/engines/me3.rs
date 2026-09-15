use crate::engines::EngineStatus;
use crate::paths;

use flate2::read::GzDecoder;
use serde::Deserialize;

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/garyttierney/me3/releases/latest";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub fn engine_directory() -> Result<PathBuf, String> {
    Ok(paths::engines_dir()?.join("me3"))
}

fn staging_directory() -> Result<PathBuf, String> {
    Ok(paths::runtime_dir()?.join("me3-install"))
}

pub fn ensure_engine_directory() -> Result<(), String> {
    let directory = engine_directory()?;

    fs::create_dir_all(&directory).map_err(|e| {
        format!(
            "Failed to create me3 engine directory {}: {e}",
            directory.display()
        )
    })
}

fn find_executable() -> Result<Option<PathBuf>, String> {
    let directory = engine_directory()?;

    #[cfg(target_os = "linux")]
    {
        let candidates = [directory.join("bin").join("me3"), directory.join("me3")];

        for candidate in candidates {
            if candidate.is_file() {
                return Ok(Some(candidate));
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let candidates = [
            directory.join("bin").join("me3.exe"),
            directory.join("me3.exe"),
        ];

        for candidate in candidates {
            if candidate.is_file() {
                return Ok(Some(candidate));
            }
        }
    }

    Ok(None)
}

fn clean_version(version: &str) -> String {
    version.trim().trim_start_matches('v').trim().to_string()
}

fn parse_installed_version(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        let token = token.trim().trim_matches('"').trim_matches(',');

        if token
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && token.contains('.')
        {
            return Some(clean_version(token));
        }
    }

    None
}

fn installed_version() -> Result<Option<String>, String> {
    let Some(executable) = find_executable()? else {
        return Ok(None);
    };

    let output = Command::new(&executable)
        .arg("--version")
        .output()
        .map_err(|e| {
            format!(
                "Failed to query me3 version from {}: {e}",
                executable.display()
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let stderr = String::from_utf8_lossy(&output.stderr);

    let combined = format!("{}\n{}", stdout, stderr);

    Ok(parse_installed_version(&combined))
}

async fn fetch_latest_release() -> Result<GithubRelease, String> {
    let response = reqwest::Client::new()
        .get(GITHUB_LATEST_RELEASE_API)
        .header("User-Agent", "Elden-Mod-Manager")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Failed to check latest me3 release: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub release check failed with HTTP status {}.",
            response.status()
        ));
    }

    response
        .json::<GithubRelease>()
        .await
        .map_err(|e| format!("Failed to read GitHub release information: {e}"))
}

fn release_asset<'a>(release: &'a GithubRelease) -> Result<&'a GithubAsset, String> {
    #[cfg(target_os = "linux")]
    {
        let preferred_names = ["me3-linux-amd64.tar.gz", "me3-linux-x86_64.tar.gz"];

        for name in preferred_names {
            if let Some(asset) = release.assets.iter().find(|asset| asset.name == name) {
                return Ok(asset);
            }
        }

        if let Some(asset) = release.assets.iter().find(|asset| {
            let name = asset.name.to_lowercase();

            name.contains("linux") && (name.ends_with(".tar.gz") || name.ends_with(".tgz"))
        }) {
            return Ok(asset);
        }

        return Err(format!(
            "Latest me3 release {} does not contain a supported Linux portable archive.",
            release.tag_name
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let preferred_names = ["me3-windows-amd64.zip", "me3-windows-x86_64.zip"];

        for name in preferred_names {
            if let Some(asset) = release.assets.iter().find(|asset| asset.name == name) {
                return Ok(asset);
            }
        }

        if let Some(asset) = release.assets.iter().find(|asset| {
            let name = asset.name.to_lowercase();

            name.contains("windows") && name.ends_with(".zip")
        }) {
            return Ok(asset);
        }

        return Err(format!(
            "Latest me3 release {} does not contain a supported Windows portable archive.",
            release.tag_name
        ));
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = release;

        Err("Automatic me3 installation is not currently supported on this platform.".to_string())
    }
}

pub async fn status() -> Result<EngineStatus, String> {
    ensure_engine_directory()?;

    let directory = engine_directory()?;

    let executable = find_executable()?;

    let installed_version = installed_version().unwrap_or(None);

    let latest_version = match fetch_latest_release().await {
        Ok(release) => Some(clean_version(&release.tag_name)),

        Err(error) => {
            eprintln!("Could not check latest me3 version: {error}");

            None
        }
    };

    let update_available = match (installed_version.as_ref(), latest_version.as_ref()) {
        (Some(installed), Some(latest)) => installed != latest,

        _ => false,
    };

    Ok(EngineStatus {
        id: "me3".to_string(),

        name: "me3".to_string(),

        description: "Modern Mod Engine successor and preferred backend.".to_string(),

        installed: executable.is_some(),

        preferred: true,

        engine_path: directory.to_string_lossy().into_owned(),

        executable_path: executable.map(|path| path.to_string_lossy().into_owned()),

        installed_version,

        latest_version,

        update_available,
    })
}

fn clear_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to clear directory {}: {e}", path.display()))?;
    }

    fs::create_dir_all(path)
        .map_err(|e| format!("Failed to create directory {}: {e}", path.display()))
}

#[cfg(target_os = "linux")]
fn extract_download(bytes: &[u8], destination: &Path) -> Result<(), String> {
    let cursor = Cursor::new(bytes);

    let decoder = GzDecoder::new(cursor);

    let mut archive = tar::Archive::new(decoder);

    archive
        .unpack(destination)
        .map_err(|e| format!("Failed to extract me3 Linux archive: {e}"))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn extract_download(bytes: &[u8], destination: &Path) -> Result<(), String> {
    let cursor = Cursor::new(bytes);

    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open me3 Windows archive: {e}"))?;

    archive
        .extract(destination)
        .map_err(|e| format!("Failed to extract me3 Windows archive: {e}"))?;

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn extract_download(_bytes: &[u8], _destination: &Path) -> Result<(), String> {
    Err("Automatic me3 installation is not currently supported on this platform.".to_string())
}

fn copy_directory_contents(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|e| format!("Failed to create {}: {e}", destination.display()))?;

    for entry in
        fs::read_dir(source).map_err(|e| format!("Failed to read {}: {e}", source.display()))?
    {
        let entry = entry.map_err(|e| format!("Failed to read extracted file: {e}"))?;

        let source_path = entry.path();

        let destination_path = destination.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory_contents(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)
                .map_err(|e| format!("Failed to copy {}: {e}", source_path.display()))?;
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn make_linux_executable(directory: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let executable = directory.join("bin").join("me3");

    if !executable.exists() {
        return Err(format!(
            "me3 extraction completed, but {} was not found.",
            executable.display()
        ));
    }

    let mut permissions = fs::metadata(&executable)
        .map_err(|e| format!("Failed to read me3 permissions: {e}"))?
        .permissions();

    permissions.set_mode(0o755);

    fs::set_permissions(&executable, permissions)
        .map_err(|e| format!("Failed to make me3 executable: {e}"))?;

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn make_linux_executable(_directory: &Path) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn install_me3() -> Result<EngineStatus, String> {
    paths::ensure_app_directories()?;

    let release = fetch_latest_release().await?;

    let asset = release_asset(&release)?;

    let engine = engine_directory()?;

    let staging = staging_directory()?;

    clear_directory(&staging)?;

    println!("Latest me3 release: {}", release.tag_name);

    println!("Downloading official asset: {}", asset.name);

    let response = reqwest::Client::new()
        .get(&asset.browser_download_url)
        .header("User-Agent", "Elden-Mod-Manager")
        .send()
        .await
        .map_err(|e| format!("Failed to download me3: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "me3 download failed with HTTP status {}.",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read me3 download: {e}"))?;

    println!("Downloaded {} bytes.", bytes.len());

    extract_download(bytes.as_ref(), &staging)?;

    clear_directory(&engine)?;

    copy_directory_contents(&staging, &engine)?;

    make_linux_executable(&engine)?;

    let _ = fs::remove_dir_all(&staging);

    let result = status().await?;

    if !result.installed {
        return Err(
            "me3 installation completed, but the executable could not be verified.".to_string(),
        );
    }

    println!("me3 installed successfully.");

    if let Some(version) = &result.installed_version {
        println!("Installed me3 version: {}", version);
    }

    Ok(result)
}

#[tauri::command]
pub async fn check_me3_status() -> Result<EngineStatus, String> {
    status().await
}
