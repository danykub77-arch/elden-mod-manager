use crate::engines::EngineStatus;
use crate::paths;

use std::fs;
use std::path::PathBuf;

pub fn engine_directory() -> Result<PathBuf, String> {
    Ok(paths::engines_dir()?.join("modengine2"))
}

pub fn ensure_engine_directory() -> Result<(), String> {
    let directory = engine_directory()?;

    fs::create_dir_all(&directory).map_err(|e| {
        format!(
            "Failed to create Mod Engine 2 directory {}: {e}",
            directory.display()
        )
    })
}

fn find_launcher() -> Result<Option<PathBuf>, String> {
    let directory = engine_directory()?;

    let candidates = [
        directory.join("modengine2_launcher.exe"),
        directory.join("bin").join("modengine2_launcher.exe"),
    ];

    for candidate in candidates {
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

pub fn status() -> Result<EngineStatus, String> {
    ensure_engine_directory()?;

    let directory = engine_directory()?;

    let executable = find_launcher()?;

    Ok(EngineStatus {
        id: "modengine2".to_string(),

        name: "Mod Engine 2".to_string(),

        description: "Legacy compatibility backend for mods that require Mod Engine 2.".to_string(),

        installed: executable.is_some(),

        preferred: false,

        engine_path: directory.to_string_lossy().into_owned(),

        executable_path: executable.map(|path| path.to_string_lossy().into_owned()),

        installed_version: None,

        latest_version: None,

        update_available: false,
    })
}
