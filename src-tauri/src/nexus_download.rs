use crate::paths;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::{HashMap, HashSet};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};

const REST_API_BASE: &str = "https://api.nexusmods.com/v1";

const GAME_DOMAIN: &str = "eldenring";

const APPLICATION_NAME: &str = "EldenModManager";

const APPLICATION_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_DOWNLOAD_SIZE: u64 = 8 * 1024 * 1024 * 1024;

static ACTIVE_NXM_DOWNLOADS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

#[derive(Debug)]
struct PendingNxmAuthorization {
    key: String,
    expires: u64,
}

static PENDING_NXM_AUTHORIZATIONS: OnceLock<
    Mutex<HashMap<String, mpsc::Sender<PendingNxmAuthorization>>>,
> = OnceLock::new();

fn pending_nxm_authorizations(
) -> &'static Mutex<HashMap<String, mpsc::Sender<PendingNxmAuthorization>>> {
    PENDING_NXM_AUTHORIZATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn pending_nxm_identity(mod_id: u64, file_id: u64) -> String {
    format!("{mod_id}:{file_id}")
}

fn register_pending_nxm_authorization(
    mod_id: u64,
    file_id: u64,
) -> Result<mpsc::Receiver<PendingNxmAuthorization>, String> {
    let (sender, receiver) = mpsc::channel();

    pending_nxm_authorizations()
        .lock()
        .map_err(|_| "Could not lock pending Nexus authorization state.".to_string())?
        .insert(pending_nxm_identity(mod_id, file_id), sender);

    Ok(receiver)
}

fn clear_pending_nxm_authorization(mod_id: u64, file_id: u64) {
    if let Ok(mut pending) = pending_nxm_authorizations().lock() {
        pending.remove(&pending_nxm_identity(mod_id, file_id));
    }
}

fn deliver_pending_nxm_authorization(mod_id: u64, file_id: u64, key: String, expires: u64) -> bool {
    let sender = pending_nxm_authorizations()
        .lock()
        .ok()
        .and_then(|mut pending| pending.remove(&pending_nxm_identity(mod_id, file_id)));

    let Some(sender) = sender else {
        return false;
    };

    sender
        .send(PendingNxmAuthorization { key, expires })
        .is_ok()
}

static CANCELLED_DOWNLOADS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn cancelled_downloads() -> &'static Mutex<HashSet<String>> {
    CANCELLED_DOWNLOADS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn download_identity(mod_id: u64, file_id: u64) -> String {
    format!("{mod_id}:{file_id}")
}

fn clear_download_cancel(mod_id: u64, file_id: u64) {
    if let Ok(mut cancelled) = cancelled_downloads().lock() {
        cancelled.remove(&download_identity(mod_id, file_id));
    }
}

fn download_is_cancelled(mod_id: u64, file_id: u64) -> bool {
    cancelled_downloads()
        .lock()
        .map(|cancelled| cancelled.contains(&download_identity(mod_id, file_id)))
        .unwrap_or(false)
}

#[tauri::command]
pub fn cancel_nexus_download(mod_id: u64, file_id: u64) -> Result<(), String> {
    let mut cancelled = cancelled_downloads()
        .lock()
        .map_err(|_| "Could not lock download cancellation state.".to_string())?;

    cancelled.insert(download_identity(mod_id, file_id));

    Ok(())
}

#[derive(Debug, Deserialize)]
struct SavedNexusConfig {
    api_key: String,

    #[serde(default)]
    is_premium: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct NexusFile {
    pub file_id: u64,

    pub name: String,

    pub file_name: String,

    pub version: String,

    pub category_name: String,

    pub is_primary: bool,

    pub size_bytes: Option<u64>,

    pub uploaded_timestamp: Option<u64>,

    pub description: String,

    pub supported_archive: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct InstalledNexusSource {
    provider: String,
    game: String,
    mod_id: u64,
    file_id: u64,

    #[serde(default)]
    version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NexusModUpdate {
    pub local_mod_id: String,

    pub nexus_mod_id: u64,

    pub installed_file_id: u64,
    pub installed_version: String,

    pub latest_file_id: Option<u64>,
    pub latest_version: Option<String>,
    pub latest_file_name: Option<String>,
    pub latest_uploaded_timestamp: Option<u64>,

    pub update_available: bool,

    /*
     * true means we know this came from Nexus,
     * but we cannot safely decide which newer file
     * should replace it automatically.
     *
     * This is important for OPTIONAL/UPDATE/MISC files,
     * because another file in the same Nexus mod is not
     * necessarily a replacement for the installed one.
     */
    pub manual_check_required: bool,

    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NexusDownloadProgress {
    pub mod_id: u64,
    pub file_id: u64,

    pub file_name: String,

    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,

    pub percent: Option<f64>,

    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NexusDownloadComplete {
    pub mod_id: u64,
    pub file_id: u64,

    pub archive_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NexusDownloadError {
    pub mod_id: Option<u64>,
    pub file_id: Option<u64>,

    pub message: String,
}

#[derive(Debug)]
struct ParsedNxmUrl {
    mod_id: u64,
    file_id: u64,

    key: String,
    expires: u64,
}

#[derive(Debug)]
struct DownloadFileDetails {
    file_name: String,
    size_bytes: Option<u64>,
}

fn nexus_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate app data directory: {error}"))?;

    Ok(directory.join("nexus.json"))
}

fn read_nexus_config(app: &AppHandle) -> Result<SavedNexusConfig, String> {
    let path = nexus_config_path(app)?;

    if !path.is_file() {
        return Err("Connect your Nexus Mods account first.".to_string());
    }

    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read Nexus settings: {error}"))?;

    serde_json::from_str::<SavedNexusConfig>(&text)
        .map_err(|error| format!("Could not parse Nexus settings: {error}"))
}

fn nexus_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(format!("{APPLICATION_NAME}/{APPLICATION_VERSION}"))
        .build()
        .map_err(|error| format!("Could not create Nexus HTTP client: {error}"))
}

fn nexus_request(client: &reqwest::Client, api_key: &str, url: &str) -> reqwest::RequestBuilder {
    client
        .get(url)
        .header("apikey", api_key)
        .header("Application-Name", APPLICATION_NAME)
        .header("Application-Version", APPLICATION_VERSION)
        .header("Accept", "application/json")
}

fn shortened_body(body: &str) -> String {
    const MAX: usize = 1000;

    let body = body.trim();

    if body.len() <= MAX {
        return body.to_string();
    }

    let mut end = MAX;

    while !body.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}…", &body[..end],)
}

async fn get_json_value(
    client: &reqwest::Client,
    api_key: &str,
    url: &str,
) -> Result<Value, String> {
    let response = nexus_request(client, api_key, url)
        .send()
        .await
        .map_err(|error| format!("Nexus request failed: {error}"))?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Nexus response: {error}"))?;

    if !status.is_success() {
        if body.trim().is_empty() {
            return Err(format!("Nexus returned HTTP {}.", status.as_u16(),));
        }

        return Err(format!(
            "Nexus returned HTTP {}: {}",
            status.as_u16(),
            shortened_body(&body,),
        ));
    }

    serde_json::from_str::<Value>(&body).map_err(|error| {
        format!(
            "Could not decode Nexus response: {error}. Response: {}",
            shortened_body(&body,),
        )
    })
}

async fn ensure_mod_allowed(
    client: &reqwest::Client,
    api_key: &str,
    mod_id: u64,
) -> Result<(), String> {
    let url = format!("{REST_API_BASE}/games/{GAME_DOMAIN}/mods/{mod_id}.json");

    let details = get_json_value(client, api_key, &url).await?;

    /*
     * Fail closed here.
     *
     * Browse Mods already filters adult content,
     * but nxm:// can be triggered externally.
     * We therefore verify the mod again before
     * downloading anything.
     */
    match details
        .get("contains_adult_content")
        .and_then(Value::as_bool)
    {
        Some(false) => {}

        Some(true) => {
            return Err("Age-restricted mods are not available through this launcher.".to_string());
        }

        None => {
            return Err(
                "Could not verify this mod's content rating, so the download was blocked."
                    .to_string(),
            );
        }
    }

    if let Some(false) = details.get("available").and_then(Value::as_bool) {
        return Err("This Nexus mod is not currently available.".to_string());
    }

    if let Some(status) = details.get("status").and_then(Value::as_str) {
        if !status.eq_ignore_ascii_case("published") {
            return Err(format!(
                "This Nexus mod is not published. Current status: {status}"
            ));
        }
    }

    Ok(())
}

fn value_as_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
            .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
    })
}

fn value_as_string(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or("").to_string()
}

fn size_bytes_from_json(value: &Value) -> Option<u64> {
    if let Some(size) = value_as_u64(value.get("size_in_bytes")) {
        return Some(size);
    }

    if let Some(size_kb) = value_as_u64(value.get("size_kb")) {
        return Some(size_kb.saturating_mul(1024));
    }

    /*
     * Nexus V1 commonly exposes `size`
     * in KiB.
     */
    value_as_u64(value.get("size")).map(|size_kb| size_kb.saturating_mul(1024))
}

fn is_supported_archive(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "zip" | "rar" | "7z"
            )
        })
        .unwrap_or(false)
}

fn parse_file(value: &Value) -> Option<NexusFile> {
    let file_id = value_as_u64(value.get("file_id"))?;

    let name = value_as_string(value.get("name"));

    let file_name = value_as_string(value.get("file_name"));

    let version = value_as_string(value.get("version"));

    let category_name = value_as_string(value.get("category_name"));

    let description = value_as_string(value.get("description"));

    let is_primary = value
        .get("is_primary")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let uploaded_timestamp = value_as_u64(value.get("uploaded_timestamp"));

    let size_bytes = size_bytes_from_json(value);

    Some(NexusFile {
        file_id,

        name,

        file_name: file_name.clone(),

        version,

        category_name,

        is_primary,

        size_bytes,

        uploaded_timestamp,

        description,

        supported_archive: is_supported_archive(&file_name),
    })
}

fn category_rank(category: &str) -> u8 {
    if category.eq_ignore_ascii_case("MAIN") {
        0
    } else if category.eq_ignore_ascii_case("OPTIONAL") {
        1
    } else if category.eq_ignore_ascii_case("UPDATE") {
        2
    } else if category.eq_ignore_ascii_case("MISCELLANEOUS") {
        3
    } else if category.eq_ignore_ascii_case("ARCHIVED") {
        9
    } else {
        4
    }
}

#[tauri::command]
pub async fn get_nexus_mod_files(app: AppHandle, mod_id: u64) -> Result<Vec<NexusFile>, String> {
    let config = read_nexus_config(&app)?;

    let client = nexus_client()?;

    ensure_mod_allowed(&client, &config.api_key, mod_id).await?;

    let url = format!("{REST_API_BASE}/games/{GAME_DOMAIN}/mods/{mod_id}/files.json");

    let response = get_json_value(&client, &config.api_key, &url).await?;

    let files = response
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "Nexus returned a file list in an unexpected format.".to_string())?;

    let mut result = files.iter().filter_map(parse_file).collect::<Vec<_>>();

    result.sort_by(|left, right| {
        right
            .is_primary
            .cmp(&left.is_primary)
            .then_with(|| {
                category_rank(&left.category_name).cmp(&category_rank(&right.category_name))
            })
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
    });

    Ok(result)
}

async fn fetch_file_details(
    client: &reqwest::Client,
    api_key: &str,
    mod_id: u64,
    file_id: u64,
) -> Result<DownloadFileDetails, String> {
    let url = format!("{REST_API_BASE}/games/{GAME_DOMAIN}/mods/{mod_id}/files/{file_id}.json");

    let details = get_json_value(client, api_key, &url).await?;

    let actual_file_id = value_as_u64(details.get("file_id"))
        .ok_or_else(|| "Nexus file response did not contain a file ID.".to_string())?;

    if actual_file_id != file_id {
        return Err("Nexus returned metadata for a different file.".to_string());
    }

    let file_name = value_as_string(details.get("file_name"));

    if file_name.trim().is_empty() {
        return Err("Nexus did not provide a filename for this download.".to_string());
    }

    if !is_supported_archive(&file_name) {
        return Err(
            format!(
                "This Nexus file is not a supported archive: {file_name}. Elden Mod Manager currently imports ZIP, RAR, and 7Z files."
            ),
        );
    }

    Ok(DownloadFileDetails {
        file_name,

        size_bytes: size_bytes_from_json(&details),
    })
}

async fn resolve_download_url(
    client: &reqwest::Client,
    api_key: &str,
    mod_id: u64,
    file_id: u64,
    nxm_auth: Option<(&str, u64)>,
) -> Result<String, String> {
    let base = format!(
        "{REST_API_BASE}/games/{GAME_DOMAIN}/mods/{mod_id}/files/{file_id}/download_link.json"
    );

    let mut url = reqwest::Url::parse(&base)
        .map_err(|error| format!("Could not construct Nexus download URL: {error}"))?;

    if let Some((key, expires)) = nxm_auth {
        url.query_pairs_mut()
            .append_pair("key", key)
            .append_pair("expires", &expires.to_string());
    }

    let response = get_json_value(client, api_key, url.as_str()).await?;

    let links = response
        .as_array()
        .ok_or_else(|| "Nexus returned an unexpected download-link response.".to_string())?;

    for link in links {
        let uri = link
            .get("URI")
            .or_else(|| link.get("uri"))
            .and_then(Value::as_str);

        let Some(uri) = uri else {
            continue;
        };

        let parsed = reqwest::Url::parse(uri)
            .map_err(|error| format!("Nexus returned an invalid CDN URL: {error}"))?;

        /*
         * Never allow an nxm:// payload to turn
         * this downloader into a generic HTTP
         * downloader.
         */
        if parsed.scheme() != "https" {
            continue;
        }

        return Ok(parsed.to_string());
    }

    Err("Nexus did not return a usable HTTPS download mirror.".to_string())
}

fn downloads_directory(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate app data directory: {error}"))?
        .join("downloads");

    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create download directory {}: {error}",
            directory.display(),
        )
    })?;

    Ok(directory)
}

fn sanitize_filename(name: &str) -> String {
    let mut result = String::with_capacity(name.len());

    for character in name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | ' ') {
            result.push(character);
        } else {
            result.push('_');
        }
    }

    let result = result.trim();

    if result.is_empty() {
        "nexus-mod.zip".to_string()
    } else {
        result.to_string()
    }
}

fn emit_progress(
    app: &AppHandle,
    mod_id: u64,
    file_id: u64,
    file_name: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    state: &str,
) {
    let percent = total_bytes
        .filter(|total| *total > 0)
        .map(|total| (downloaded_bytes as f64 / total as f64) * 100.0);

    let _ = app.emit(
        "nexus-download-progress",
        NexusDownloadProgress {
            mod_id,
            file_id,

            file_name: file_name.to_string(),

            downloaded_bytes,
            total_bytes,

            percent,

            state: state.to_string(),
        },
    );
}

async fn download_archive(
    app: &AppHandle,
    client: &reqwest::Client,
    mod_id: u64,
    file_id: u64,
    details: &DownloadFileDetails,
    download_url: &str,
) -> Result<String, String> {
    clear_download_cancel(mod_id, file_id);

    let directory = downloads_directory(app)?;

    let safe_name = sanitize_filename(&details.file_name);

    let final_path = directory.join(format!("{mod_id}-{file_id}-{safe_name}"));

    let part_path = directory.join(format!("{mod_id}-{file_id}-{safe_name}.part"));

    if part_path.exists() {
        let _ = fs::remove_file(&part_path);
    }

    emit_progress(
        app,
        mod_id,
        file_id,
        &details.file_name,
        0,
        details.size_bytes,
        "connecting",
    );

    /*
     * IMPORTANT:
     *
     * This is a CDN request, not an API request.
     * Do NOT send the user's Nexus API key to
     * the CDN host.
     */
    let mut response = client
        .get(download_url)
        .send()
        .await
        .map_err(|error| format!("Could not start Nexus download: {error}"))?;

    let status = response.status();

    if !status.is_success() {
        return Err(format!(
            "Nexus download server returned HTTP {}.",
            status.as_u16(),
        ));
    }

    let total_bytes = response.content_length().or(details.size_bytes);

    if let Some(total) = total_bytes {
        if total > MAX_DOWNLOAD_SIZE {
            return Err(
                "This mod archive is larger than the launcher's 8 GB download limit.".to_string(),
            );
        }
    }

    let mut file = File::create(&part_path).map_err(|error| {
        format!(
            "Could not create temporary download {}: {error}",
            part_path.display(),
        )
    })?;

    let mut downloaded = 0u64;

    let result: Result<(), String> = async {
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| format!("Nexus download failed while receiving data: {error}"))?
        {
            if download_is_cancelled(mod_id, file_id) {
                emit_progress(
                    app,
                    mod_id,
                    file_id,
                    &details.file_name,
                    downloaded,
                    total_bytes,
                    "cancelled",
                );

                return Err("Download cancelled.".to_string());
            }

            downloaded = downloaded.saturating_add(chunk.len() as u64);

            if downloaded > MAX_DOWNLOAD_SIZE {
                return Err("The downloaded archive exceeded the 8 GB safety limit.".to_string());
            }

            file.write_all(&chunk)
                .map_err(|error| format!("Could not write Nexus download: {error}"))?;

            emit_progress(
                app,
                mod_id,
                file_id,
                &details.file_name,
                downloaded,
                total_bytes,
                "downloading",
            );
        }

        file.flush()
            .map_err(|error| format!("Could not finish writing Nexus download: {error}"))?;

        Ok(())
    }
    .await;

    if let Err(error) = result {
        drop(file);

        let _ = fs::remove_file(&part_path);

        return Err(error);
    }

    drop(file);

    if final_path.exists() {
        fs::remove_file(&final_path).map_err(|error| {
            format!(
                "Could not replace old download {}: {error}",
                final_path.display(),
            )
        })?;
    }

    fs::rename(&part_path, &final_path)
        .map_err(|error| format!("Could not finalize Nexus download: {error}"))?;

    emit_progress(
        app,
        mod_id,
        file_id,
        &details.file_name,
        downloaded,
        total_bytes,
        "complete",
    );

    Ok(final_path.to_string_lossy().into_owned())
}

async fn perform_download(
    app: &AppHandle,
    mod_id: u64,
    file_id: u64,
    nxm_auth: Option<(&str, u64)>,
) -> Result<String, String> {
    let config = read_nexus_config(app)?;

    let client = nexus_client()?;

    ensure_mod_allowed(&client, &config.api_key, mod_id).await?;

    let details = fetch_file_details(&client, &config.api_key, mod_id, file_id).await?;

    let download_url =
        resolve_download_url(&client, &config.api_key, mod_id, file_id, nxm_auth).await?;

    download_archive(app, &client, mod_id, file_id, &details, &download_url).await
}

#[tauri::command]
pub async fn download_nexus_mod_file(
    app: AppHandle,
    mod_id: u64,
    file_id: u64,
) -> Result<String, String> {
    let config = read_nexus_config(&app)?;

    if !config.is_premium {
        let receiver = register_pending_nxm_authorization(mod_id, file_id)?;

        emit_progress(
            &app,
            mod_id,
            file_id,
            "Nexus Mod",
            0,
            None,
            "awaiting_authorization",
        );

        if let Err(error) = open_nexus_download_authorization(app.clone(), mod_id, file_id).await {
            clear_pending_nxm_authorization(mod_id, file_id);

            return Err(error);
        }

        let authorization = tauri::async_runtime::spawn_blocking(move || {
            receiver.recv_timeout(Duration::from_secs(600))
        })
        .await
        .map_err(|error| format!("Nexus authorization worker failed: {error}"))?
        .map_err(|_| "Timed out waiting for Nexus Mod Manager Download authorization.".to_string());

        clear_pending_nxm_authorization(mod_id, file_id);

        let authorization = authorization?;

        return perform_download(
            &app,
            mod_id,
            file_id,
            Some((&authorization.key, authorization.expires)),
        )
        .await;
    }

    perform_download(&app, mod_id, file_id, None).await
}

fn nexus_source_path(profile_id: &str, local_mod_id: &str) -> Result<PathBuf, String> {
    Ok(paths::profiles_dir()?
        .join(profile_id)
        .join("mods")
        .join(local_mod_id)
        .join(".nexus-source.json"))
}

fn read_installed_nexus_source(
    profile_id: &str,
    local_mod_id: &str,
) -> Result<Option<InstalledNexusSource>, String> {
    let path = nexus_source_path(profile_id, local_mod_id)?;

    if !path.is_file() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read {}: {error}", path.display(),))?;

    let source = serde_json::from_str::<InstalledNexusSource>(&text)
        .map_err(|error| format!("Could not parse {}: {error}", path.display(),))?;

    if !source.provider.eq_ignore_ascii_case("nexus") {
        return Ok(None);
    }

    if !source.game.eq_ignore_ascii_case(GAME_DOMAIN) {
        return Ok(None);
    }

    Ok(Some(source))
}

fn file_is_main_candidate(file: &NexusFile) -> bool {
    file.supported_archive
        && (file.is_primary || file.category_name.eq_ignore_ascii_case("MAIN"))
        && !file.category_name.eq_ignore_ascii_case("ARCHIVED")
}

fn newer_file(left: &NexusFile, right: &NexusFile) -> std::cmp::Ordering {
    left.uploaded_timestamp
        .unwrap_or(0)
        .cmp(&right.uploaded_timestamp.unwrap_or(0))
        .then_with(|| left.file_id.cmp(&right.file_id))
}

#[tauri::command]
pub async fn check_nexus_mod_updates(
    app: AppHandle,
    profile_id: String,
) -> Result<Vec<NexusModUpdate>, String> {
    let mods_directory = paths::profiles_dir()?.join(&profile_id).join("mods");

    if !mods_directory.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&mods_directory).map_err(|error| {
        format!(
            "Could not inspect profile mods directory {}: {error}",
            mods_directory.display(),
        )
    })?;

    let mut sources: Vec<(String, InstalledNexusSource)> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("Could not inspect installed mod: {error}"))?;

        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let local_mod_id = entry.file_name().to_string_lossy().into_owned();

        if let Some(source) = read_installed_nexus_source(&profile_id, &local_mod_id)? {
            sources.push((local_mod_id, source));
        }
    }

    let mut result = Vec::with_capacity(sources.len());

    for (local_mod_id, source) in sources {
        /*
         * Reuse the same Nexus file loader already
         * used by the Browse Mods install flow.
         *
         * That means adult-content blocking and the
         * existing Nexus validation stay centralized.
         */
        let files = get_nexus_mod_files(app.clone(), source.mod_id).await?;

        let installed = files.iter().find(|file| file.file_id == source.file_id);

        let Some(installed_file) = installed else {
            result.push(NexusModUpdate {
                local_mod_id,

                nexus_mod_id: source.mod_id,

                installed_file_id: source.file_id,

                installed_version: source.version,

                latest_file_id: None,

                latest_version: None,

                latest_file_name: None,

                latest_uploaded_timestamp: None,

                update_available: false,

                manual_check_required: true,

                reason: "The installed Nexus file is no longer present in the current file list."
                    .to_string(),
            });

            continue;
        };

        /*
         * Do not guess replacements for optional,
         * miscellaneous, or patch/update files.
         *
         * Nexus mods can contain several unrelated
         * downloads. Automatically replacing one
         * optional variant with another would be wrong.
         */
        if !file_is_main_candidate(installed_file) {
            result.push(
                NexusModUpdate {
                    local_mod_id,

                    nexus_mod_id:
                    source.mod_id,

                    installed_file_id:
                    source.file_id,

                    installed_version:
                    if source.version
                        .trim()
                        .is_empty()
                    {
                        installed_file
                        .version
                        .clone()
                    } else {
                        source.version
                    },

                    latest_file_id:
                    None,

                    latest_version:
                    None,

                    latest_file_name:
                    None,

                    latest_uploaded_timestamp:
                    None,

                    update_available:
                    false,

                    manual_check_required:
                    true,

                    reason:
                    format!(
                        "Installed Nexus file is in category {}. Automatic replacement is disabled for non-main variants.",
                        installed_file.category_name,
                    ),
                },
            );

            continue;
        }

        let latest = files
            .iter()
            .filter(|file| file_is_main_candidate(file))
            .max_by(|left, right| newer_file(left, right));

        let Some(latest) = latest else {
            result.push(NexusModUpdate {
                local_mod_id,

                nexus_mod_id: source.mod_id,

                installed_file_id: source.file_id,

                installed_version: source.version,

                latest_file_id: None,

                latest_version: None,

                latest_file_name: None,

                latest_uploaded_timestamp: None,

                update_available: false,

                manual_check_required: true,

                reason: "Nexus did not return a supported main archive for this mod.".to_string(),
            });

            continue;
        };

        let installed_timestamp = installed_file.uploaded_timestamp.unwrap_or(0);

        let latest_timestamp = latest.uploaded_timestamp.unwrap_or(0);

        let update_available = latest.file_id != installed_file.file_id
            && (latest_timestamp > installed_timestamp
                || (latest_timestamp == installed_timestamp
                    && latest.file_id > installed_file.file_id));

        result.push(NexusModUpdate {
            local_mod_id,

            nexus_mod_id: source.mod_id,

            installed_file_id: source.file_id,

            installed_version: if source.version.trim().is_empty() {
                installed_file.version.clone()
            } else {
                source.version
            },

            latest_file_id: Some(latest.file_id),

            latest_version: Some(latest.version.clone()),

            latest_file_name: Some(latest.file_name.clone()),

            latest_uploaded_timestamp: latest.uploaded_timestamp,

            update_available,

            manual_check_required: false,

            reason: if update_available {
                "A newer Nexus main file is available.".to_string()
            } else {
                "Installed Nexus file is current.".to_string()
            },
        });
    }

    result.sort_by(|left, right| {
        right
            .update_available
            .cmp(&left.update_available)
            .then_with(|| left.local_mod_id.cmp(&right.local_mod_id))
    });

    Ok(result)
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|error| format!("Could not open browser: {error}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|error| format!("Could not open browser: {error}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|error| format!("Could not open browser: {error}"))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn open_nexus_download_authorization(
    app: AppHandle,
    mod_id: u64,
    file_id: u64,
) -> Result<(), String> {
    let config = read_nexus_config(&app)?;

    let client = nexus_client()?;

    ensure_mod_allowed(&client, &config.api_key, mod_id).await?;

    /*
     * Verify the file actually exists and is an
     * archive our importer understands before
     * sending the user to Nexus.
     */
    fetch_file_details(&client, &config.api_key, mod_id, file_id).await?;

    let url = format!(
        "https://www.nexusmods.com/{GAME_DOMAIN}/mods/{mod_id}?tab=files&file_id={file_id}&nmm=1"
    );

    open_browser(&url)
}

fn unix_time() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("System clock error: {error}"))
}

fn parse_nxm_url(raw: &str) -> Result<ParsedNxmUrl, String> {
    let url = reqwest::Url::parse(raw).map_err(|_| "Invalid NXM download URL.".to_string())?;

    if url.scheme() != "nxm" {
        return Err("Unsupported deep-link protocol.".to_string());
    }

    let game = url.host_str().unwrap_or("");

    if !game.eq_ignore_ascii_case(GAME_DOMAIN) {
        return Err("This NXM link is not for Elden Ring.".to_string());
    }

    let segments = url
        .path_segments()
        .ok_or_else(|| "Invalid Nexus download path.".to_string())?
        .collect::<Vec<_>>();

    if segments.len() != 4 || segments[0] != "mods" || segments[2] != "files" {
        return Err("Invalid Nexus Mod Manager download path.".to_string());
    }

    let mod_id = segments[1]
        .parse::<u64>()
        .map_err(|_| "Invalid Nexus mod ID.".to_string())?;

    let file_id = segments[3]
        .parse::<u64>()
        .map_err(|_| "Invalid Nexus file ID.".to_string())?;

    let mut key: Option<String> = None;

    let mut expires: Option<u64> = None;

    for (name, value) in url.query_pairs() {
        match name.as_ref() {
            "key" => {
                key = Some(value.into_owned());
            }

            "expires" => {
                expires = value.parse::<u64>().ok();
            }

            _ => {}
        }
    }

    let key = key
        .filter(|value| !value.trim().is_empty() && value.len() <= 2048)
        .ok_or_else(|| "The NXM link did not contain a valid authorization key.".to_string())?;

    let expires = expires
        .ok_or_else(|| "The NXM link did not contain a valid expiration time.".to_string())?;

    if expires <= unix_time()? {
        return Err(
            "This Nexus download authorization has expired. Start the download again from Nexus."
                .to_string(),
        );
    }

    Ok(ParsedNxmUrl {
        mod_id,
        file_id,

        key,
        expires,
    })
}

fn active_downloads() -> &'static Mutex<HashSet<String>> {
    ACTIVE_NXM_DOWNLOADS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn claim_nxm_download(identity: &str) -> bool {
    let Ok(mut active) = active_downloads().lock() else {
        return false;
    };

    active.insert(identity.to_string())
}

fn release_nxm_download(identity: &str) {
    if let Ok(mut active) = active_downloads().lock() {
        active.remove(identity);
    }
}

pub async fn handle_nxm_url(app: AppHandle, raw_url: String) -> Result<(), String> {
    let parsed = match parse_nxm_url(&raw_url) {
        Ok(parsed) => parsed,

        Err(error) => {
            let _ = app.emit(
                "nexus-download-error",
                NexusDownloadError {
                    mod_id: None,
                    file_id: None,

                    message: error.clone(),
                },
            );

            return Err(error);
        }
    };

    /*
     * A modpack import may already be waiting for this
     * exact Nexus authorization.
     *
     * In that case, give the temporary key to the
     * importer instead of starting a second download.
     */
    if deliver_pending_nxm_authorization(
        parsed.mod_id,
        parsed.file_id,
        parsed.key.clone(),
        parsed.expires,
    ) {
        return Ok(());
    }

    /*
     * The exact key is never logged.
     *
     * It is only used here to deduplicate the same
     * deep-link while the download is active.
     */
    let identity = format!(
        "{}:{}:{}:{}",
        parsed.mod_id, parsed.file_id, parsed.expires, parsed.key,
    );

    if !claim_nxm_download(&identity) {
        return Ok(());
    }

    let result = perform_download(
        &app,
        parsed.mod_id,
        parsed.file_id,
        Some((&parsed.key, parsed.expires)),
    )
    .await;

    release_nxm_download(&identity);

    match result {
        Ok(archive_path) => {
            let _ = app.emit(
                "nexus-download-complete",
                NexusDownloadComplete {
                    mod_id: parsed.mod_id,

                    file_id: parsed.file_id,

                    archive_path,
                },
            );

            Ok(())
        }

        Err(error) => {
            let _ = app.emit(
                "nexus-download-error",
                NexusDownloadError {
                    mod_id: Some(parsed.mod_id),

                    file_id: Some(parsed.file_id),

                    message: error.clone(),
                },
            );

            Err(error)
        }
    }
}
