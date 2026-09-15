use crate::engines::{choose_engine, EngineDecision, EngineKind};

use crate::paths;
use crate::runtime::UnifiedProfile;

use serde::{Deserialize, Serialize};

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub engine: EngineKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub selected_profile: String,
    pub profiles: Vec<Profile>,
}

fn default_config() -> ProfileConfig {
    ProfileConfig {
        selected_profile: "default".to_string(),

        profiles: vec![Profile {
            id: "default".to_string(),
            name: "Default".to_string(),
            engine: EngineKind::Auto,
        }],
    }
}

fn profile_directory(profile_id: &str) -> Result<PathBuf, String> {
    Ok(paths::profiles_dir()?.join(profile_id))
}

fn profile_mods_directory(profile_id: &str) -> Result<PathBuf, String> {
    Ok(profile_directory(profile_id)?.join("mods"))
}

fn unified_profile_path(profile_id: &str) -> Result<PathBuf, String> {
    Ok(profile_directory(profile_id)?.join("profile.json"))
}

fn ensure_profile_storage(profile_id: &str) -> Result<(), String> {
    let directory = profile_mods_directory(profile_id)?;

    fs::create_dir_all(&directory).map_err(|e| {
        format!(
            "Failed to create profile directory {}: {e}",
            directory.display()
        )
    })?;

    let runtime_path = unified_profile_path(profile_id)?;

    if !runtime_path.exists() {
        save_unified_profile(&UnifiedProfile::empty(profile_id))?;
    }

    Ok(())
}

fn save_unified_profile(profile: &UnifiedProfile) -> Result<(), String> {
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize unified profile: {e}"))?;

    let directory = profile_directory(&profile.profile_id)?;

    fs::create_dir_all(&directory).map_err(|e| format!("Failed to create profile storage: {e}"))?;

    fs::write(unified_profile_path(&profile.profile_id)?, json)
        .map_err(|e| format!("Failed to save unified profile: {e}"))
}

fn load_unified_profile(profile_id: &str) -> Result<UnifiedProfile, String> {
    ensure_profile_storage(profile_id)?;

    let path = unified_profile_path(profile_id)?;

    let contents =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse unified profile: {e}"))
}

fn save_config(config: &ProfileConfig) -> Result<(), String> {
    paths::ensure_app_directories()?;

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize profiles: {e}"))?;

    fs::write(paths::profile_config_path()?, json)
        .map_err(|e| format!("Failed to save profiles: {e}"))?;

    for profile in &config.profiles {
        ensure_profile_storage(&profile.id)?;
    }

    Ok(())
}

fn load_config() -> Result<ProfileConfig, String> {
    paths::ensure_app_directories()?;

    let path = paths::profile_config_path()?;

    if !path.exists() {
        let config = default_config();

        save_config(&config)?;

        return Ok(config);
    }

    let contents =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read profiles: {e}"))?;

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse profiles: {e}"))
}

pub fn initialize_profiles() -> Result<(), String> {
    let config = load_config()?;

    save_config(&config)
}

fn sanitize_id(name: &str) -> String {
    let mut output = String::new();

    for character in name.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character);
        } else if matches!(character, ' ' | '-' | '_') && !output.ends_with('-') {
            output.push('-');
        }
    }

    output.trim_matches('-').to_string()
}

#[tauri::command]
pub fn get_profiles() -> Result<ProfileConfig, String> {
    load_config()
}

#[tauri::command]
pub fn create_profile(name: String) -> Result<ProfileConfig, String> {
    let name = name.trim();

    if name.is_empty() {
        return Err("Profile name cannot be empty.".to_string());
    }

    if name.len() > 40 {
        return Err("Profile name is too long.".to_string());
    }

    let mut config = load_config()?;

    let base_id = sanitize_id(name);

    if base_id.is_empty() {
        return Err("Profile name must contain letters or numbers.".to_string());
    }

    let mut id = base_id.clone();

    let mut number = 2;

    while config.profiles.iter().any(|profile| profile.id == id) {
        id = format!("{base_id}-{number}");

        number += 1;
    }

    let profile = Profile {
        id: id.clone(),
        name: name.to_string(),
        engine: EngineKind::Auto,
    };

    config.profiles.push(profile);

    config.selected_profile = id.clone();

    ensure_profile_storage(&id)?;

    save_config(&config)?;

    Ok(config)
}

#[tauri::command]
pub fn select_profile(id: String) -> Result<ProfileConfig, String> {
    let mut config = load_config()?;

    if !config.profiles.iter().any(|profile| profile.id == id) {
        return Err("Profile does not exist.".to_string());
    }

    config.selected_profile = id;

    save_config(&config)?;

    Ok(config)
}

#[tauri::command]
pub fn set_profile_engine(id: String, engine: EngineKind) -> Result<ProfileConfig, String> {
    let mut config = load_config()?;

    let profile = config
        .profiles
        .iter_mut()
        .find(|profile| profile.id == id)
        .ok_or_else(|| "Profile does not exist.".to_string())?;

    profile.engine = engine;

    save_config(&config)?;

    Ok(config)
}

#[tauri::command]
pub fn get_profile_engine_decision(id: String) -> Result<EngineDecision, String> {
    let config = load_config()?;

    let profile = config
        .profiles
        .iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| "Profile does not exist.".to_string())?;

    let unified = load_unified_profile(&id)?;

    Ok(choose_engine(profile.engine.clone(), &unified))
}

#[tauri::command]
pub fn delete_profile(id: String) -> Result<ProfileConfig, String> {
    if id == "default" {
        return Err("The Default profile cannot be deleted.".to_string());
    }

    let mut config = load_config()?;

    if !config.profiles.iter().any(|profile| profile.id == id) {
        return Err("Profile does not exist.".to_string());
    }

    config.profiles.retain(|profile| profile.id != id);

    if config.selected_profile == id {
        config.selected_profile = "default".to_string();
    }

    let directory = profile_directory(&id)?;

    if directory.exists() {
        fs::remove_dir_all(&directory).map_err(|e| format!("Failed to delete profile: {e}"))?;
    }

    save_config(&config)?;

    Ok(config)
}
