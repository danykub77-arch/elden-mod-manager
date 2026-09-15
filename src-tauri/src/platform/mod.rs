use serde::Serialize;

#[derive(Serialize)]
pub struct PlatformInfo {
    os: String,
    display_name: String,
    uses_proton: bool,
    supported: bool,
}

#[tauri::command]
pub fn get_platform_info() -> PlatformInfo {
    #[cfg(target_os = "linux")]
    {
        return PlatformInfo {
            os: "linux".to_string(),
            display_name: "Linux".to_string(),
            uses_proton: true,
            supported: true,
        };
    }

    #[cfg(target_os = "windows")]
    {
        return PlatformInfo {
            os: "windows".to_string(),
            display_name: "Windows".to_string(),
            uses_proton: false,
            supported: true,
        };
    }

    #[cfg(target_os = "macos")]
    {
        return PlatformInfo {
            os: "macos".to_string(),
            display_name: "macOS".to_string(),
            uses_proton: false,

            // The manager can run here, but Elden Ring
            // launching requires another compatibility
            // solution.
            supported: false,
        };
    }

    #[allow(unreachable_code)]
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        display_name: "Unknown".to_string(),
        uses_proton: false,
        supported: false,
    }
}
