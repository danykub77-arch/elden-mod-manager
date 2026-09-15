use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModContentType {
    Assets,
    NativeDll,
    Config,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityLevel {
    Universal,
    CompatibilityRequired,
    ModEngine2Required,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModContent {
    pub content_type: ModContentType,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ModSource {
    Nexus {
        mod_id: u64,
        file_id: u64,
        version: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMod {
    pub id: String,
    pub name: String,
    pub version: Option<String>,

    #[serde(default)]
    pub source: Option<ModSource>,

    pub enabled: bool,

    pub compatibility: CompatibilityLevel,

    pub contents: Vec<ModContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedProfile {
    pub profile_id: String,

    pub mods: Vec<InstalledMod>,
}

impl UnifiedProfile {
    pub fn empty(profile_id: &str) -> Self {
        Self {
            profile_id: profile_id.to_string(),
            mods: Vec::new(),
        }
    }

    pub fn requires_modengine2(&self) -> bool {
        self.mods.iter().any(|installed_mod| {
            installed_mod.enabled
                && matches!(
                    installed_mod.compatibility,
                    CompatibilityLevel::ModEngine2Required
                )
        })
    }
}
