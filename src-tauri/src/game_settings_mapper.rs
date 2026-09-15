use crate::paths;
use crate::save_game_settings::get_save_game_settings;

use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct SettingChange {
    pub key: String,
    pub label: String,
    pub before: u64,
    pub after: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsComparison {
    pub baseline_exists: bool,
    pub change_count: usize,
    pub changes: Vec<SettingChange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BaselineStatus {
    pub baseline_exists: bool,
}

fn baseline_path() -> Result<PathBuf, String> {
    Ok(paths::app_data_dir()?.join("game-settings-baseline.json"))
}

fn setting_fields() -> &'static [(&'static str, &'static str)] {
    &[
        ("camera_speed", "Camera Speed"),
        ("controller_vibration", "Controller Vibration"),
        ("brightness", "Brightness"),
        ("music_volume", "Music Volume"),
        ("sound_effects_volume", "Sound Effects Volume"),
        ("voice_volume", "Voice Volume"),
        ("master_volume", "Master Volume"),
        ("display_blood", "Display Blood"),
        ("subtitles", "Subtitles"),
        ("hud", "HUD"),
        ("camera_x_axis", "Camera X Axis"),
        ("camera_y_axis", "Camera Y Axis"),
        ("toggle_auto_lockon", "Auto Lock-On"),
        ("camera_auto_wall_recovery", "Automatic Wall Recovery"),
        ("reset_camera_y_axis", "Reset Camera Y Axis"),
        ("cinematic_effects", "Cinematic Effects"),
        ("camera_auto_rotation", "Camera Auto-Rotation"),
        ("perform_matchmaking", "Cross-Region / Matchmaking"),
        ("manual_attack_aim", "Manual Attack Aiming"),
        ("autotarget", "Auto-Target"),
        ("launchsettings", "Launch Setting"),
        ("send_summon_sign", "Send Summon Sign"),
        ("hdr", "HDR"),
        ("hdr_adjust_brightness", "HDR Brightness"),
        ("hdr_maximum_brightness", "HDR Maximum Brightness"),
        ("hdr_adjust_saturation", "HDR Saturation"),
        ("is_raytracing_on", "Ray Tracing Flag"),
        ("mark_new_items", "Mark New Items"),
        ("show_recent_tabs", "Show Recent Tabs"),
        ("show_tutorials", "Show Tutorials"),
    ]
}

fn current_settings_json() -> Result<Value, String> {
    let settings = get_save_game_settings()?;
    if !settings.save_found {
        return Err("Elden Ring save was not found.".to_string());
    }
    serde_json::to_value(settings)
        .map_err(|error| format!("Failed to serialize settings: {}", error))
}

#[tauri::command]
pub fn get_game_settings_baseline_status() -> Result<BaselineStatus, String> {
    Ok(BaselineStatus {
        baseline_exists: baseline_path()?.is_file(),
    })
}

#[tauri::command]
pub fn capture_game_settings_baseline() -> Result<BaselineStatus, String> {
    let current = current_settings_json()?;
    let path = baseline_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create data directory: {}", error))?;
    }

    let json = serde_json::to_string_pretty(&current)
        .map_err(|error| format!("Failed to encode baseline: {}", error))?;

    fs::write(&path, json).map_err(|error| format!("Failed to save baseline: {}", error))?;

    println!("[Game Settings Mapper] Baseline captured.");

    Ok(BaselineStatus {
        baseline_exists: true,
    })
}

#[tauri::command]
pub fn compare_game_settings_baseline() -> Result<SettingsComparison, String> {
    let path = baseline_path()?;

    if !path.is_file() {
        return Ok(SettingsComparison {
            baseline_exists: false,
            change_count: 0,
            changes: Vec::new(),
        });
    }

    let text =
        fs::read_to_string(&path).map_err(|error| format!("Failed to read baseline: {}", error))?;

    let baseline: Value = serde_json::from_str(&text)
        .map_err(|error| format!("Failed to decode baseline: {}", error))?;

    let current = current_settings_json()?;
    let mut changes = Vec::new();

    for &(key, label) in setting_fields() {
        let before = baseline.get(key).and_then(Value::as_u64);
        let after = current.get(key).and_then(Value::as_u64);

        if let (Some(before), Some(after)) = (before, after) {
            if before != after {
                changes.push(SettingChange {
                    key: key.to_string(),
                    label: label.to_string(),
                    before,
                    after,
                });
            }
        }
    }

    let change_count = changes.len();

    Ok(SettingsComparison {
        baseline_exists: true,
        change_count,
        changes,
    })
}

#[tauri::command]
pub fn clear_game_settings_baseline() -> Result<BaselineStatus, String> {
    let path = baseline_path()?;

    if path.is_file() {
        fs::remove_file(&path).map_err(|error| format!("Failed to clear baseline: {}", error))?;
    }

    Ok(BaselineStatus {
        baseline_exists: false,
    })
}
