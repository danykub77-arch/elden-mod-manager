mod conflicts;
mod elden_ring;
mod engines;
mod game_settings;
mod game_settings_mapper;
mod load_order;
mod mod_import;
mod modded_launch;
mod modpacks;
mod nexus;
mod nexus_download;
mod paths;
mod platform;
mod profiles;
mod runtime;
mod save_game_settings;

use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;

use conflicts::get_profile_conflicts;

use load_order::{move_mod, set_mod_order};

use elden_ring::{detect_elden_ring, launch_elden_ring_vanilla};

use engines::get_engine_overview;

use save_game_settings::{
    get_save_game_settings, restore_save_game_settings_backup, save_save_game_settings,
};

use engines::me3::{check_me3_status, install_me3};

use game_settings::{
    get_game_graphics_settings, restore_game_graphics_backup, save_game_graphics_settings,
};

use mod_import::{
    analyze_mod_archive, import_mod_variant, is_nexus_mod_installed, list_profile_mods,
    pick_mod_archive, record_nexus_mod_install, replace_nexus_mod_variant, set_mod_enabled,
    uninstall_mod,
};

use modded_launch::launch_elden_ring_modded;

use modpacks::{export_modpack, import_modpack, inspect_modpack};

use nexus::{
    browse_nexus_mods, connect_nexus, disconnect_nexus, get_nexus_account, get_nexus_tags,
    open_nexus_mod,
};

use nexus_download::{
    cancel_nexus_download, check_nexus_mod_updates, download_nexus_mod_file, get_nexus_mod_files,
    open_nexus_download_authorization,
};

use platform::get_platform_info;

use profiles::{
    create_profile, delete_profile, get_profile_engine_decision, get_profiles, select_profile,
    set_profile_engine,
};

use game_settings_mapper::{
    capture_game_settings_baseline, clear_game_settings_baseline, compare_game_settings_baseline,
    get_game_settings_baseline_status,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    /*
     * Must be before deep-link on desktop.
     *
     * Opening nxm:// can launch another copy of
     * the executable. Single-instance redirects
     * that launch into the existing application.
     */
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();

                let _ = window.unminimize();

                let _ = window.set_focus();
            }
        }));
    }

    builder = builder.plugin(tauri_plugin_deep_link::init());

    builder
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            paths::ensure_app_directories().map_err(std::io::Error::other)?;

            profiles::initialize_profiles().map_err(std::io::Error::other)?;

            engines::me3::ensure_engine_directory().map_err(std::io::Error::other)?;

            engines::modengine2::ensure_engine_directory().map_err(std::io::Error::other)?;

            /*
             * Development builds aren't installed
             * through a package yet, so explicitly
             * register nxm:// on Linux.
             */
            #[cfg(any(target_os = "linux", all(debug_assertions, target_os = "windows")))]
            {
                app.deep_link()
                    .register_all()
                    .map_err(std::io::Error::other)?;

                println!("Registered development deep links.");
            }

            /*
             * Handle an NXM URL that STARTED
             * the application.
             *
             * Do not print the URL here because
             * real NXM links contain a temporary
             * authorization key.
             */
            if let Some(urls) = app
                .deep_link()
                .get_current()
                .map_err(std::io::Error::other)?
            {
                for url in urls {
                    let handle = app.handle().clone();

                    let raw_url = url.to_string();

                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = nexus_download::handle_nxm_url(handle, raw_url).await {
                            eprintln!("NXM download failed: {error}");
                        }
                    });
                }
            }

            /*
             * Handle NXM links while the manager
             * is already running.
             */
            let handle_for_deep_links = app.handle().clone();

            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    let handle = handle_for_deep_links.clone();

                    let raw_url = url.to_string();

                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = nexus_download::handle_nxm_url(handle, raw_url).await {
                            eprintln!("NXM download failed: {error}");
                        }
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_elden_ring,
            launch_elden_ring_vanilla,
            launch_elden_ring_modded,
            get_profiles,
            create_profile,
            select_profile,
            delete_profile,
            set_profile_engine,
            get_profile_engine_decision,
            connect_nexus,
            get_nexus_account,
            disconnect_nexus,
            browse_nexus_mods,
            get_nexus_tags,
            open_nexus_mod,
            get_nexus_mod_files,
            check_nexus_mod_updates,
            download_nexus_mod_file,
            open_nexus_download_authorization,
            cancel_nexus_download,
            get_platform_info,
            get_engine_overview,
            check_me3_status,
            install_me3,
            pick_mod_archive,
            analyze_mod_archive,
            import_mod_variant,
            replace_nexus_mod_variant,
            list_profile_mods,
            record_nexus_mod_install,
            is_nexus_mod_installed,
            set_mod_enabled,
            uninstall_mod,
            get_profile_conflicts,
            move_mod,
            set_mod_order,
            export_modpack,
            inspect_modpack,
            import_modpack,
            get_game_graphics_settings,
            save_game_graphics_settings,
            restore_game_graphics_backup,
            get_save_game_settings,
            save_save_game_settings,
            restore_save_game_settings_backup,
            capture_game_settings_baseline,
            clear_game_settings_baseline,
            compare_game_settings_baseline,
            get_game_settings_baseline_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
