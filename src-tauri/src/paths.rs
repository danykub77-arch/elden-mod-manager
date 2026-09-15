use std::fs;
use std::path::PathBuf;

pub fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "Could not determine home directory.".to_string())
}

pub fn app_data_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            return Ok(PathBuf::from(appdata).join("EldenModManager"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(home_dir()?
            .join("Library")
            .join("Application Support")
            .join("EldenModManager"));
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return Ok(PathBuf::from(xdg).join("elden-mod-manager"));
        }

        return Ok(home_dir()?
            .join(".local")
            .join("share")
            .join("elden-mod-manager"));
    }

    #[allow(unreachable_code)]
    Err("Unsupported operating system.".to_string())
}

pub fn profiles_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("profiles"))
}

pub fn engines_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("engines"))
}

pub fn runtime_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("runtime"))
}

pub fn downloads_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("downloads"))
}

pub fn profile_config_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("profiles.json"))
}

pub fn ensure_app_directories() -> Result<(), String> {
    let directories = [
        app_data_dir()?,
        profiles_dir()?,
        engines_dir()?,
        runtime_dir()?,
        downloads_dir()?,
    ];

    for directory in directories {
        fs::create_dir_all(&directory)
            .map_err(|e| format!("Failed to create {}: {e}", directory.display()))?;
    }

    Ok(())
}
