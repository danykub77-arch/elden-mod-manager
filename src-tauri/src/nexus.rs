use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
use tauri::{AppHandle, Manager};

const REST_API_BASE: &str = "https://api.nexusmods.com/v1";
const GRAPHQL_API: &str = "https://api.nexusmods.com/v2/graphql";
const GAME_DOMAIN: &str = "eldenring";

const APPLICATION_NAME: &str = "EldenModManager";
const APPLICATION_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_PAGE_SIZE: u32 = 50;
const DEFAULT_PAGE_SIZE: u32 = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusAccountStatus {
    pub connected: bool,
    pub username: Option<String>,
    pub is_premium: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct NexusValidateResponse {
    name: String,

    #[serde(default)]
    is_premium: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedNexusConfig {
    api_key: String,
    username: String,
    is_premium: bool,
}

#[derive(Debug, Serialize)]
pub struct NexusBrowseMod {
    pub mod_id: u64,
    pub name: String,
    pub summary: String,
    pub version: String,
    pub author: String,

    pub endorsements: u64,
    pub downloads: u64,

    pub created_at: Option<String>,
    pub updated_at: Option<String>,

    pub picture_url: Option<String>,
    pub category: Option<String>,

    pub nexus_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NexusTag {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct NexusTagsData {
    mods: NexusTagsMods,
}

#[derive(Debug, Deserialize)]
struct NexusTagsMods {
    #[serde(rename = "facetsData")]
    facets_data: Value,
}

#[derive(Debug, Serialize)]
pub struct NexusBrowseResult {
    pub mods: Vec<NexusBrowseMod>,

    pub page: u32,
    pub page_size: u32,

    pub total_count: u64,
    pub total_pages: u64,

    pub has_previous: bool,
    pub has_next: bool,
}

#[derive(Debug, Deserialize)]
struct GraphQlEnvelope<T> {
    data: Option<T>,

    #[serde(default)]
    errors: Vec<GraphQlError>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct BrowseData {
    mods: BrowseModsConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowseModsConnection {
    #[serde(default)]
    nodes: Vec<GraphQlMod>,

    #[serde(default)]
    total_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQlMod {
    mod_id: u64,

    #[serde(default)]
    name: String,

    #[serde(default)]
    summary: String,

    #[serde(default)]
    version: String,

    #[serde(default)]
    author: String,

    #[serde(default)]
    downloads: u64,

    #[serde(default)]
    endorsements: u64,

    #[serde(default)]
    created_at: Option<String>,

    #[serde(default)]
    updated_at: Option<String>,

    #[serde(default)]
    picture_url: Option<String>,

    #[serde(default)]
    adult: bool,

    #[serde(default)]
    status: Option<String>,

    #[serde(default)]
    mod_category: Option<GraphQlCategory>,
}

#[derive(Debug, Deserialize)]
struct GraphQlCategory {
    #[serde(default)]
    name: String,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate app data directory: {error}"))?;

    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create app data directory: {error}"))?;

    Ok(directory.join("nexus.json"))
}

fn read_config(app: &AppHandle) -> Result<Option<SavedNexusConfig>, String> {
    let path = config_path(app)?;

    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read Nexus settings: {error}"))?;

    let config = serde_json::from_str::<SavedNexusConfig>(&text)
        .map_err(|error| format!("Could not parse Nexus settings: {error}"))?;

    Ok(Some(config))
}

fn write_config(app: &AppHandle, config: &SavedNexusConfig) -> Result<(), String> {
    let path = config_path(app)?;

    let text = serde_json::to_string_pretty(config)
        .map_err(|error| format!("Could not encode Nexus settings: {error}"))?;

    fs::write(path, text).map_err(|error| format!("Could not save Nexus settings: {error}"))
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(format!("{APPLICATION_NAME}/{APPLICATION_VERSION}"))
        .build()
        .map_err(|error| format!("Could not create Nexus client: {error}"))
}

fn nexus_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    api_key: &str,
) -> reqwest::RequestBuilder {
    client
        .request(method, url)
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

    format!("{}…", &body[..end])
}

async fn get_json<T>(client: &reqwest::Client, api_key: &str, url: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = nexus_request(client, reqwest::Method::GET, url, api_key)
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
            shortened_body(&body),
        ));
    }

    serde_json::from_str::<T>(&body).map_err(|error| {
        format!(
            "Could not decode Nexus response: {error}. Response: {}",
            shortened_body(&body),
        )
    })
}

async fn graphql<T>(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    variables: Value,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = nexus_request(client, reqwest::Method::POST, GRAPHQL_API, api_key)
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query,
            "variables": variables,
        }))
        .send()
        .await
        .map_err(|error| format!("Nexus GraphQL request failed: {error}"))?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Nexus GraphQL response: {error}"))?;

    if !status.is_success() {
        return Err(format!(
            "Nexus GraphQL returned HTTP {}{}",
            status.as_u16(),
            if body.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", shortened_body(&body),)
            },
        ));
    }

    let envelope = serde_json::from_str::<GraphQlEnvelope<T>>(&body).map_err(|error| {
        format!(
            "Could not decode Nexus GraphQL response: {error}. Response: {}",
            shortened_body(&body),
        )
    })?;

    if !envelope.errors.is_empty() {
        let messages = envelope
            .errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; ");

        return Err(format!("Nexus GraphQL error: {messages}"));
    }

    envelope
        .data
        .ok_or_else(|| "Nexus returned an empty GraphQL response.".to_string())
}

fn browse_sort(sort: &str, searching: bool) -> Value {
    if searching && sort == "relevance" {
        return json!([
            {
                "relevance": {
                    "direction": "DESC"
                }
            }
        ]);
    }

    match sort {
        "downloads" => json!([
            {
                "downloads": {
                    "direction": "DESC"
                }
            }
        ]),

        "new" => json!([
            {
                "createdAt": {
                    "direction": "DESC"
                }
            }
        ]),

        "updated" => json!([
            {
                "updatedAt": {
                    "direction": "DESC"
                }
            }
        ]),

        "name" => json!([
            {
                "name": {
                    "direction": "ASC"
                }
            }
        ]),

        _ => json!([
            {
                "endorsements": {
                    "direction": "DESC"
                }
            }
        ]),
    }
}

fn browse_filter(search: &str, tags: &[String]) -> Value {
    let mut filters = vec![
        json!({
            "gameDomainName": [
                {
                    "value": GAME_DOMAIN,
                    "op": "EQUALS"
                }
            ]
        }),
        json!({
            "adultContent": [
                {
                    "value": false,
                    "op": "EQUALS"
                }
            ]
        }),
    ];

    let search = search.trim();

    if !search.is_empty() {
        filters.push(json!({
            "nameStemmed": [
                {
                    "value": search,
                    "op": "MATCHES"
                }
            ]
        }));
    }

    let selected_tags = tags
        .iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| {
            json!({
                "value": tag,
                "op": "EQUALS"
            })
        })
        .collect::<Vec<_>>();

    if !selected_tags.is_empty() {
        filters.push(json!({
            "tag": selected_tags
        }));
    }

    json!({
        "op": "AND",
        "filter": filters,
    })
}

const BROWSE_QUERY: &str = r#"
query EldenModManagerBrowse(
    $filter: ModsFilter,
$sort: [ModsSort!],
$count: Int!,
$offset: Int!
) {
mods(
    filter: $filter,
sort: $sort,
count: $count,
offset: $offset
) {
totalCount

nodes {
modId
name
summary
version
author

downloads
endorsements

createdAt
updatedAt

pictureUrl
adult
status

modCategory {
name
}
}
}
}
"#;

#[tauri::command]
pub async fn connect_nexus(app: AppHandle, api_key: String) -> Result<NexusAccountStatus, String> {
    let api_key = api_key.trim().to_string();

    if api_key.is_empty() {
        return Err("Enter a Nexus Mods API key.".to_string());
    }

    let client = client()?;

    let account: NexusValidateResponse = get_json(
        &client,
        &api_key,
        &format!("{REST_API_BASE}/users/validate.json"),
    )
    .await?;

    let saved = SavedNexusConfig {
        api_key,
        username: account.name.clone(),
        is_premium: account.is_premium,
    };

    write_config(&app, &saved)?;

    Ok(NexusAccountStatus {
        connected: true,
        username: Some(account.name),
        is_premium: Some(account.is_premium),
    })
}

#[tauri::command]
pub fn get_nexus_account(app: AppHandle) -> Result<NexusAccountStatus, String> {
    match read_config(&app)? {
        Some(config) => Ok(NexusAccountStatus {
            connected: true,
            username: Some(config.username),
            is_premium: Some(config.is_premium),
        }),

        None => Ok(NexusAccountStatus {
            connected: false,
            username: None,
            is_premium: None,
        }),
    }
}

#[tauri::command]
pub fn disconnect_nexus(app: AppHandle) -> Result<NexusAccountStatus, String> {
    let path = config_path(&app)?;

    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("Could not remove Nexus settings: {error}"))?;
    }

    Ok(NexusAccountStatus {
        connected: false,
        username: None,
        is_premium: None,
    })
}

const TAGS_QUERY: &str = r#"
query EldenModManagerTags($filter: ModsFilter) {
    mods(
        facets: {
            tag: []
        }
        filter: $filter
    ) {
        facetsData
    }
}
"#;

fn nexus_tag_is_allowed(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();

    if normalized.is_empty() {
        return false;
    }

    const BLOCKED: &[&str] = &[
        "adult", "nsfw", "sexual", "nudity", "nude", "erotic", "explicit",
    ];

    !BLOCKED.iter().any(|word| normalized.contains(word))
}

fn looks_like_metadata_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "tag"
            | "tags"
            | "value"
            | "values"
            | "name"
            | "label"
            | "count"
            | "total"
            | "id"
            | "key"
            | "field"
            | "facet"
            | "facets"
            | "data"
            | "items"
            | "buckets"
    )
}

fn collect_tag_facet_strings(value: &Value, inside_tag_facet: bool, output: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            /*
             * facetsData is a JSON scalar.
             *
             * Depending on how Nexus serializes it,
             * serde_json may receive either the actual
             * JSON object OR JSON encoded inside a string.
             */
            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                if parsed != *value {
                    collect_tag_facet_strings(&parsed, inside_tag_facet, output);

                    return;
                }
            }

            if inside_tag_facet {
                let trimmed = text.trim();

                if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("tag") {
                    output.push(trimmed.to_string());
                }
            }
        }

        Value::Array(values) => {
            for child in values {
                collect_tag_facet_strings(child, inside_tag_facet, output);
            }
        }

        Value::Object(object) => {
            /*
             * The most obvious shape:
             *
             * {
             *   "tag": [...]
             * }
             */
            for (key, child) in object {
                if key.eq_ignore_ascii_case("tag") || key.eq_ignore_ascii_case("tags") {
                    collect_tag_facet_strings(child, true, output);
                }
            }

            let object_declares_tag = ["name", "field", "facet", "key"].iter().any(|field| {
                object
                    .get(*field)
                    .and_then(Value::as_str)
                    .map(|name| name.eq_ignore_ascii_case("tag"))
                    .unwrap_or(false)
            });

            let now_inside = inside_tag_facet || object_declares_tag;

            if now_inside {
                /*
                 * Common bucket/value shapes.
                 */
                for field in ["value", "label", "displayName", "display_name"] {
                    if let Some(name) = object.get(field).and_then(Value::as_str) {
                        let name = name.trim();

                        if !name.is_empty() && !name.eq_ignore_ascii_case("tag") {
                            output.push(name.to_string());
                        }
                    }
                }

                /*
                 * Some facet implementations use:
                 *
                 * {
                 *   "Models/Meshes": 123,
                 *   "Gameplay": 456
                 * }
                 *
                 * In that case the object KEYS are
                 * the tag names.
                 */
                let all_values_are_counts = !object.is_empty()
                    && object
                        .values()
                        .all(|child| child.is_number() || child.is_null());

                if all_values_are_counts {
                    for key in object.keys() {
                        if !looks_like_metadata_key(key) {
                            output.push(key.to_string());
                        }
                    }
                }
            }

            /*
             * Continue recursively because Nexus may wrap
             * buckets under values/items/buckets/etc.
             */
            for (key, child) in object {
                let child_inside = now_inside
                    || key.eq_ignore_ascii_case("tag")
                    || key.eq_ignore_ascii_case("tags");

                collect_tag_facet_strings(child, child_inside, output);
            }
        }

        _ => {}
    }
}

#[tauri::command]
pub async fn get_nexus_tags(app: AppHandle) -> Result<Vec<NexusTag>, String> {
    let config =
        read_config(&app)?.ok_or_else(|| "Connect a Nexus Mods account first.".to_string())?;

    let client = client()?;

    let filter = browse_filter("", &[]);

    println!("NEXUS TAGS: requesting facetsData...");

    let data: NexusTagsData = graphql(
        &client,
        &config.api_key,
        TAGS_QUERY,
        json!({
            "filter": filter
        }),
    )
    .await?;

    println!(
        "NEXUS TAGS RAW facetsData: {}",
        serde_json::to_string_pretty(&data.mods.facets_data,)
            .unwrap_or_else(|_| { "<could not serialize>".to_string() },),
    );

    let mut names = Vec::<String>::new();

    collect_tag_facet_strings(&data.mods.facets_data, false, &mut names);

    names.retain(|name| nexus_tag_is_allowed(name));

    names.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));

    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    if names.is_empty() {
        return Err(format!(
            "Nexus returned facetsData, but no tags could be parsed. Raw facetsData: {}",
            shortened_body(&data.mods.facets_data.to_string(),),
        ));
    }

    let tags = names
        .into_iter()
        .enumerate()
        .map(|(index, name)| NexusTag {
            id: index as u64 + 1,

            name,
        })
        .collect::<Vec<_>>();

    println!("NEXUS TAGS: parsed {} tags.", tags.len(),);

    Ok(tags)
}

#[tauri::command]
pub async fn browse_nexus_mods(
    app: AppHandle,
    search: String,
    sort: String,
    page: u32,
    page_size: Option<u32>,
    tags: Option<Vec<String>>,
) -> Result<NexusBrowseResult, String> {
    let config =
        read_config(&app)?.ok_or_else(|| "Connect a Nexus Mods account first.".to_string())?;

    let client = client()?;

    let page = page.max(1);

    let page_size = page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);

    let offset_u64 = u64::from(page - 1) * u64::from(page_size);

    if offset_u64 > i32::MAX as u64 {
        return Err("Requested Nexus page is too large.".to_string());
    }

    let searching = !search.trim().is_empty();

    let variables = json!({
        "filter":
        browse_filter(
            &search,
            tags.as_deref().unwrap_or(&[]),
        ),

                          "sort":
                          browse_sort(
                              &sort,
                              searching,
                          ),

                          "count":
                          page_size,

                          "offset":
                          offset_u64,
    });

    let data: BrowseData = graphql(&client, &config.api_key, BROWSE_QUERY, variables).await?;

    let total_count = data.mods.total_count;

    let total_pages = if total_count == 0 {
        0
    } else {
        total_count.div_ceil(u64::from(page_size))
    };

    let mods = data
        .mods
        .nodes
        .into_iter()
        /*
         * We deliberately filter again locally
         * even though adultContent=false is also
         * sent to Nexus.
         *
         * Third-party API clients are responsible
         * for filtering age-restricted content.
         */
        .filter(|item| !item.adult)
        .filter(|item| {
            item.status
                .as_deref()
                .map(|status| status.eq_ignore_ascii_case("published"))
                .unwrap_or(true)
        })
        .map(|item| NexusBrowseMod {
            mod_id: item.mod_id,

            name: item.name,

            summary: item.summary,

            version: item.version,

            author: item.author,

            endorsements: item.endorsements,

            downloads: item.downloads,

            created_at: item.created_at,

            updated_at: item.updated_at,

            picture_url: item.picture_url,

            category: item
                .mod_category
                .map(|category| category.name)
                .filter(|name| !name.trim().is_empty()),

            nexus_url: format!("https://www.nexusmods.com/eldenring/mods/{}", item.mod_id,),
        })
        .collect::<Vec<_>>();

    Ok(NexusBrowseResult {
        mods,

        page,
        page_size,

        total_count,
        total_pages,

        has_previous: page > 1,

        has_next: total_pages > 0 && u64::from(page) < total_pages,
    })
}

#[tauri::command]
pub fn open_nexus_mod(mod_id: u64) -> Result<(), String> {
    let url = format!("https://www.nexusmods.com/eldenring/mods/{mod_id}");

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|error| format!("Could not open browser: {error}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|error| format!("Could not open browser: {error}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|error| format!("Could not open browser: {error}"))?;
    }

    Ok(())
}
