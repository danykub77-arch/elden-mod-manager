use deku::ctx::Endian;
use deku::reader::Reader;
use er_save_lib::save::user_data_10::UserData10;
use serde::{Deserialize, Serialize};

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::elden_ring::elden_ring_config_dir;

const USER_DATA_10_OFFSET: usize = 0x19003A0;
const USER_DATA_10_SIZE: usize = 0x60010;

const USER_DATA_10_CHECKSUM_SIZE: usize = 0x10;
const USER_DATA_10_PAYLOAD_SIZE: usize = 0x60000;

// USER_DATA_10 layout:
//
// 0x00..0x10   MD5 checksum
// 0x10..0x14   version
// 0x14..0x1C   Steam ID
// 0x1C..       Settings
//
// These offsets below are relative to the start of Settings.
const SETTING_CAMERA_SPEED: usize = 0x00;
const SETTING_CONTROLLER_VIBRATION: usize = 0x01;
const SETTING_BRIGHTNESS: usize = 0x02;
const SETTING_MUSIC_VOLUME: usize = 0x04;
const SETTING_SOUND_EFFECTS_VOLUME: usize = 0x05;
const SETTING_VOICE_VOLUME: usize = 0x06;
const SETTING_DISPLAY_BLOOD: usize = 0x07;
const SETTING_SUBTITLES: usize = 0x08;
const SETTING_HUD: usize = 0x09;
const SETTING_CAMERA_X_AXIS: usize = 0x0A;
const SETTING_CAMERA_Y_AXIS: usize = 0x0B;
const SETTING_TOGGLE_AUTO_LOCKON: usize = 0x0C;
const SETTING_CAMERA_AUTO_WALL_RECOVERY: usize = 0x0D;
const SETTING_RESET_CAMERA_Y_AXIS: usize = 0x10;
const SETTING_CINEMATIC_EFFECTS: usize = 0x11;
const SETTING_PERFORM_MATCHMAKING: usize = 0x13;
const SETTING_MANUAL_ATTACK_AIM: usize = 0x16;
const SETTING_AUTOTARGET: usize = 0x17;
const SETTING_LAUNCHSETTINGS: usize = 0x18;
const SETTING_SEND_SUMMON_SIGN: usize = 0x19;
const SETTING_HDR: usize = 0x1B;
const SETTING_HDR_ADJUST_BRIGHTNESS: usize = 0x1C;
const SETTING_HDR_ADJUST_SATURATION: usize = 0x1E;
const SETTING_MASTER_VOLUME: usize = 0x20;
const SETTING_IS_RAYTRACING_ON: usize = 0x21;
const SETTING_MARK_NEW_ITEMS: usize = 0x22;
const SETTING_SHOW_RECENT_TABS: usize = 0x23;
const SETTING_SHOW_TUTORIALS: usize = 0x2E;
const SETTING_CAMERA_AUTO_ROTATION: usize = 0x2F;

#[derive(Debug, Clone, Serialize)]
pub struct SaveGameSettings {
    pub save_found: bool,
    pub save_path: String,
    pub backup_available: bool,

    pub camera_speed: u8,
    pub controller_vibration: u8,
    pub brightness: u8,

    pub music_volume: u8,
    pub sound_effects_volume: u8,
    pub voice_volume: u8,
    pub master_volume: u8,

    pub display_blood: u8,
    pub subtitles: u8,
    pub hud: u8,

    pub camera_x_axis: u8,
    pub camera_y_axis: u8,

    pub toggle_auto_lockon: u8,
    pub camera_auto_wall_recovery: u8,
    pub reset_camera_y_axis: u8,
    pub cinematic_effects: u8,
    pub camera_auto_rotation: u8,

    pub perform_matchmaking: u8,
    pub manual_attack_aim: u8,
    pub autotarget: u8,
    pub launchsettings: u8,
    pub send_summon_sign: u8,

    pub hdr: u8,
    pub hdr_adjust_brightness: u8,
    pub hdr_maximum_brightness: u8,
    pub hdr_adjust_saturation: u8,

    pub is_raytracing_on: u8,
    pub mark_new_items: u8,
    pub show_recent_tabs: u8,
    pub show_tutorials: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EditableSaveGameSettings {
    pub camera_speed: u8,
    pub controller_vibration: u8,
    pub brightness: u8,
    pub music_volume: u8,
    pub sound_effects_volume: u8,
    pub voice_volume: u8,
    pub master_volume: u8,

    pub display_blood: u8,
    pub subtitles: u8,
    pub hud: u8,
    pub camera_x_axis: u8,
    pub camera_y_axis: u8,
    pub toggle_auto_lockon: u8,
    pub camera_auto_wall_recovery: u8,
    pub reset_camera_y_axis: u8,
    pub cinematic_effects: u8,
    pub camera_auto_rotation: u8,

    pub perform_matchmaking: u8,
    pub manual_attack_aim: u8,
    pub autotarget: u8,
    pub launchsettings: u8,
    pub send_summon_sign: u8,

    pub hdr: u8,
    pub hdr_adjust_brightness: u8,
    pub hdr_adjust_saturation: u8,
    pub is_raytracing_on: u8,
    pub mark_new_items: u8,
    pub show_recent_tabs: u8,
    pub show_tutorials: u8,
}

impl SaveGameSettings {
    fn missing(path: &Path) -> Self {
        Self {
            save_found: false,
            save_path: path.display().to_string(),
            backup_available: backup_path(path).is_file(),

            camera_speed: 0,
            controller_vibration: 0,
            brightness: 0,

            music_volume: 0,
            sound_effects_volume: 0,
            voice_volume: 0,
            master_volume: 0,

            display_blood: 0,
            subtitles: 0,
            hud: 0,

            camera_x_axis: 0,
            camera_y_axis: 0,

            toggle_auto_lockon: 0,
            camera_auto_wall_recovery: 0,
            reset_camera_y_axis: 0,
            cinematic_effects: 0,
            camera_auto_rotation: 0,

            perform_matchmaking: 0,
            manual_attack_aim: 0,
            autotarget: 0,
            launchsettings: 0,
            send_summon_sign: 0,

            hdr: 0,
            hdr_adjust_brightness: 0,
            hdr_maximum_brightness: 0,
            hdr_adjust_saturation: 0,

            is_raytracing_on: 0,
            mark_new_items: 0,
            show_recent_tabs: 0,
            show_tutorials: 0,
        }
    }
}

fn find_save_file() -> Result<PathBuf, String> {
    let elden_ring_dir = elden_ring_config_dir()?;

    if !elden_ring_dir.exists() {
        return Err(format!(
            "Elden Ring data directory does not exist: {}",
            elden_ring_dir.display()
        ));
    }

    let mut candidates = Vec::new();

    for entry in fs::read_dir(&elden_ring_dir).map_err(|error| {
        format!(
            "Failed to read Elden Ring data directory {}: {}",
            elden_ring_dir.display(),
            error
        )
    })? {
        let entry = entry
            .map_err(|error| format!("Failed to inspect Elden Ring data directory: {}", error))?;

        let directory = entry.path();

        if !directory.is_dir() {
            continue;
        }

        let Some(name) = directory.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if !name.chars().all(|character| character.is_ascii_digit()) {
            continue;
        }

        let save = directory.join("ER0000.sl2");

        if save.is_file() {
            candidates.push(save);
        }
    }

    if candidates.is_empty() {
        return Ok(elden_ring_dir.join("<SteamID>").join("ER0000.sl2"));
    }

    candidates.sort_by(|a, b| {
        let a_modified = fs::metadata(a)
            .and_then(|metadata| metadata.modified())
            .ok();

        let b_modified = fs::metadata(b)
            .and_then(|metadata| metadata.modified())
            .ok();

        b_modified.cmp(&a_modified)
    });

    Ok(candidates.remove(0))
}

fn backup_path(save_path: &Path) -> PathBuf {
    save_path.with_file_name("ER0000.sl2.emm-backup")
}

fn pre_restore_path(save_path: &Path) -> PathBuf {
    save_path.with_file_name("ER0000.sl2.emm-pre-restore")
}

fn temporary_path(save_path: &Path) -> PathBuf {
    save_path.with_file_name("ER0000.sl2.emm-tmp")
}

fn verify_save_bytes(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 4 || &bytes[0..4] != b"BND4" {
        return Err("File is not a valid PC Elden Ring BND4 save.".to_string());
    }

    let required = USER_DATA_10_OFFSET
        .checked_add(USER_DATA_10_SIZE)
        .ok_or_else(|| "USER_DATA_10 offset overflow.".to_string())?;

    if bytes.len() < required {
        return Err(format!(
            "Save is too small for USER_DATA_10. Expected at least {} bytes, found {}.",
            required,
            bytes.len()
        ));
    }

    Ok(())
}

fn parse_user_data_10(save_bytes: &[u8]) -> Result<UserData10, String> {
    verify_save_bytes(save_bytes)?;

    let end = USER_DATA_10_OFFSET + USER_DATA_10_SIZE;

    let section = &save_bytes[USER_DATA_10_OFFSET..end];

    let mut cursor = Cursor::new(section);

    let mut reader = Reader::new(&mut cursor);

    UserData10::read(&mut reader, Endian::Little, 0, USER_DATA_10_SIZE, false)
        .map_err(|error| format!("Failed to parse USER_DATA_10 settings block: {}", error))
}

fn settings_offset() -> usize {
    USER_DATA_10_OFFSET + USER_DATA_10_CHECKSUM_SIZE + 4 + 8
}

fn set_setting_byte(bytes: &mut [u8], relative_offset: usize, value: u8) -> Result<(), String> {
    let absolute = settings_offset()
        .checked_add(relative_offset)
        .ok_or_else(|| "Settings offset overflow.".to_string())?;

    let Some(target) = bytes.get_mut(absolute) else {
        return Err(format!(
            "Setting offset 0x{:X} is outside the save.",
            absolute
        ));
    };

    *target = value;

    Ok(())
}

fn recalculate_user_data_10_checksum(bytes: &mut [u8]) -> Result<(), String> {
    verify_save_bytes(bytes)?;

    let payload_start = USER_DATA_10_OFFSET + USER_DATA_10_CHECKSUM_SIZE;

    let payload_end = payload_start + USER_DATA_10_PAYLOAD_SIZE;

    let checksum = md5::compute(&bytes[payload_start..payload_end]);

    bytes[USER_DATA_10_OFFSET..USER_DATA_10_OFFSET + USER_DATA_10_CHECKSUM_SIZE]
        .copy_from_slice(&checksum.0);

    Ok(())
}

fn validate_direct_setting(name: &str, value: u8) -> Result<(), String> {
    if value > 10 {
        return Err(format!(
            "{} must be between 0 and 10. Received {}.",
            name, value
        ));
    }

    Ok(())
}

fn validate_enum(name: &str, value: u8, max: u8) -> Result<(), String> {
    if value > max {
        return Err(format!(
            "{} must be between 0 and {}. Received {}.",
            name, max, value
        ));
    }
    Ok(())
}

fn validate_editable_settings(settings: &EditableSaveGameSettings) -> Result<(), String> {
    validate_direct_setting("Camera Speed", settings.camera_speed)?;
    validate_direct_setting("Controller Vibration", settings.controller_vibration)?;
    validate_direct_setting("Brightness", settings.brightness)?;
    validate_direct_setting("Music Volume", settings.music_volume)?;
    validate_direct_setting("Sound Effects Volume", settings.sound_effects_volume)?;
    validate_direct_setting("Voice Volume", settings.voice_volume)?;
    validate_direct_setting("Master Volume", settings.master_volume)?;
    validate_direct_setting("HDR Brightness", settings.hdr_adjust_brightness)?;
    validate_direct_setting("HDR Saturation", settings.hdr_adjust_saturation)?;

    validate_enum("Display Blood", settings.display_blood, 2)?;
    validate_enum("Subtitles", settings.subtitles, 1)?;
    validate_enum("HUD", settings.hud, 2)?;
    validate_enum("Camera X Axis", settings.camera_x_axis, 1)?;
    validate_enum("Camera Y Axis", settings.camera_y_axis, 1)?;
    validate_enum("Auto Lock-On", settings.toggle_auto_lockon, 1)?;
    validate_enum(
        "Automatic Wall Recovery",
        settings.camera_auto_wall_recovery,
        1,
    )?;
    validate_enum("Reset Camera Y Axis", settings.reset_camera_y_axis, 1)?;
    validate_enum("Cinematic Effects", settings.cinematic_effects, 1)?;
    validate_enum("Camera Auto-Rotation", settings.camera_auto_rotation, 1)?;
    validate_enum("Cross-Region Play", settings.perform_matchmaking, 1)?;
    validate_enum("Manual Attack Aiming", settings.manual_attack_aim, 1)?;
    validate_enum("Auto-Target", settings.autotarget, 1)?;
    validate_enum("Launch Setting", settings.launchsettings, 1)?;
    validate_enum("Send Summon Sign", settings.send_summon_sign, 1)?;
    validate_enum("HDR", settings.hdr, 1)?;
    validate_enum("Ray Tracing", settings.is_raytracing_on, 1)?;
    validate_enum("Mark New Items", settings.mark_new_items, 1)?;
    validate_enum("Show Recent Tabs", settings.show_recent_tabs, 1)?;
    validate_enum("Show Tutorials", settings.show_tutorials, 1)?;

    Ok(())
}

fn create_original_backup(save_path: &Path) -> Result<PathBuf, String> {
    let backup = backup_path(save_path);

    if backup.is_file() {
        return Ok(backup);
    }

    fs::copy(save_path, &backup).map_err(|error| {
        format!(
            "Failed to create save backup {}: {}",
            backup.display(),
            error
        )
    })?;

    println!(
        "[Game Settings] Created original save backup: {}",
        backup.display()
    );

    Ok(backup)
}

fn replace_save(save_path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = temporary_path(save_path);

    if temp.exists() {
        let _ = fs::remove_file(&temp);
    }

    fs::write(&temp, bytes).map_err(|error| {
        format!(
            "Failed to write temporary save {}: {}",
            temp.display(),
            error
        )
    })?;

    verify_save_bytes(
        &fs::read(&temp).map_err(|error| format!("Failed to verify temporary save: {}", error))?,
    )?;

    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(&temp, save_path)
            .map_err(|error| format!("Failed to replace Elden Ring save: {}", error))?;
    }

    #[cfg(target_os = "windows")]
    {
        if save_path.exists() {
            fs::remove_file(save_path).map_err(|error| {
                format!("Failed to prepare Elden Ring save replacement: {}", error)
            })?;
        }

        fs::rename(&temp, save_path)
            .map_err(|error| format!("Failed to replace Elden Ring save: {}", error))?;
    }

    Ok(())
}

fn settings_from_bytes(save_path: &Path, bytes: &[u8]) -> Result<SaveGameSettings, String> {
    let user_data_10 = parse_user_data_10(bytes)?;

    let settings = &user_data_10.settings;

    Ok(SaveGameSettings {
        save_found: true,
        save_path: save_path.display().to_string(),

        backup_available: backup_path(save_path).is_file(),

        camera_speed: settings.camera_speed,

        controller_vibration: settings.controller_vibration,

        brightness: settings.brightness,

        music_volume: settings.music_volume,

        sound_effects_volume: settings.sound_effects_volume,

        voice_volume: settings.voice_volume,

        master_volume: settings.master_volume,

        display_blood: settings.display_blood,

        subtitles: settings.subtitles,

        hud: settings.hud,

        camera_x_axis: settings.camera_x_axis,

        camera_y_axis: settings.camera_y_axis,

        toggle_auto_lockon: settings.toggle_auto_lockon,

        camera_auto_wall_recovery: settings.camera_auto_wall_recovery,

        reset_camera_y_axis: settings.reset_camera_y_axis,

        cinematic_effects: settings.cinematic_effects,

        camera_auto_rotation: settings.camera_auto_rotation,

        perform_matchmaking: settings.perform_matchmaking,

        manual_attack_aim: settings.manual_attack_aim,

        autotarget: settings.autotarget,

        launchsettings: settings.launchsettings,

        send_summon_sign: settings.send_summon_sign,

        hdr: settings.hdr,

        hdr_adjust_brightness: settings.hdr_adjust_brightness,

        hdr_maximum_brightness: settings.hdr_maximum_brightness,

        hdr_adjust_saturation: settings.hdr_adjust_saturation,

        is_raytracing_on: settings.is_raytracing_on,

        mark_new_items: settings.mark_new_items,

        show_recent_tabs: settings.show_recent_tabs,

        show_tutorials: settings.show_tutorials,
    })
}

#[tauri::command]
pub fn get_save_game_settings() -> Result<SaveGameSettings, String> {
    let save_path = find_save_file()?;

    if !save_path.is_file() {
        return Ok(SaveGameSettings::missing(&save_path));
    }

    let bytes = fs::read(&save_path).map_err(|error| {
        format!(
            "Failed to read Elden Ring save {}: {}",
            save_path.display(),
            error
        )
    })?;

    let result = settings_from_bytes(&save_path, &bytes).map_err(|error| {
        let message = format!(
            "Failed to read Elden Ring settings from {}: {}",
            save_path.display(),
            error
        );

        eprintln!("[Game Settings] {}", message);

        message
    })?;

    println!(
        "[Game Settings] USER_DATA_10 parsed successfully from {}",
        save_path.display()
    );

    Ok(result)
}

#[tauri::command]
pub fn save_save_game_settings(
    settings: EditableSaveGameSettings,
) -> Result<SaveGameSettings, String> {
    validate_editable_settings(&settings)?;

    let save_path = find_save_file()?;

    if !save_path.is_file() {
        return Err("ER0000.sl2 was not found.".to_string());
    }

    let mut bytes = fs::read(&save_path).map_err(|error| {
        format!(
            "Failed to read Elden Ring save {}: {}",
            save_path.display(),
            error
        )
    })?;

    // Validate before touching anything.
    verify_save_bytes(&bytes)?;

    parse_user_data_10(&bytes)?;

    // Preserve the users original save the first time
    // this launcher ever modifies it.
    create_original_backup(&save_path)?;

    set_setting_byte(&mut bytes, SETTING_CAMERA_SPEED, settings.camera_speed)?;
    set_setting_byte(
        &mut bytes,
        SETTING_CONTROLLER_VIBRATION,
        settings.controller_vibration,
    )?;
    set_setting_byte(&mut bytes, SETTING_BRIGHTNESS, settings.brightness)?;
    set_setting_byte(&mut bytes, SETTING_MUSIC_VOLUME, settings.music_volume)?;
    set_setting_byte(
        &mut bytes,
        SETTING_SOUND_EFFECTS_VOLUME,
        settings.sound_effects_volume,
    )?;
    set_setting_byte(&mut bytes, SETTING_VOICE_VOLUME, settings.voice_volume)?;
    set_setting_byte(&mut bytes, SETTING_DISPLAY_BLOOD, settings.display_blood)?;
    set_setting_byte(&mut bytes, SETTING_SUBTITLES, settings.subtitles)?;
    set_setting_byte(&mut bytes, SETTING_HUD, settings.hud)?;
    set_setting_byte(&mut bytes, SETTING_CAMERA_X_AXIS, settings.camera_x_axis)?;
    set_setting_byte(&mut bytes, SETTING_CAMERA_Y_AXIS, settings.camera_y_axis)?;
    set_setting_byte(
        &mut bytes,
        SETTING_TOGGLE_AUTO_LOCKON,
        settings.toggle_auto_lockon,
    )?;
    set_setting_byte(
        &mut bytes,
        SETTING_CAMERA_AUTO_WALL_RECOVERY,
        settings.camera_auto_wall_recovery,
    )?;
    set_setting_byte(
        &mut bytes,
        SETTING_RESET_CAMERA_Y_AXIS,
        settings.reset_camera_y_axis,
    )?;
    set_setting_byte(
        &mut bytes,
        SETTING_CINEMATIC_EFFECTS,
        settings.cinematic_effects,
    )?;
    set_setting_byte(
        &mut bytes,
        SETTING_PERFORM_MATCHMAKING,
        settings.perform_matchmaking,
    )?;
    set_setting_byte(
        &mut bytes,
        SETTING_MANUAL_ATTACK_AIM,
        settings.manual_attack_aim,
    )?;
    set_setting_byte(&mut bytes, SETTING_AUTOTARGET, settings.autotarget)?;
    set_setting_byte(&mut bytes, SETTING_LAUNCHSETTINGS, settings.launchsettings)?;
    set_setting_byte(
        &mut bytes,
        SETTING_SEND_SUMMON_SIGN,
        settings.send_summon_sign,
    )?;
    set_setting_byte(&mut bytes, SETTING_HDR, settings.hdr)?;
    set_setting_byte(
        &mut bytes,
        SETTING_HDR_ADJUST_BRIGHTNESS,
        settings.hdr_adjust_brightness,
    )?;
    set_setting_byte(
        &mut bytes,
        SETTING_HDR_ADJUST_SATURATION,
        settings.hdr_adjust_saturation,
    )?;
    set_setting_byte(&mut bytes, SETTING_MASTER_VOLUME, settings.master_volume)?;
    set_setting_byte(
        &mut bytes,
        SETTING_IS_RAYTRACING_ON,
        settings.is_raytracing_on,
    )?;
    set_setting_byte(&mut bytes, SETTING_MARK_NEW_ITEMS, settings.mark_new_items)?;
    set_setting_byte(
        &mut bytes,
        SETTING_SHOW_RECENT_TABS,
        settings.show_recent_tabs,
    )?;
    set_setting_byte(&mut bytes, SETTING_SHOW_TUTORIALS, settings.show_tutorials)?;
    set_setting_byte(
        &mut bytes,
        SETTING_CAMERA_AUTO_ROTATION,
        settings.camera_auto_rotation,
    )?;

    recalculate_user_data_10_checksum(&mut bytes)?;

    // Make sure our edited block is still parseable
    // before replacing the real save.
    parse_user_data_10(&bytes)?;

    replace_save(&save_path, &bytes)?;

    println!("[Game Settings] Saved editable Elden Ring settings.");

    let final_bytes = fs::read(&save_path).map_err(|error| {
        format!(
            "Settings were saved, but the launcher could not reload them: {}",
            error
        )
    })?;

    settings_from_bytes(&save_path, &final_bytes)
}

#[tauri::command]
pub fn restore_save_game_settings_backup() -> Result<SaveGameSettings, String> {
    let save_path = find_save_file()?;

    let backup = backup_path(&save_path);

    if !backup.is_file() {
        return Err("No Elden Mod Manager save backup exists yet.".to_string());
    }

    let backup_bytes = fs::read(&backup)
        .map_err(|error| format!("Failed to read backup {}: {}", backup.display(), error))?;

    verify_save_bytes(&backup_bytes)?;

    parse_user_data_10(&backup_bytes)?;

    // Preserve the state immediately before restore,
    // so even restoring is reversible.
    if save_path.is_file() {
        let pre_restore = pre_restore_path(&save_path);

        fs::copy(&save_path, &pre_restore).map_err(|error| {
            format!(
                "Failed to create pre-restore safety copy {}: {}",
                pre_restore.display(),
                error
            )
        })?;
    }

    replace_save(&save_path, &backup_bytes)?;

    println!("[Game Settings] Restored original Elden Ring save backup.");

    let restored = fs::read(&save_path)
        .map_err(|error| format!("Backup was restored, but could not be reloaded: {}", error))?;

    settings_from_bytes(&save_path, &restored)
}
