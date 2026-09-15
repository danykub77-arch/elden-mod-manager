use crate::engines::EngineKind;
use crate::mod_import::{
    analyze_mod_archive, import_mod_variant, record_nexus_mod_install, set_mod_enabled,
};
use crate::nexus_download::download_nexus_mod_file;
use crate::paths;
use crate::profiles;
use crate::runtime::{ModSource, UnifiedProfile};

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

const MODPACK_FORMAT_VERSION: u32 = 1;
const MODPACK_GAME: &str = "elden-ring";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ModpackSource {
    Nexus {
        mod_id: u64,
        file_id: u64,
        version: Option<String>,
    },
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackEntry {
    pub name: String,
    pub version: Option<String>,
    pub enabled: bool,
    pub source: ModpackSource,
    pub variant_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackManifest {
    pub format_version: u32,
    pub game: String,
    pub name: String,
    pub engine: EngineKind,
    pub mods: Vec<ModpackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NexusSourceMetadata {
    provider: String,
    mod_id: u64,
    file_id: u64,
    version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModpackImportFailure {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModpackImportResult {
    pub profile_id: String,
    pub profile_name: String,
    pub installed_count: usize,
    pub skipped_manual: Vec<String>,
    pub failures: Vec<ModpackImportFailure>,
}

fn profile_path(profile_id: &str) -> Result<PathBuf, String> {
    Ok(paths::profiles_dir()?.join(profile_id).join("profile.json"))
}

fn load_profile(profile_id: &str) -> Result<UnifiedProfile, String> {
    let path = profile_path(profile_id)?;
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;

    serde_json::from_str(&text)
        .map_err(|error| format!("Failed to parse {}: {error}", path.display()))
}

fn nexus_metadata_path(profile_id: &str, mod_id: &str) -> Result<PathBuf, String> {
    if mod_id == "elden-ring-randomizer" {
        return Ok(paths::profiles_dir()?
            .join(profile_id)
            .join("tools")
            .join("randomizer")
            .join(".nexus-source.json"));
    }

    Ok(paths::profiles_dir()?
        .join(profile_id)
        .join("mods")
        .join(mod_id)
        .join(".nexus-source.json"))
}

fn legacy_nexus_source(text: &str) -> Option<NexusSourceMetadata> {
    let mut parts = text.splitn(3, '-');
    let mod_id = parts.next()?.parse::<u64>().ok()?;
    let file_id = parts.next()?.parse::<u64>().ok()?;
    parts.next()?;

    Some(NexusSourceMetadata {
        provider: "nexus".to_string(),
        mod_id,
        file_id,
        version: None,
    })
}

fn read_nexus_source(profile_id: &str, mod_id: &str) -> Option<NexusSourceMetadata> {
    if let Ok(path) = nexus_metadata_path(profile_id, mod_id) {
        if let Ok(text) = fs::read_to_string(path) {
            if let Ok(metadata) = serde_json::from_str::<NexusSourceMetadata>(&text) {
                if metadata.provider.eq_ignore_ascii_case("nexus") {
                    return Some(metadata);
                }
            }
        }
    }

    legacy_nexus_source(mod_id)
}

fn read_nexus_source_for_mod(
    profile_id: &str,
    mod_id: &str,
    mod_name: &str,
) -> Option<NexusSourceMetadata> {
    read_nexus_source(profile_id, mod_id).or_else(|| legacy_nexus_source(mod_name))
}

fn variant_hint(name: &str) -> Option<String> {
    let (_, suffix) = name.rsplit_once(" — ")?;
    let suffix = suffix.trim();

    if suffix.is_empty() {
        None
    } else {
        Some(suffix.to_string())
    }
}

fn sanitize_file_name(name: &str) -> String {
    let mut output = String::new();

    for character in name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_') {
            output.push(character);
        }
    }

    let trimmed = output.trim();
    if trimmed.is_empty() {
        "elden-ring-modpack".to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_manifest(manifest: &ModpackManifest) -> Result<(), String> {
    if manifest.format_version != MODPACK_FORMAT_VERSION {
        return Err(format!(
            "Unsupported modpack format version {}. This build supports version {}.",
            manifest.format_version, MODPACK_FORMAT_VERSION
        ));
    }

    if manifest.game != MODPACK_GAME {
        return Err(format!(
            "This modpack targets '{}', not Elden Ring.",
            manifest.game
        ));
    }

    if manifest.name.trim().is_empty() {
        return Err("The modpack has no name.".to_string());
    }

    Ok(())
}

fn read_manifest(path: &Path) -> Result<ModpackManifest, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;

    let manifest = serde_json::from_str::<ModpackManifest>(&text)
        .map_err(|error| format!("Invalid .ermodpack file: {error}"))?;

    validate_manifest(&manifest)?;
    Ok(manifest)
}

#[tauri::command]
pub async fn export_modpack(app: AppHandle, profile_id: String) -> Result<Option<String>, String> {
    let profile = load_profile(&profile_id)?;
    let config = profiles::get_profiles()?;
    let profile_settings = config
        .profiles
        .iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| format!("Profile '{profile_id}' does not exist."))?;

    let mods = profile
        .mods
        .iter()
        .map(|installed_mod| {
            let source = match &installed_mod.source {
                Some(ModSource::Nexus {
                    mod_id,
                    file_id,
                    version,
                }) => ModpackSource::Nexus {
                    mod_id: *mod_id,
                    file_id: *file_id,
                    version: version.clone(),
                },

                None => {
                    read_nexus_source_for_mod(&profile_id, &installed_mod.id, &installed_mod.name)
                        .map(|metadata| ModpackSource::Nexus {
                            mod_id: metadata.mod_id,
                            file_id: metadata.file_id,
                            version: metadata.version,
                        })
                        .unwrap_or(ModpackSource::Manual)
                }
            };

            ModpackEntry {
                name: installed_mod.name.clone(),
                version: installed_mod.version.clone(),
                enabled: installed_mod.enabled,
                source,
                variant_hint: variant_hint(&installed_mod.name),
            }
        })
        .collect();

    let manifest = ModpackManifest {
        format_version: MODPACK_FORMAT_VERSION,
        game: MODPACK_GAME.to_string(),
        name: profile_settings.name.clone(),
        engine: profile_settings.engine.clone(),
        mods,
    };

    let suggested_name = format!("{}.ermodpack", sanitize_file_name(&manifest.name));
    let dialog_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .add_filter("Elden Mod Manager Pack", &["ermodpack"])
            .set_file_name(&suggested_name)
            .blocking_save_file()
    })
    .await
    .map_err(|error| format!("Save dialog task failed: {error}"))?;

    let Some(selected) = selected else {
        return Ok(None);
    };

    let Some(path) = selected.as_path() else {
        return Err(
            "The selected save location did not resolve to a normal file path.".to_string(),
        );
    };

    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("Failed to serialize modpack: {error}"))?;

    fs::write(path, json)
        .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;

    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn inspect_modpack(app: AppHandle) -> Result<Option<ModpackManifest>, String> {
    let dialog_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .add_filter("Elden Mod Manager Pack", &["ermodpack"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| format!("Open dialog task failed: {error}"))?;

    let Some(selected) = selected else {
        return Ok(None);
    };

    let Some(path) = selected.as_path() else {
        return Err("The selected modpack did not resolve to a normal file path.".to_string());
    };

    Ok(Some(read_manifest(path)?))
}

fn choose_variant<'a>(
    entry: &ModpackEntry,
    analysis: &'a crate::mod_import::ModArchiveAnalysis,
) -> Result<&'a crate::mod_import::ModArchiveVariant, String> {
    if analysis.variants.len() == 1 {
        return Ok(&analysis.variants[0]);
    }

    if let Some(hint) = entry.variant_hint.as_deref() {
        if let Some(variant) = analysis
            .variants
            .iter()
            .find(|variant| variant.name.eq_ignore_ascii_case(hint))
        {
            return Ok(variant);
        }
    }

    Err(format!(
        "{} has {} install variants and this modpack does not contain enough information to select one automatically.",
        entry.name,
        analysis.variants.len()
    ))
}

#[tauri::command]
pub async fn import_modpack(app: AppHandle) -> Result<Option<ModpackImportResult>, String> {
    let dialog_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .add_filter("Elden Mod Manager Pack", &["ermodpack"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| format!("Open dialog task failed: {error}"))?;

    let Some(selected) = selected else {
        return Ok(None);
    };

    let Some(path) = selected.as_path() else {
        return Err("The selected modpack did not resolve to a normal file path.".to_string());
    };

    let manifest = read_manifest(path)?;
    let config = profiles::create_profile(manifest.name.clone())?;
    let profile_id = config.selected_profile.clone();

    profiles::set_profile_engine(profile_id.clone(), manifest.engine.clone())?;

    let mut installed_count = 0usize;
    let mut skipped_manual = Vec::new();
    let mut failures = Vec::new();

    for entry in &manifest.mods {
        let ModpackSource::Nexus {
            mod_id,
            file_id,
            version,
        } = &entry.source
        else {
            skipped_manual.push(entry.name.clone());
            continue;
        };

        let install_result = async {
            let archive_path = download_nexus_mod_file(app.clone(), *mod_id, *file_id).await?;
            let analysis = analyze_mod_archive(archive_path.clone())?;
            let variant = choose_variant(entry, &analysis)?.clone();

            let installed = import_mod_variant(
                profile_id.clone(),
                archive_path,
                variant.relative_path,
                variant.name,
                analysis.variants.len() > 1,
            )?;

            record_nexus_mod_install(
                profile_id.clone(),
                installed.id.clone(),
                *mod_id,
                *file_id,
                version.clone(),
            )?;

            if !entry.enabled {
                set_mod_enabled(profile_id.clone(), installed.id, false)?;
            }

            Ok::<(), String>(())
        }
        .await;

        match install_result {
            Ok(()) => installed_count += 1,
            Err(reason) => failures.push(ModpackImportFailure {
                name: entry.name.clone(),
                reason,
            }),
        }
    }

    Ok(Some(ModpackImportResult {
        profile_id,
        profile_name: manifest.name,
        installed_count,
        skipped_manual,
        failures,
    }))
}
