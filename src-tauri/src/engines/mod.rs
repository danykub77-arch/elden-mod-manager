pub mod me3;
pub mod modengine2;

use crate::runtime::UnifiedProfile;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EngineKind {
    Auto,
    Me3,
    ModEngine2,
}

impl Default for EngineKind {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub id: String,

    pub name: String,

    pub description: String,

    pub installed: bool,

    pub preferred: bool,

    pub engine_path: String,

    pub executable_path: Option<String>,

    pub installed_version: Option<String>,

    pub latest_version: Option<String>,

    pub update_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineOverview {
    pub preferred_engine: String,

    pub me3: EngineStatus,

    pub modengine2: EngineStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineDecision {
    pub requested: EngineKind,

    pub selected: EngineKind,

    pub reason: String,
}

pub fn choose_engine(preference: EngineKind, profile: &UnifiedProfile) -> EngineDecision {
    match preference {
        EngineKind::Me3 => EngineDecision {
            requested: EngineKind::Me3,

            selected: EngineKind::Me3,

            reason: "Profile is explicitly configured to use me3.".to_string(),
        },

        EngineKind::ModEngine2 => EngineDecision {
            requested: EngineKind::ModEngine2,

            selected: EngineKind::ModEngine2,

            reason: "Profile is explicitly configured to use Mod Engine 2.".to_string(),
        },

        EngineKind::Auto => {
            if profile.requires_modengine2() {
                EngineDecision {
                    requested: EngineKind::Auto,

                    selected: EngineKind::ModEngine2,

                    reason: "An enabled mod requires the Mod Engine 2 compatibility runtime."
                        .to_string(),
                }
            } else {
                EngineDecision {
                    requested: EngineKind::Auto,

                    selected: EngineKind::Me3,

                    reason: "All enabled mods are compatible with the preferred me3 runtime."
                        .to_string(),
                }
            }
        }
    }
}

#[tauri::command]
pub async fn get_engine_overview() -> Result<EngineOverview, String> {
    Ok(EngineOverview {
        preferred_engine: "me3".to_string(),

        me3: me3::status().await?,

        modengine2: modengine2::status()?,
    })
}
