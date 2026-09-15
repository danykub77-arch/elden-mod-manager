use crate::paths;
use crate::runtime::{InstalledMod, ModContentType, UnifiedProfile};

use serde::Serialize;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    Asset,
    Native,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModConflict {
    pub conflict_type: ConflictType,

    /*
     * Keep the exact conflicting path available in the backend.
     *
     * The normal UI does NOT need to display this.
     * Later we can expose it behind a Debugging setting.
     */
    pub relative_path: String,

    /*
     * These are stored in effective profile load order:
     *
     * first  = loaded earlier
     * last   = loaded later
     *
     * For loose asset conflicts, the last mod wins.
     */
    pub mod_ids: Vec<String>,
    pub mod_names: Vec<String>,

    /*
     * Only asset conflicts receive a winner.
     *
     * Native DLL conflicts intentionally do not because DLL conflicts
     * cannot safely be considered resolved purely by package order.
     */
    pub winner_mod_id: Option<String>,
    pub winner_mod_name: Option<String>,

    /*
     * For asset conflicts, these are every conflicting mod that appears
     * before the winner and is therefore overridden for this file.
     *
     * Native conflicts leave these empty.
     */
    pub overridden_mod_ids: Vec<String>,
    pub overridden_mod_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictReport {
    pub profile_id: String,
    pub conflict_count: usize,
    pub conflicts: Vec<ModConflict>,
}

#[derive(Debug, Clone)]
struct FileOwner {
    mod_id: String,
    mod_name: String,
}

fn profile_directory(profile_id: &str) -> Result<PathBuf, String> {
    Ok(paths::profiles_dir()?.join(profile_id))
}

fn profile_json_path(profile_id: &str) -> Result<PathBuf, String> {
    Ok(profile_directory(profile_id)?.join("profile.json"))
}

fn load_profile(profile_id: &str) -> Result<UnifiedProfile, String> {
    let path = profile_json_path(profile_id)?;

    if !path.is_file() {
        return Err(format!("Profile '{}' does not exist.", profile_id,));
    }

    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display(),))?;

    serde_json::from_str(&text)
        .map_err(|error| format!("Failed to parse {}: {error}", path.display(),))
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
        .to_ascii_lowercase()
}

fn collect_files_recursive(
    root: &Path,
    current: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(current)
        .map_err(|error| format!("Failed to inspect {}: {error}", current.display(),))?
    {
        let entry = entry.map_err(|error| error.to_string())?;

        let path = entry.path();

        if path.is_dir() {
            collect_files_recursive(root, &path, files)?;
        } else if path.is_file() {
            let relative = path.strip_prefix(root).map_err(|_| {
                format!(
                    "Installed mod file escaped its content root: {}",
                    path.display(),
                )
            })?;

            files.push(normalize_relative_path(relative));
        }
    }

    Ok(())
}

fn collect_asset_files(
    profile_directory: &Path,
    installed_mod: &InstalledMod,
) -> Result<Vec<String>, String> {
    let mut files = Vec::new();

    for content in &installed_mod.contents {
        if !matches!(content.content_type, ModContentType::Assets) {
            continue;
        }

        let content_root = profile_directory.join(&content.relative_path);

        if !content_root.is_dir() {
            continue;
        }

        collect_files_recursive(&content_root, &content_root, &mut files)?;
    }

    files.sort();
    files.dedup();

    Ok(files)
}

fn collect_native_files(profile_directory: &Path, installed_mod: &InstalledMod) -> Vec<String> {
    let mut files = Vec::new();

    for content in &installed_mod.contents {
        if !matches!(content.content_type, ModContentType::NativeDll) {
            continue;
        }

        let path = Path::new(&content.relative_path);

        /*
         * Native modules do not share the same virtual filesystem
         * namespace that loose asset packages do.
         *
         * For now we conservatively compare the DLL filename itself.
         */
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        /*
         * Ignore stale profile entries when the DLL no longer exists.
         */
        let absolute = profile_directory.join(&content.relative_path);

        if !absolute.is_file() {
            continue;
        }

        files.push(file_name.to_ascii_lowercase());
    }

    files.sort();
    files.dedup();

    files
}

fn build_conflicts(
    owners: HashMap<String, Vec<FileOwner>>,
    conflict_type: ConflictType,
) -> Vec<ModConflict> {
    let mut conflicts = Vec::new();

    for (relative_path, file_owners) in owners {
        if file_owners.len() < 2 {
            continue;
        }

        /*
         * file_owners arrives in profile.mods order because owners are
         * inserted while walking the profile from top to bottom.
         *
         * Preserve that order while removing any duplicate ownership
         * entries for the same mod.
         */
        let mut mod_ids = Vec::new();

        let mut mod_names = Vec::new();

        for owner in file_owners {
            if mod_ids.contains(&owner.mod_id) {
                continue;
            }

            mod_ids.push(owner.mod_id);

            mod_names.push(owner.mod_name);
        }

        if mod_ids.len() < 2 {
            continue;
        }

        let (winner_mod_id, winner_mod_name, overridden_mod_ids, overridden_mod_names) =
            match &conflict_type {
                ConflictType::Asset => {
                    /*
                     * Verified behavior:
                     *
                     * bottom of the profile list loads later.
                     * Later loose assets override earlier loose assets.
                     *
                     * Therefore the final owner is the winner.
                     */
                    let winner_index = mod_ids.len() - 1;

                    (
                        Some(mod_ids[winner_index].clone()),
                        Some(mod_names[winner_index].clone()),
                        mod_ids[..winner_index].to_vec(),
                        mod_names[..winner_index].to_vec(),
                    )
                }

                ConflictType::Native => {
                    /*
                     * Do not claim native DLL conflicts are resolved by
                     * load order.
                     */
                    (None, None, Vec::new(), Vec::new())
                }
            };

        conflicts.push(ModConflict {
            conflict_type: conflict_type.clone(),

            relative_path,

            mod_ids,
            mod_names,

            winner_mod_id,
            winner_mod_name,

            overridden_mod_ids,
            overridden_mod_names,
        });
    }

    conflicts
}

pub fn analyze_profile_conflicts(profile_id: &str) -> Result<ConflictReport, String> {
    let profile = load_profile(profile_id)?;

    let profile_directory = profile_directory(profile_id)?;

    let mut asset_owners: HashMap<String, Vec<FileOwner>> = HashMap::new();

    let mut native_owners: HashMap<String, Vec<FileOwner>> = HashMap::new();

    /*
     * IMPORTANT:
     *
     * Do not sort this.
     *
     * profile.mods order is our actual load order.
     * Bottom entries are loaded later and therefore win asset conflicts.
     */
    for installed_mod in profile.mods.iter().filter(|item| item.enabled) {
        let asset_files = collect_asset_files(&profile_directory, installed_mod)?;

        for relative_path in asset_files {
            asset_owners
                .entry(relative_path)
                .or_default()
                .push(FileOwner {
                    mod_id: installed_mod.id.clone(),

                    mod_name: installed_mod.name.clone(),
                });
        }

        let native_files = collect_native_files(&profile_directory, installed_mod);

        for relative_path in native_files {
            native_owners
                .entry(relative_path)
                .or_default()
                .push(FileOwner {
                    mod_id: installed_mod.id.clone(),

                    mod_name: installed_mod.name.clone(),
                });
        }
    }

    let mut conflicts = build_conflicts(asset_owners, ConflictType::Asset);

    conflicts.extend(build_conflicts(native_owners, ConflictType::Native));

    conflicts.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    Ok(ConflictReport {
        profile_id: profile_id.to_string(),

        conflict_count: conflicts.len(),

        conflicts,
    })
}

#[tauri::command]
pub fn get_profile_conflicts(profile_id: String) -> Result<ConflictReport, String> {
    analyze_profile_conflicts(&profile_id)
}
