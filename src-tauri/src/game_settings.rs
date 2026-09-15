use crate::elden_ring::elden_ring_config_dir;

use serde::{Deserialize, Serialize};

use std::fs;
use std::path::{Path, PathBuf};

const GRAPHICS_CONFIG_FILE: &str = "GraphicsConfig.xml";

const BACKUP_FILE: &str = "GraphicsConfig.xml.emm-backup";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameGraphicsSettings {
    pub config_found: bool,
    pub config_path: String,

    pub screen_mode: String,

    pub window_width: u32,
    pub window_height: u32,

    pub fullscreen_width: u32,
    pub fullscreen_height: u32,

    pub borderless_width: u32,
    pub borderless_height: u32,

    pub auto_detect: String,
    pub quality_setting: String,

    pub texture_quality: String,
    pub antialiasing: String,
    pub ssao: String,
    pub depth_of_field: String,
    pub motion_blur: String,
    pub shadow_quality: String,
    pub lighting_quality: String,
    pub effects_quality: String,
    pub reflection_quality: String,
    pub water_surface_quality: String,
    pub shade_quality: String,
    pub volumetric_effect_quality: String,
    pub raytracing_quality: String,
    pub gi_data_quality: String,
    pub grass_quality: String,

    pub backup_available: bool,
}

fn config_path() -> Result<PathBuf, String> {
    Ok(elden_ring_config_dir()?.join(GRAPHICS_CONFIG_FILE))
}

fn backup_path(config: &Path) -> PathBuf {
    config.with_file_name(BACKUP_FILE)
}

/*
 * GraphicsConfig.xml is normally UTF-16.
 *
 * We decode it ourselves so this works without
 * pulling an XML/encoding dependency into the
 * project.
 */
fn decode_xml(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() >= 2 {
        if bytes[0] == 0xff && bytes[1] == 0xfe {
            let words: Vec<u16> = bytes[2..]
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();

            return String::from_utf16(&words)
                .map_err(|error| format!("Invalid UTF-16LE GraphicsConfig.xml: {error}"));
        }

        if bytes[0] == 0xfe && bytes[1] == 0xff {
            let words: Vec<u16> = bytes[2..]
                .chunks_exact(2)
                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                .collect();

            return String::from_utf16(&words)
                .map_err(|error| format!("Invalid UTF-16BE GraphicsConfig.xml: {error}"));
        }
    }

    /*
     * Elden Ring configs can also appear as
     * UTF-16LE without a BOM.
     */
    if bytes.len() >= 4
        && bytes.len() % 2 == 0
        && bytes
            .iter()
            .skip(1)
            .step_by(2)
            .take(16)
            .filter(|byte| **byte == 0)
            .count()
            >= 4
    {
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        return String::from_utf16(&words)
            .map_err(|error| format!("Invalid UTF-16LE GraphicsConfig.xml: {error}"));
    }

    String::from_utf8(bytes.to_vec())
        .map_err(|error| format!("Could not decode GraphicsConfig.xml: {error}"))
}

fn encode_utf16_le(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() * 2 + 2);

    /*
     * Write a UTF-16LE BOM.
     */
    bytes.extend_from_slice(&[0xff, 0xfe]);

    for word in text.encode_utf16() {
        bytes.extend_from_slice(&word.to_le_bytes());
    }

    bytes
}

fn read_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");

    let close = format!("</{tag}>");

    let start = xml.find(&open)? + open.len();

    let rest = &xml[start..];

    let end = rest.find(&close)?;

    Some(rest[..end].trim().to_string())
}

fn read_u32(xml: &str, tag: &str, fallback: u32) -> u32 {
    read_tag(xml, tag)
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn text_value(xml: &str, tag: &str, fallback: &str) -> String {
    read_tag(xml, tag)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn replace_tag(xml: &mut String, tag: &str, value: &str) -> Result<(), String> {
    let open = format!("<{tag}>");

    let close = format!("</{tag}>");

    let Some(start_pos) = xml.find(&open) else {
        return Err(format!("GraphicsConfig.xml is missing <{tag}>."));
    };

    let value_start = start_pos + open.len();

    let Some(relative_end) = xml[value_start..].find(&close) else {
        return Err(format!("GraphicsConfig.xml has an invalid <{tag}> entry."));
    };

    let value_end = value_start + relative_end;

    xml.replace_range(value_start..value_end, value);

    Ok(())
}

fn validate_choice(name: &str, value: &str, allowed: &[&str]) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!("Invalid {name} value: {value}"))
    }
}

fn validate_settings(settings: &GameGraphicsSettings) -> Result<(), String> {
    if settings.window_width < 640
        || settings.window_height < 360
        || settings.fullscreen_width < 640
        || settings.fullscreen_height < 360
        || settings.borderless_width < 640
        || settings.borderless_height < 360
    {
        return Err("Resolution is too small.".to_string());
    }

    validate_choice(
        "screen mode",
        &settings.screen_mode,
        &["FULLSCREEN", "WINDOW", "BORDERLESS"],
    )?;

    validate_choice("auto detect", &settings.auto_detect, &["ON", "OFF"])?;

    validate_choice(
        "quality setting",
        &settings.quality_setting,
        &["LOW", "MEDIUM", "HIGH", "MAX", "CUSTOM"],
    )?;

    validate_choice(
        "texture quality",
        &settings.texture_quality,
        &["LOW", "MEDIUM", "HIGH", "MAX"],
    )?;

    validate_choice(
        "anti-aliasing",
        &settings.antialiasing,
        &["DISABLE", "LOW", "HIGH"],
    )?;

    validate_choice(
        "SSAO",
        &settings.ssao,
        &["DISABLE", "MEDIUM", "HIGH", "MAX"],
    )?;

    validate_choice(
        "depth of field",
        &settings.depth_of_field,
        &["DISABLE", "LOW", "MEDIUM", "HIGH", "MAX"],
    )?;

    validate_choice(
        "motion blur",
        &settings.motion_blur,
        &["DISABLE", "LOW", "MEDIUM", "HIGH"],
    )?;

    for (name, value) in [
        ("shadow quality", settings.shadow_quality.as_str()),
        ("lighting quality", settings.lighting_quality.as_str()),
        ("effects quality", settings.effects_quality.as_str()),
        (
            "volumetric quality",
            settings.volumetric_effect_quality.as_str(),
        ),
    ] {
        validate_choice(name, value, &["LOW", "MEDIUM", "HIGH", "MAX"])?;
    }

    validate_choice(
        "reflection quality",
        &settings.reflection_quality,
        &["LOW", "HIGH", "MAX"],
    )?;

    validate_choice(
        "water surface quality",
        &settings.water_surface_quality,
        &["LOW", "HIGH"],
    )?;

    validate_choice(
        "shader quality",
        &settings.shade_quality,
        &["LOW", "MEDIUM", "HIGH"],
    )?;

    validate_choice(
        "global illumination quality",
        &settings.gi_data_quality,
        &["LOW", "MEDIUM", "HIGH"],
    )?;

    validate_choice(
        "grass quality",
        &settings.grass_quality,
        &["MEDIUM", "HIGH", "MAX"],
    )?;

    validate_choice(
        "ray tracing quality",
        &settings.raytracing_quality,
        &["DISABLE", "LOW", "MEDIUM", "HIGH", "MAX"],
    )?;

    Ok(())
}

fn read_settings_from(path: &Path) -> Result<GameGraphicsSettings, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("Failed to read {}: {error}", path.display(),))?;

    let xml = decode_xml(&bytes)?;

    let backup = backup_path(path);

    Ok(GameGraphicsSettings {
        config_found: true,

        config_path: path.to_string_lossy().into_owned(),

        screen_mode: text_value(&xml, "ScreenMode", "BORDERLESS"),

        window_width: read_u32(&xml, "Resolution-WindowScreenWidth", 1920),

        window_height: read_u32(&xml, "Resolution-WindowScreenHeight", 1080),

        fullscreen_width: read_u32(&xml, "Resolution-FullScreenWidth", 1920),

        fullscreen_height: read_u32(&xml, "Resolution-FullScreenHeight", 1080),

        borderless_width: read_u32(&xml, "Resolution-BorderlessScreenWidth", 1920),

        borderless_height: read_u32(&xml, "Resolution-BorderlessScreenHeight", 1080),

        auto_detect: text_value(&xml, "Auto-detectBestRenderingSettings", "OFF"),

        quality_setting: text_value(&xml, "QualitySetting", "CUSTOM"),

        texture_quality: text_value(&xml, "TextureQuality", "HIGH"),

        antialiasing: text_value(&xml, "Antialiasing", "HIGH"),

        ssao: text_value(&xml, "SSAO", "HIGH"),

        depth_of_field: text_value(&xml, "DepthOfField", "HIGH"),

        motion_blur: text_value(&xml, "MotionBlur", "HIGH"),

        shadow_quality: text_value(&xml, "ShadowQuality", "HIGH"),

        lighting_quality: text_value(&xml, "LightingQuality", "HIGH"),

        effects_quality: text_value(&xml, "EffectsQuality", "HIGH"),

        reflection_quality: text_value(&xml, "ReflectionQuality", "HIGH"),

        water_surface_quality: text_value(&xml, "WaterSurfaceQuality", "HIGH"),

        shade_quality: text_value(&xml, "ShadeQuality", "HIGH"),

        volumetric_effect_quality: text_value(&xml, "VolumetricEffectQuality", "HIGH"),

        raytracing_quality: text_value(&xml, "RaytracingQuality", "DISABLE"),

        gi_data_quality: text_value(&xml, "GIDataQuality", "HIGH"),

        grass_quality: text_value(&xml, "GrassQuality", "HIGH"),

        backup_available: backup.exists(),
    })
}

#[tauri::command]
pub fn get_game_graphics_settings() -> Result<GameGraphicsSettings, String> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(GameGraphicsSettings {
            config_found: false,

            config_path: path.to_string_lossy().into_owned(),

            screen_mode: "BORDERLESS".to_string(),

            window_width: 1920,
            window_height: 1080,

            fullscreen_width: 1920,
            fullscreen_height: 1080,

            borderless_width: 1920,
            borderless_height: 1080,

            auto_detect: "OFF".to_string(),

            quality_setting: "CUSTOM".to_string(),

            texture_quality: "HIGH".to_string(),

            antialiasing: "HIGH".to_string(),

            ssao: "HIGH".to_string(),

            depth_of_field: "HIGH".to_string(),

            motion_blur: "HIGH".to_string(),

            shadow_quality: "HIGH".to_string(),

            lighting_quality: "HIGH".to_string(),

            effects_quality: "HIGH".to_string(),

            reflection_quality: "HIGH".to_string(),

            water_surface_quality: "HIGH".to_string(),

            shade_quality: "HIGH".to_string(),

            volumetric_effect_quality: "HIGH".to_string(),

            raytracing_quality: "DISABLE".to_string(),

            gi_data_quality: "HIGH".to_string(),

            grass_quality: "HIGH".to_string(),

            backup_available: false,
        });
    }

    read_settings_from(&path)
}

#[tauri::command]
pub fn save_game_graphics_settings(
    settings: GameGraphicsSettings,
) -> Result<GameGraphicsSettings, String> {
    validate_settings(&settings)?;

    let path = config_path()?;

    if !path.exists() {
        return Err(
            format!(
                "GraphicsConfig.xml was not found at {}. Launch Elden Ring once so the game can create it.",
                path.display(),
            )
        );
    }

    let original =
        fs::read(&path).map_err(|error| format!("Failed to read GraphicsConfig.xml: {error}"))?;

    let backup = backup_path(&path);

    /*
     * Preserve the last known-good config before
     * every launcher save.
     */
    fs::write(&backup, &original)
        .map_err(|error| format!("Failed to create GraphicsConfig backup: {error}"))?;

    let mut xml = decode_xml(&original)?;

    replace_tag(&mut xml, "ScreenMode", &settings.screen_mode)?;

    replace_tag(
        &mut xml,
        "Resolution-WindowScreenWidth",
        &settings.window_width.to_string(),
    )?;

    replace_tag(
        &mut xml,
        "Resolution-WindowScreenHeight",
        &settings.window_height.to_string(),
    )?;

    replace_tag(
        &mut xml,
        "Resolution-FullScreenWidth",
        &settings.fullscreen_width.to_string(),
    )?;

    replace_tag(
        &mut xml,
        "Resolution-FullScreenHeight",
        &settings.fullscreen_height.to_string(),
    )?;

    replace_tag(
        &mut xml,
        "Resolution-BorderlessScreenWidth",
        &settings.borderless_width.to_string(),
    )?;

    replace_tag(
        &mut xml,
        "Resolution-BorderlessScreenHeight",
        &settings.borderless_height.to_string(),
    )?;

    replace_tag(
        &mut xml,
        "Auto-detectBestRenderingSettings",
        &settings.auto_detect,
    )?;

    replace_tag(&mut xml, "QualitySetting", &settings.quality_setting)?;

    replace_tag(&mut xml, "TextureQuality", &settings.texture_quality)?;

    replace_tag(&mut xml, "Antialiasing", &settings.antialiasing)?;

    replace_tag(&mut xml, "SSAO", &settings.ssao)?;

    replace_tag(&mut xml, "DepthOfField", &settings.depth_of_field)?;

    replace_tag(&mut xml, "MotionBlur", &settings.motion_blur)?;

    replace_tag(&mut xml, "ShadowQuality", &settings.shadow_quality)?;

    replace_tag(&mut xml, "LightingQuality", &settings.lighting_quality)?;

    replace_tag(&mut xml, "EffectsQuality", &settings.effects_quality)?;

    replace_tag(&mut xml, "ReflectionQuality", &settings.reflection_quality)?;

    replace_tag(
        &mut xml,
        "WaterSurfaceQuality",
        &settings.water_surface_quality,
    )?;

    replace_tag(&mut xml, "ShadeQuality", &settings.shade_quality)?;

    replace_tag(
        &mut xml,
        "VolumetricEffectQuality",
        &settings.volumetric_effect_quality,
    )?;

    replace_tag(&mut xml, "RaytracingQuality", &settings.raytracing_quality)?;

    replace_tag(&mut xml, "GIDataQuality", &settings.gi_data_quality)?;

    replace_tag(&mut xml, "GrassQuality", &settings.grass_quality)?;

    let encoded = encode_utf16_le(&xml);

    fs::write(&path, encoded)
        .map_err(|error| format!("Failed to save GraphicsConfig.xml: {error}"))?;

    read_settings_from(&path)
}

#[tauri::command]
pub fn restore_game_graphics_backup() -> Result<GameGraphicsSettings, String> {
    let path = config_path()?;

    let backup = backup_path(&path);

    if !backup.exists() {
        return Err("No launcher graphics backup exists yet.".to_string());
    }

    fs::copy(&backup, &path)
        .map_err(|error| format!("Failed to restore graphics backup: {error}"))?;

    read_settings_from(&path)
}
