use crate::paths;
use crate::runtime::{
    CompatibilityLevel, InstalledMod, ModContent, ModContentType, ModSource, UnifiedProfile,
};

use serde::Serialize;

use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};

use tauri_plugin_dialog::DialogExt;

const MAX_UNCOMPRESSED_SIZE: u64 = 8 * 1024 * 1024 * 1024;

const ASSET_DIRECTORIES: &[&str] = &[
    "action", "asset", "chr", "event", "map", "menu", "msg", "parts", "script", "sfx", "shader",
    "material", "movie", "param", "sound",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveFormat {
    Zip,
    Rar,
    SevenZip,
}

impl ArchiveFormat {
    fn display_name(self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "ZIP",
            ArchiveFormat::Rar => "RAR",
            ArchiveFormat::SevenZip => "7Z",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateKind {
    Assets,
    Native,
    Mixed,
    Randomizer,
}

impl CandidateKind {
    fn display_name(self) -> &'static str {
        match self {
            CandidateKind::Assets => "Loose assets",
            CandidateKind::Native => "Native DLL",
            CandidateKind::Mixed => "Assets + native DLL",
            CandidateKind::Randomizer => "Elden Ring Randomizer",
        }
    }
}

#[derive(Debug, Clone)]
struct InstallCandidate {
    root: PathBuf,
    kind: CandidateKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModArchiveVariant {
    pub id: String,
    pub name: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModArchiveAnalysis {
    pub archive_path: String,
    pub display_name: String,
    pub variants: Vec<ModArchiveVariant>,
}

fn profile_directory(profile_id: &str) -> Result<PathBuf, String> {
    Ok(paths::profiles_dir()?.join(profile_id))
}

fn profile_json_path(profile_id: &str) -> Result<PathBuf, String> {
    Ok(profile_directory(profile_id)?.join("profile.json"))
}

fn profile_mods_directory(profile_id: &str) -> Result<PathBuf, String> {
    Ok(profile_directory(profile_id)?.join("mods"))
}

fn profile_tools_directory(profile_id: &str) -> Result<PathBuf, String> {
    Ok(profile_directory(profile_id)?.join("tools"))
}

fn staging_directory() -> Result<PathBuf, String> {
    Ok(paths::runtime_dir()?.join("mod-import-staging"))
}

fn load_profile(profile_id: &str) -> Result<UnifiedProfile, String> {
    let path = profile_json_path(profile_id)?;

    if !path.is_file() {
        return Err(format!("Profile '{}' does not exist.", profile_id));
    }

    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;

    serde_json::from_str(&text)
        .map_err(|error| format!("Failed to parse {}: {error}", path.display()))
}

fn save_profile(profile: &UnifiedProfile) -> Result<(), String> {
    let path = profile_json_path(&profile.profile_id)?;

    let text = serde_json::to_string_pretty(profile)
        .map_err(|error| format!("Failed to serialize profile: {error}"))?;

    fs::write(&path, text).map_err(|error| format!("Failed to save {}: {error}", path.display()))
}

fn clear_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("Failed to remove {}: {error}", path.display()))?;
    }

    fs::create_dir_all(path)
        .map_err(|error| format!("Failed to create {}: {error}", path.display()))
}

fn archive_format(archive_path: &Path) -> Result<ArchiveFormat, String> {
    let extension = archive_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "zip" => Ok(ArchiveFormat::Zip),
        "rar" => Ok(ArchiveFormat::Rar),
        "7z" => Ok(ArchiveFormat::SevenZip),
        _ => {
            Err("Unsupported archive format. Choose a .zip, .rar, or .7z mod archive.".to_string())
        }
    }
}

fn validate_archive_path(archive_path: &Path) -> Result<ArchiveFormat, String> {
    if !archive_path.is_file() {
        return Err("The selected mod archive no longer exists.".to_string());
    }

    archive_format(archive_path)
}

fn safe_relative_path(raw_path: &str) -> Result<PathBuf, String> {
    let normalized = raw_path.replace('\\', "/");

    let mut result = PathBuf::new();

    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Archive contains an unsafe path: {raw_path}"));
            }
        }
    }

    if result.as_os_str().is_empty() {
        return Err("Archive contains an empty file path.".to_string());
    }

    Ok(result)
}

fn extract_zip(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|error| format!("Failed to open {}: {error}", archive_path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("Failed to read ZIP archive: {error}"))?;

    let mut total_size = 0u64;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Failed to read ZIP entry: {error}"))?;

        total_size = total_size.saturating_add(entry.size());

        if total_size > MAX_UNCOMPRESSED_SIZE {
            return Err("This archive expands to more than 8 GB. Import cancelled.".to_string());
        }

        let relative_path = entry
            .enclosed_name()
            .map(|path| path.to_path_buf())
            .ok_or_else(|| "The ZIP contains an unsafe file path.".to_string())?;

        let output_path = destination.join(relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|error| format!("Failed to create {}: {error}", output_path.display()))?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
        }

        let mut output = File::create(&output_path)
            .map_err(|error| format!("Failed to create {}: {error}", output_path.display()))?;

        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("Failed to extract {}: {error}", output_path.display()))?;
    }

    Ok(())
}

fn extract_rar(archive_path: &Path, destination: &Path) -> Result<(), String> {
    use unrar_rs::RarArchive;

    let file = File::open(archive_path)
        .map_err(|error| format!("Failed to open RAR {}: {error}", archive_path.display()))?;

    let mut archive =
        RarArchive::open(file).map_err(|error| format!("Failed to read RAR archive: {error}"))?;

    let mut total_size = 0u64;
    let entry_count = archive.len();

    for index in 0..entry_count {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("Failed to read RAR entry #{index}: {error}"))?;

        let entry_name = entry.name().to_string();
        let is_directory = entry.is_dir();
        let entry_size = entry.size().unwrap_or(0);

        total_size = total_size.saturating_add(entry_size);

        if total_size > MAX_UNCOMPRESSED_SIZE {
            return Err("This archive expands to more than 8 GB. Import cancelled.".to_string());
        }

        let relative_path = safe_relative_path(&entry_name)?;
        let output_path = destination.join(&relative_path);

        if is_directory {
            fs::create_dir_all(&output_path)
                .map_err(|error| format!("Failed to create {}: {error}", output_path.display()))?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
        }

        let entry = archive
            .by_index(index)
            .map_err(|error| format!("Failed to prepare RAR entry '{entry_name}': {error}"))?;

        entry
            .unpack_to(&output_path)
            .map_err(|error| format!("Failed to extract RAR entry '{}': {error}", entry_name))?;
    }

    Ok(())
}

fn directory_size(path: &Path) -> Result<u64, String> {
    let mut total = 0u64;

    for entry in fs::read_dir(path)
        .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            total = total.saturating_add(directory_size(&entry_path)?);
        } else {
            total =
                total.saturating_add(entry.metadata().map_err(|error| error.to_string())?.len());
        }

        if total > MAX_UNCOMPRESSED_SIZE {
            return Ok(total);
        }
    }

    Ok(total)
}

fn extract_7z(archive_path: &Path, destination: &Path) -> Result<(), String> {
    sevenz_rust::decompress_file(archive_path, destination)
        .map_err(|error| format!("Failed to extract 7Z archive: {error}"))?;

    let extracted_size = directory_size(destination)?;

    if extracted_size > MAX_UNCOMPRESSED_SIZE {
        return Err("This archive expands to more than 8 GB. Import cancelled.".to_string());
    }

    Ok(())
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<ArchiveFormat, String> {
    let format = validate_archive_path(archive_path)?;

    match format {
        ArchiveFormat::Zip => extract_zip(archive_path, destination)?,
        ArchiveFormat::Rar => extract_rar(archive_path, destination)?,
        ArchiveFormat::SevenZip => extract_7z(archive_path, destination)?,
    }

    Ok(format)
}

fn has_asset_content(directory: &Path) -> bool {
    if directory.join("regulation.bin").is_file() {
        return true;
    }

    ASSET_DIRECTORIES
        .iter()
        .any(|name| directory.join(name).is_dir())
}

fn is_dll(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("dll"))
        .unwrap_or(false)
}

fn direct_dlls(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut dlls = Vec::new();

    for entry in fs::read_dir(directory)
        .map_err(|error| format!("Failed to inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();

        if path.is_file() && is_dll(&path) {
            dlls.push(path);
        }
    }

    dlls.sort();

    Ok(dlls)
}

fn has_native_content(directory: &Path) -> Result<bool, String> {
    Ok(!direct_dlls(directory)?.is_empty())
}

fn has_randomizer_launcher(directory: &Path) -> bool {
    directory.join("EldenRingRandomizer.exe").is_file()
}

fn candidate_kind(directory: &Path) -> Result<Option<CandidateKind>, String> {
    if has_randomizer_launcher(directory) {
        return Ok(Some(CandidateKind::Randomizer));
    }

    let assets = has_asset_content(directory);
    let natives = has_native_content(directory)?;

    match (assets, natives) {
        (true, true) => Ok(Some(CandidateKind::Mixed)),
        (true, false) => Ok(Some(CandidateKind::Assets)),
        (false, true) => Ok(Some(CandidateKind::Native)),
        (false, false) => Ok(None),
    }
}

fn find_install_candidates_recursive(
    directory: &Path,
    candidates: &mut Vec<InstallCandidate>,
) -> Result<(), String> {
    if let Some(kind) = candidate_kind(directory)? {
        candidates.push(InstallCandidate {
            root: directory.to_path_buf(),
            kind,
        });

        /*
         * Once a directory itself is an installable root, everything below
         * it belongs to that candidate. This prevents chr/, parts/, etc.
         * from becoming fake variants.
         */
        return Ok(());
    }

    for entry in fs::read_dir(directory)
        .map_err(|error| format!("Failed to inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            find_install_candidates_recursive(&path, candidates)?;
        }
    }

    Ok(())
}

fn find_install_candidates(staging: &Path) -> Result<Vec<InstallCandidate>, String> {
    let mut candidates = Vec::new();

    find_install_candidates_recursive(staging, &mut candidates)?;

    candidates.sort_by(|left, right| left.root.cmp(&right.root));
    candidates.dedup_by(|left, right| left.root == right.root);

    Ok(candidates)
}

fn path_to_forward_slashes(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn relative_path_from_staging(staging: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(staging)
        .map_err(|_| "Detected mod path escaped the staging directory.".to_string())?;

    Ok(path_to_forward_slashes(relative))
}

fn variant_name_for_root(staging: &Path, root: &Path) -> String {
    if root == staging {
        return "Default".to_string();
    }

    let file_name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Default");

    if file_name.eq_ignore_ascii_case("mod") {
        if let Some(parent) = root.parent() {
            if parent != staging {
                if let Some(parent_name) = parent.file_name().and_then(|value| value.to_str()) {
                    return parent_name.to_string();
                }
            }
        }
    }

    file_name.to_string()
}

fn build_variants(
    staging: &Path,
    candidates: &[InstallCandidate],
) -> Result<Vec<ModArchiveVariant>, String> {
    let mut variants = Vec::new();
    let mut used_names = HashSet::new();

    for (index, candidate) in candidates.iter().enumerate() {
        let relative_path = relative_path_from_staging(staging, &candidate.root)?;

        let base_name = variant_name_for_root(staging, &candidate.root);

        let mut name = if candidates.len() > 1 {
            format!("{} ({})", base_name, candidate.kind.display_name())
        } else {
            base_name
        };

        if used_names.contains(&name.to_lowercase()) {
            name = if relative_path.is_empty() {
                format!("Default ({})", candidate.kind.display_name())
            } else {
                format!("{} ({})", relative_path, candidate.kind.display_name())
            };
        }

        used_names.insert(name.to_lowercase());

        variants.push(ModArchiveVariant {
            id: format!("variant-{}", index + 1),
            name,
            relative_path,
        });
    }

    Ok(variants)
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("Failed to create {}: {error}", destination.display()))?;

    for entry in fs::read_dir(source)
        .map_err(|error| format!("Failed to read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)
                .map_err(|error| format!("Failed to copy {}: {error}", source_path.display()))?;
        }
    }

    Ok(())
}

fn copy_supported_assets(source: &Path, destination: &Path) -> Result<usize, String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("Failed to create {}: {error}", destination.display()))?;

    let mut copied = 0usize;

    let regulation = source.join("regulation.bin");

    if regulation.is_file() {
        fs::copy(&regulation, destination.join("regulation.bin"))
            .map_err(|error| format!("Failed to copy regulation.bin: {error}"))?;

        copied += 1;
    }

    for directory_name in ASSET_DIRECTORIES {
        let source_directory = source.join(directory_name);

        if !source_directory.is_dir() {
            continue;
        }

        copy_directory_recursive(&source_directory, &destination.join(directory_name))?;

        copied += 1;
    }

    Ok(copied)
}

fn copy_native_bundle(source: &Path, destination: &Path) -> Result<Vec<String>, String> {
    /*
     * Preserve the native candidate's complete sibling layout.
     *
     * Example:
     *
     * ermerchant.dll
     * ermerchant.ini
     * ermerchant.me3
     * LICENSE.txt
     *
     * becomes:
     *
     * native/ermerchant.dll
     * native/ermerchant.ini
     * native/ermerchant.me3
     * native/LICENSE.txt
     *
     * Only DLLs become [[natives]] entries. The other files are preserved
     * because the native module may expect configuration/data beside it.
     */

    fs::create_dir_all(destination)
        .map_err(|error| format!("Failed to create {}: {error}", destination.display()))?;

    let dlls = direct_dlls(source)?;

    if dlls.is_empty() {
        return Ok(Vec::new());
    }

    for entry in fs::read_dir(source)
        .map_err(|error| format!("Failed to inspect {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        /*
         * Don't duplicate recognized Elden Ring asset directories into
         * native/. Mixed mods install those through assets/.
         */
        if source_path.is_dir() {
            let directory_name = entry.file_name().to_string_lossy().to_ascii_lowercase();

            if ASSET_DIRECTORIES
                .iter()
                .any(|asset| asset.eq_ignore_ascii_case(&directory_name))
            {
                continue;
            }

            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            if source_path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|name| name.eq_ignore_ascii_case("regulation.bin"))
                .unwrap_or(false)
            {
                continue;
            }

            fs::copy(&source_path, &destination_path)
                .map_err(|error| format!("Failed to copy {}: {error}", source_path.display()))?;
        }
    }

    let mut relative_dlls = Vec::new();

    for dll in dlls {
        let file_name = dll
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("DLL has an invalid filename: {}", dll.display()))?;

        relative_dlls.push(file_name.to_string());
    }

    Ok(relative_dlls)
}

fn display_name_from_archive(archive_path: &Path) -> String {
    archive_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Imported Mod")
        .to_string()
}

fn sanitize_id(value: &str) -> String {
    let mut result = String::new();
    let mut previous_dash = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !result.is_empty() {
            result.push('-');
            previous_dash = true;
        }
    }

    while result.ends_with('-') {
        result.pop();
    }

    if result.is_empty() {
        "mod".to_string()
    } else {
        result
    }
}

fn unique_mod_id(mods_directory: &Path, base: &str) -> String {
    if !mods_directory.join(base).exists() {
        return base.to_string();
    }

    for number in 2..10000 {
        let candidate = format!("{base}-{number}");

        if !mods_directory.join(&candidate).exists() {
            return candidate;
        }
    }

    format!("{base}-import")
}

fn safe_variant_path(staging: &Path, relative_path: &str) -> Result<PathBuf, String> {
    if relative_path.trim().is_empty() {
        if staging.is_dir() {
            return Ok(staging.to_path_buf());
        }

        return Err("The extracted mod staging directory could not be found.".to_string());
    }

    let relative = safe_relative_path(relative_path)?;
    let result = staging.join(relative);

    if !result.is_dir() {
        return Err(
            "The selected mod variant could not be found after extracting the archive.".to_string(),
        );
    }

    Ok(result)
}

#[tauri::command]
pub async fn pick_mod_archive(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let selected = app
        .dialog()
        .file()
        .add_filter("Elden Ring Mod Archives", &["zip", "rar", "7z"])
        .blocking_pick_file();

    let Some(selected) = selected else {
        return Ok(None);
    };

    let Some(path) = selected.as_path() else {
        return Err("The selected file did not resolve to a normal desktop file path.".to_string());
    };

    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn analyze_mod_archive(archive_path: String) -> Result<ModArchiveAnalysis, String> {
    let archive_path = PathBuf::from(archive_path);

    let format = validate_archive_path(&archive_path)?;
    let staging = staging_directory()?;

    clear_directory(&staging)?;

    let result = (|| {
        let extracted_format = extract_archive(&archive_path, &staging)?;

        let candidates = find_install_candidates(&staging)?;

        if candidates.is_empty() {
            return Err(
                "I couldn't find an installable Elden Ring mod inside this archive. No supported loose-file assets, native DLLs, or Elden Ring Randomizer launcher were detected."
                .to_string(),
            );
        }

        let variants = build_variants(&staging, &candidates)?;

        println!("Mod archive analysis: {}", archive_path.display());

        println!("Archive format: {}", extracted_format.display_name());

        println!("Detected {} installable candidate(s).", candidates.len());

        for (candidate, variant) in candidates.iter().zip(variants.iter()) {
            println!(
                "Candidate: {} [{}] -> {}",
                variant.name,
                candidate.kind.display_name(),
                variant.relative_path
            );
        }

        Ok(ModArchiveAnalysis {
            archive_path: archive_path.to_string_lossy().into_owned(),

            display_name: display_name_from_archive(&archive_path),

            variants,
        })
    })();

    let _ = fs::remove_dir_all(&staging);

    if result.is_err() {
        println!(
            "Archive analysis failed for {} ({})",
            archive_path.display(),
            format.display_name()
        );
    }

    result
}

#[tauri::command]
pub fn import_mod_variant(
    profile_id: String,
    archive_path: String,
    variant_path: String,
    variant_name: String,
    multiple_variants: bool,
) -> Result<InstalledMod, String> {
    let archive_path = PathBuf::from(archive_path);

    validate_archive_path(&archive_path)?;

    let mut profile = load_profile(&profile_id)?;

    let mods_directory = profile_mods_directory(&profile_id)?;

    fs::create_dir_all(&mods_directory)
        .map_err(|error| format!("Failed to create {}: {error}", mods_directory.display()))?;

    let staging = staging_directory()?;

    clear_directory(&staging)?;

    let import_result = (|| {
        let format = extract_archive(&archive_path, &staging)?;

        println!("Installing from {} archive.", format.display_name());

        let candidate_root = safe_variant_path(&staging, &variant_path)?;

        let kind = candidate_kind(&candidate_root)?.ok_or_else(|| {
            "The selected candidate is no longer recognized as an Elden Ring mod.".to_string()
        })?;

        println!("Detected content type: {}", kind.display_name());

        if kind == CandidateKind::Randomizer {
            const RANDOMIZER_ID: &str = "elden-ring-randomizer";

            if profile
                .mods
                .iter()
                .any(|installed_mod| installed_mod.id == RANDOMIZER_ID)
            {
                return Err(
                    "Elden Ring Randomizer is already installed in this profile. Uninstall it before installing another copy."
                        .to_string(),
                );
            }

            let tools_directory = profile_tools_directory(&profile_id)?;
            fs::create_dir_all(&tools_directory).map_err(|error| {
                format!(
                    "Failed to create profile tools directory {}: {error}",
                    tools_directory.display()
                )
            })?;

            let randomizer_directory = tools_directory.join("randomizer");

            if randomizer_directory.exists() {
                fs::remove_dir_all(&randomizer_directory).map_err(|error| {
                    format!(
                        "Failed to clear previous Randomizer files from {}: {error}",
                        randomizer_directory.display()
                    )
                })?;
            }

            copy_directory_recursive(&candidate_root, &randomizer_directory)?;

            let randomizer_executable = randomizer_directory.join("EldenRingRandomizer.exe");

            if !randomizer_executable.is_file() {
                let _ = fs::remove_dir_all(&randomizer_directory);

                return Err(
                    "The Randomizer archive was detected, but EldenRingRandomizer.exe was not preserved during installation."
                        .to_string(),
                );
            }

            let installed_mod = InstalledMod {
                id: RANDOMIZER_ID.to_string(),
                name: "Elden Ring Randomizer".to_string(),
                version: None,
                source: None,
                enabled: true,
                compatibility: CompatibilityLevel::CompatibilityRequired,
                contents: vec![ModContent {
                    content_type: ModContentType::Config,
                    relative_path: "tools/randomizer".to_string(),
                }],
            };

            profile.mods.push(installed_mod.clone());

            if let Err(error) = save_profile(&profile) {
                let _ = fs::remove_dir_all(&randomizer_directory);
                return Err(error);
            }

            println!("Installed special mod: {}", installed_mod.name);
            println!("Installed to: {}", randomizer_directory.display());

            return Ok(installed_mod);
        }

        let archive_display_name = display_name_from_archive(&archive_path);

        let display_name = if multiple_variants {
            format!("{} — {}", archive_display_name, variant_name)
        } else {
            archive_display_name
        };

        let base_id = sanitize_id(&display_name);

        let mod_id = unique_mod_id(&mods_directory, &base_id);

        let mod_directory = mods_directory.join(&mod_id);

        fs::create_dir_all(&mod_directory)
            .map_err(|error| format!("Failed to create {}: {error}", mod_directory.display()))?;

        let mut contents = Vec::new();

        if matches!(kind, CandidateKind::Assets | CandidateKind::Mixed) {
            let assets_directory = mod_directory.join("assets");

            let copied = copy_supported_assets(&candidate_root, &assets_directory)?;

            if copied == 0 {
                let _ = fs::remove_dir_all(&mod_directory);

                return Err(
                    "The mod was detected as containing loose assets, but no supported assets could be copied."
                    .to_string(),
                );
            }

            contents.push(ModContent {
                content_type: ModContentType::Assets,
                relative_path: format!("mods/{mod_id}/assets"),
            });

            println!("Installed loose assets to: {}", assets_directory.display());
        }

        if matches!(kind, CandidateKind::Native | CandidateKind::Mixed) {
            let native_directory = mod_directory.join("native");

            let dlls = copy_native_bundle(&candidate_root, &native_directory)?;

            if dlls.is_empty() {
                let _ = fs::remove_dir_all(&mod_directory);

                return Err(
                    "The mod was detected as native, but no DLL could be installed.".to_string(),
                );
            }

            for dll in dlls {
                println!("Installed native DLL: {}", dll);

                contents.push(ModContent {
                    content_type: ModContentType::NativeDll,

                    relative_path: format!("mods/{mod_id}/native/{dll}"),
                });
            }

            println!(
                "Preserved native companion files in: {}",
                native_directory.display()
            );
        }

        if contents.is_empty() {
            let _ = fs::remove_dir_all(&mod_directory);

            return Err("Nothing installable was found in the selected mod candidate.".to_string());
        }

        let installed_mod = InstalledMod {
            id: mod_id.clone(),
            name: display_name,
            version: None,
            source: None,
            enabled: true,
            compatibility: CompatibilityLevel::Universal,
            contents,
        };

        profile.mods.push(installed_mod.clone());

        if let Err(error) = save_profile(&profile) {
            let _ = fs::remove_dir_all(&mod_directory);

            return Err(error);
        }

        println!("Installed mod: {}", installed_mod.name);

        println!("Installed to: {}", mod_directory.display());

        Ok(installed_mod)
    })();

    let _ = fs::remove_dir_all(&staging);

    import_result
}

#[tauri::command]
pub fn replace_nexus_mod_variant(
    profile_id: String,
    old_mod_id: String,
    archive_path: String,
    variant_path: String,
    variant_name: String,
    multiple_variants: bool,
    nexus_mod_id: u64,
    nexus_file_id: u64,
    nexus_version: Option<String>,
) -> Result<InstalledMod, String> {
    /*
     * SAFETY RULE:
     *
     * Never remove the currently installed mod until the replacement
     * has been completely imported.
     */

    let original_profile = load_profile(&profile_id)?;

    let old_index = original_profile
        .mods
        .iter()
        .position(|installed_mod| installed_mod.id == old_mod_id)
        .ok_or_else(|| {
            format!(
                "Installed mod {} was not found in profile {}.",
                old_mod_id, profile_id,
            )
        })?;

    let old_enabled = original_profile.mods[old_index].enabled;

    /*
     * import_mod_variant installs the replacement as a completely
     * separate mod and appends it to the profile.
     *
     * If this fails, the old mod has not been touched.
     */
    let mut replacement = import_mod_variant(
        profile_id.clone(),
        archive_path,
        variant_path,
        variant_name,
        multiple_variants,
    )?;

    let new_mod_id = replacement.id.clone();

    /*
     * Write Nexus source metadata BEFORE replacing the old profile
     * entry. If metadata recording fails, remove the temporary new
     * installation and leave the old one alone.
     */
    if let Err(error) = record_nexus_mod_install(
        profile_id.clone(),
        new_mod_id.clone(),
        nexus_mod_id,
        nexus_file_id,
        nexus_version,
    ) {
        let _ = uninstall_mod(profile_id.clone(), new_mod_id.clone());

        return Err(format!(
            "The new version was imported, but its Nexus metadata could not be recorded. The update was rolled back: {error}"
        ));
    }

    let mut profile = match load_profile(&profile_id) {
        Ok(profile) => profile,
        Err(error) => {
            let _ = uninstall_mod(profile_id.clone(), new_mod_id.clone());

            return Err(format!(
                "Could not reload the profile after importing the update. The update was rolled back: {error}"
            ));
        }
    };

    let Some(new_index) = profile
        .mods
        .iter()
        .position(|installed_mod| installed_mod.id == new_mod_id)
    else {
        let _ = uninstall_mod(profile_id.clone(), new_mod_id.clone());

        return Err(
            "The replacement mod disappeared from the profile before the update could be finalized."
                .to_string(),
        );
    };

    /*
     * Remove the newly appended entry from its temporary position.
     */
    replacement = profile.mods.remove(new_index);

    /*
     * Removing the appended entry normally does not affect old_index,
     * but calculate the old position again rather than relying on that.
     */
    let Some(current_old_index) = profile
        .mods
        .iter()
        .position(|installed_mod| installed_mod.id == old_mod_id)
    else {
        /*
         * Do NOT delete the new installation here. Something external
         * changed the profile and the safest option is to leave both
         * directories intact for recovery.
         */
        return Err(
            "The original mod disappeared while the update was being finalized. No mod files were deleted."
                .to_string(),
        );
    };

    replacement.enabled = old_enabled;

    /*
     * Put the replacement directly into the old mod load-order slot.
     */
    profile.mods[current_old_index] = replacement.clone();

    if let Err(error) = save_profile(&profile) {
        /*
         * The profile on disk still contains the old version plus the
         * temporary new version, so uninstalling the new one is safe.
         */
        let _ = uninstall_mod(profile_id.clone(), new_mod_id.clone());

        return Err(format!(
            "Could not save the updated load order. The update was rolled back: {error}"
        ));
    }

    /*
     * The profile now points at the replacement.
     * Only NOW may the old files be removed.
     */
    let old_directory = profile_mods_directory(&profile_id)?.join(&old_mod_id);

    if old_directory.exists() {
        if let Err(error) = fs::remove_dir_all(&old_directory) {
            /*
             * The update itself succeeded. Leaving the old directory
             * behind is safer than rolling back a valid profile.
             */
            eprintln!(
                "Updated mod successfully, but could not remove old directory {}: {}",
                old_directory.display(),
                error,
            );
        }
    }

    println!(
        "Updated Nexus mod {}: {} -> {}",
        nexus_mod_id, old_mod_id, replacement.id,
    );

    println!("Preserved load-order position {}.", old_index,);

    Ok(replacement)
}

#[tauri::command]
pub fn list_profile_mods(profile_id: String) -> Result<Vec<InstalledMod>, String> {
    Ok(load_profile(&profile_id)?.mods)
}

#[tauri::command]
pub fn set_mod_enabled(
    profile_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<Vec<InstalledMod>, String> {
    let mut profile = load_profile(&profile_id)?;

    let Some(installed_mod) = profile
        .mods
        .iter_mut()
        .find(|installed_mod| installed_mod.id == mod_id)
    else {
        return Err(format!("Mod '{}' was not found.", mod_id));
    };

    installed_mod.enabled = enabled;

    save_profile(&profile)?;

    Ok(profile.mods)
}

#[tauri::command]
pub fn uninstall_mod(profile_id: String, mod_id: String) -> Result<Vec<InstalledMod>, String> {
    let mut profile = load_profile(&profile_id)?;

    let Some(index) = profile
        .mods
        .iter()
        .position(|installed_mod| installed_mod.id == mod_id)
    else {
        return Err(format!("Mod '{}' was not found.", mod_id));
    };

    let installed_mod = profile.mods[index].clone();
    let actual_id = installed_mod.id.clone();

    profile.mods.remove(index);

    save_profile(&profile)?;

    let mut removed_special_directory = false;

    for content in &installed_mod.contents {
        if !matches!(content.content_type, ModContentType::Config) {
            continue;
        }

        let normalized = content.relative_path.replace('\\', "/");

        if normalized == "tools/randomizer" {
            let special_directory = profile_directory(&profile_id)?.join(&content.relative_path);

            if special_directory.exists() {
                fs::remove_dir_all(&special_directory).map_err(|error| {
                    format!(
                        "The Randomizer was removed from the profile, but its files could not be deleted from {}: {error}",
                        special_directory.display()
                    )
                })?;
            }

            removed_special_directory = true;
        }
    }

    if !removed_special_directory {
        let mod_directory = profile_mods_directory(&profile_id)?.join(actual_id);

        if mod_directory.exists() {
            fs::remove_dir_all(&mod_directory)
                .map_err(|error| {
                    format!(
                        "The mod was removed from the profile, but its files could not be deleted from {}: {error}",
                        mod_directory.display()
                    )
                })?;
        }
    }

    Ok(profile.mods)
}

fn legacy_nexus_identity_matches(text: &str, nexus_mod_id: u64) -> bool {
    let prefix = format!("{nexus_mod_id}-");

    let Some(rest) = text.strip_prefix(&prefix) else {
        return false;
    };

    let Some((possible_file_id, _)) = rest.split_once("-") else {
        return false;
    };

    !possible_file_id.is_empty()
        && possible_file_id
            .chars()
            .all(|character| character.is_ascii_digit())
}

#[tauri::command]
pub fn record_nexus_mod_install(
    profile_id: String,
    installed_mod_id: String,
    nexus_mod_id: u64,
    nexus_file_id: u64,
    nexus_version: Option<String>,
) -> Result<(), String> {
    let mut profile = load_profile(&profile_id)?;

    let installed_mod = profile
        .mods
        .iter()
        .position(|installed_mod| installed_mod.id == installed_mod_id)
        .ok_or_else(|| {
            format!(
                "Installed mod {} was not found in profile {}.",
                installed_mod_id, profile_id,
            )
        })?;

    let installed_mod_id_for_path = profile.mods[installed_mod].id.clone();

    let mod_directory = if installed_mod_id_for_path == "elden-ring-randomizer" {
        profile_tools_directory(&profile_id)?.join("randomizer")
    } else {
        profile_mods_directory(&profile_id)?.join(&installed_mod_id_for_path)
    };

    if !mod_directory.is_dir() {
        return Err(format!(
            "Installed mod directory does not exist: {}",
            mod_directory.display(),
        ));
    }

    profile.mods[installed_mod].source = Some(ModSource::Nexus {
        mod_id: nexus_mod_id,
        file_id: nexus_file_id,
        version: nexus_version.clone(),
    });

    save_profile(&profile)?;

    let metadata = serde_json::json!({
        "provider": "nexus",
        "game": "eldenring",
        "mod_id": nexus_mod_id,
        "file_id": nexus_file_id,
        "version": nexus_version,
    });

    let metadata_text = serde_json::to_string_pretty(&metadata)
        .map_err(|error| format!("Could not serialize Nexus install metadata: {error}"))?;

    let metadata_path = mod_directory.join(".nexus-source.json");

    fs::write(&metadata_path, metadata_text)
        .map_err(|error| format!("Could not write {}: {error}", metadata_path.display(),))?;

    Ok(())
}

#[tauri::command]
pub fn is_nexus_mod_installed(profile_id: String, nexus_mod_id: u64) -> Result<bool, String> {
    let profile = load_profile(&profile_id)?;

    for installed_mod in &profile.mods {
        let metadata_path = profile_mods_directory(&profile_id)?
            .join(&installed_mod.id)
            .join(".nexus-source.json");

        if metadata_path.is_file() {
            if let Ok(text) = fs::read_to_string(&metadata_path) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    let provider_matches = value
                        .get("provider")
                        .and_then(serde_json::Value::as_str)
                        .map(|provider| provider.eq_ignore_ascii_case("nexus"))
                        .unwrap_or(false);

                    let mod_matches = value.get("mod_id").and_then(serde_json::Value::as_u64)
                        == Some(nexus_mod_id);

                    if provider_matches && mod_matches {
                        return Ok(true);
                    }
                }
            }
        }

        /*
         * Migration fallback for Nexus mods installed
         * before .nexus-source.json existed.
         *
         * The downloader names archives:
         *   MODID-FILEID-original-name.zip
         *
         * The importer derives its old ID/name from that
         * archive filename, so this lets existing installs
         * be recognized without reinstalling them.
         */
        if legacy_nexus_identity_matches(&installed_mod.id, nexus_mod_id)
            || legacy_nexus_identity_matches(&installed_mod.name, nexus_mod_id)
        {
            return Ok(true);
        }
    }

    Ok(false)
}
