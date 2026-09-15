use crate::paths;
use crate::runtime::{InstalledMod, UnifiedProfile};

use std::fs;
use std::path::PathBuf;

fn profile_path(profile_id: &str) -> Result<PathBuf, String> {
    Ok(paths::profiles_dir()?.join(profile_id).join("profile.json"))
}

fn load_profile(profile_id: &str) -> Result<UnifiedProfile, String> {
    let path = profile_path(profile_id)?;

    if !path.is_file() {
        return Err(format!("Profile '{}' does not exist.", profile_id,));
    }

    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read {}: {}", path.display(), error,))?;

    serde_json::from_str(&text)
        .map_err(|error| format!("Failed to parse {}: {}", path.display(), error,))
}

fn save_profile(profile: &UnifiedProfile) -> Result<(), String> {
    let path = profile_path(&profile.profile_id)?;

    let text = serde_json::to_string_pretty(profile)
        .map_err(|error| format!("Failed to serialize profile: {}", error,))?;

    fs::write(&path, text).map_err(|error| format!("Failed to save {}: {}", path.display(), error,))
}

#[tauri::command]
pub fn move_mod(
    profile_id: String,
    mod_id: String,
    direction: String,
) -> Result<Vec<InstalledMod>, String> {
    let mut profile = load_profile(&profile_id)?;

    let current_index = profile
        .mods
        .iter()
        .position(|installed_mod| installed_mod.id == mod_id)
        .ok_or_else(|| format!("Mod '{}' was not found.", mod_id,))?;

    let target_index = match direction.as_str() {
        "up" => {
            if current_index == 0 {
                return Ok(profile.mods);
            }

            current_index - 1
        }

        "down" => {
            if current_index + 1 >= profile.mods.len() {
                return Ok(profile.mods);
            }

            current_index + 1
        }

        _ => {
            return Err(format!("Unknown load-order direction '{}'.", direction,));
        }
    };

    profile.mods.swap(current_index, target_index);

    save_profile(&profile)?;

    Ok(profile.mods)
}

#[tauri::command]
pub fn set_mod_order(
    profile_id: String,
    mod_ids: Vec<String>,
) -> Result<Vec<InstalledMod>, String> {
    let mut profile = load_profile(&profile_id)?;

    if mod_ids.len() != profile.mods.len() {
        return Err(
            "The new load order must contain every installed mod exactly once.".to_string(),
        );
    }

    let mut reordered = Vec::with_capacity(profile.mods.len());

    for mod_id in mod_ids {
        let index = profile
            .mods
            .iter()
            .position(|installed_mod| installed_mod.id == mod_id)
            .ok_or_else(|| format!("Unknown or duplicate mod '{}' in load order.", mod_id,))?;

        reordered.push(profile.mods.remove(index));
    }

    if !profile.mods.is_empty() {
        return Err("The new load order contained duplicate mod IDs.".to_string());
    }

    profile.mods = reordered;

    save_profile(&profile)?;

    Ok(profile.mods)
}
