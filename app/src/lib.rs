// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use tauri::{Builder, Manager};
use tauri_plugin_autostart::MacosLauncher;

pub mod state;
pub mod utils;

pub mod brain;
pub mod commands;
pub mod constants;
pub mod services;

use commands::audio::play_ui_sound;
use commands::auth::{
    cache_avatar, cancel_google_auth, get_api_key, get_user_data, logout, start_google_auth,
};
use commands::capture::{spawn_capture, spawn_capture_to_input};
use commands::chat::{
    append_chat_message, create_chat, delete_chat, get_image_path, get_imgbb_url, get_ocr_data,
    get_ocr_frame, init_ocr_frame, list_chats, load_chat, overwrite_chat_messages,
    read_attachment_text, resolve_attachment_path, reveal_in_file_manager, save_imgbb_url,
    save_ocr_data, store_file_from_path, store_image_bytes, store_image_from_path,
    update_chat_metadata,
};
use commands::clipboard::{
    copy_image_from_path_to_clipboard, copy_image_to_clipboard, read_clipboard_image,
    read_clipboard_text,
};
use commands::constants::get_app_constants;
use commands::image::{
    copy_image_to_path, get_initial_image, process_image_path, read_image_file,
    upload_image_to_imgbb,
};
use commands::models::{download_ocr_model, get_model_path, list_downloaded_models};
use commands::ocr::{cancel_ocr_job, ocr_image};
use commands::profile::{
    delete_profile, get_active_profile, get_active_profile_id, get_profile_count, has_profiles,
    list_profiles, set_active_profile,
};
use commands::security::{check_file_exists, encrypt_and_save, has_agreed_flag, set_agreed_flag};
use commands::speech::SpeechState;
use commands::window::{
    close_window, get_always_on_top, maximize_window, minimize_window, open_external_url,
    reload_window, set_always_on_top, set_background_color, show_window,
};
use services::image::process_and_store_image;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    std::env::set_var("GDK_BACKEND", "x11");

    Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let wants_background = args.iter().any(|a| a == "--background" || a == "-b");
            if !wants_background {
                services::tray::show_window(app);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .manage(services::audio::UiSoundPlayer::new())
        .manage(SpeechState::default())
        .invoke_handler(tauri::generate_handler![
            // Image processing
            process_image_path,
            read_image_file,
            get_initial_image,
            upload_image_to_imgbb,
            // Clipboard
            read_clipboard_image,
            read_clipboard_text,
            copy_image_to_clipboard,
            copy_image_from_path_to_clipboard,
            // Security
            encrypt_and_save,
            check_file_exists,
            set_agreed_flag,
            has_agreed_flag,
            // Auth
            get_api_key,
            start_google_auth,
            cancel_google_auth,
            logout,
            get_user_data,
            cache_avatar,
            // Gemini
            commands::gemini::stream_gemini_chat_v2,
            commands::gemini::generate_chat_title,
            commands::gemini::cancel_gemini_request,
            // Window
            open_external_url,
            set_background_color,
            minimize_window,
            maximize_window,
            close_window,
            reload_window,
            set_always_on_top,
            get_always_on_top,
            show_window,
            play_ui_sound,
            // Constants
            get_app_constants,
            // OCR
            ocr_image,
            cancel_ocr_job,
            // Model Management
            download_ocr_model,
            commands::models::cancel_download_ocr_model,
            list_downloaded_models,
            get_model_path,
            // CAS Image Storage
            store_image_bytes,
            store_image_from_path,
            store_file_from_path,
            get_image_path,
            resolve_attachment_path,
            read_attachment_text,
            reveal_in_file_manager,
            // Chat Storage
            create_chat,
            load_chat,
            list_chats,
            delete_chat,
            update_chat_metadata,
            append_chat_message,
            overwrite_chat_messages,
            commands::chat::validate_text_file,
            // OCR Storage
            save_ocr_data,
            get_ocr_data,
            get_ocr_frame,
            init_ocr_frame,
            // ImgBB Storage
            save_imgbb_url,
            get_imgbb_url,
            // Native File IO
            copy_image_to_path,
            // Profile Management
            get_active_profile,
            get_active_profile_id,
            set_active_profile,
            list_profiles,
            delete_profile,
            has_profiles,
            get_profile_count,
            // Theme
            commands::theme::get_system_theme,
            // Speech
            commands::speech::start_stt,
            commands::speech::stop_stt,
            // Capture
            spawn_capture,
            spawn_capture_to_input,
        ])
        .manage(services::models::ModelManager::new().expect("Failed to init ModelManager"))
        .setup(move |app| {
            let handle = app.handle().clone();
            let model_manager = app.state::<services::models::ModelManager>();
            model_manager.start_monitor();

            let args: Vec<String> = std::env::args().collect();
            let has_cli_image = args.iter().skip(1).find(|arg| !arg.starts_with("-"));

            if let Some(path) = has_cli_image.clone() {
                println!("CLI Image argument detected: {}", path);
                let state = handle.state::<AppState>();
                let _ = process_and_store_image(path.clone(), &state);
            }

            #[cfg(target_os = "linux")]
            {
                let config_dir = crate::utils::get_app_config_dir(&handle);
                let marker_file = config_dir.join(".shortcut_installed");

                if !marker_file.exists() {
                    log::info!(
                        "First run on Linux detected: attempting to install global shortcut"
                    );
                    if let Ok(exe) = std::env::current_exe() {
                        let bin = exe.to_string_lossy();
                        match sys_global_shortcut::install_linux_shortcut(
                            &bin,
                            "SUPER+SHIFT+a",
                            crate::constants::APP_NAME,
                        ) {
                            Ok(_) => {
                                log::info!("Successfully installed Linux global shortcut");
                                if let Err(e) = std::fs::write(&marker_file, "") {
                                    log::error!("Failed to create shortcut marker file: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to install Linux global shortcut: {}", e);
                            }
                        }
                    }
                }
            }

            let (base_w, base_h) = (1030.0, 690.0);

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let config_dir = crate::utils::get_app_config_dir(&handle);
            let prefs_file = config_dir.join("preferences.json");
            if !prefs_file.exists() {
                log::info!("First run detected: creating default preferences.json");
                let default_prefs = serde_json::json!({
                    "model": crate::constants::DEFAULT_MODEL,
                    "theme": crate::constants::DEFAULT_THEME,
                    "prompt": crate::constants::DEFAULT_PROMPT,
                    "ocrEnabled": true,
                    "autoExpandOCR": true,
                    "captureType": crate::constants::DEFAULT_CAPTURE_TYPE,
                    "ocrLanguage": crate::constants::DEFAULT_OCR_LANGUAGE,
                    "activeAccount": crate::constants::DEFAULT_ACTIVE_ACCOUNT
                });

                if let Err(e) = std::fs::create_dir_all(&config_dir) {
                    log::error!("Failed to create config dir: {}", e);
                } else if let Err(e) = std::fs::write(
                    &prefs_file,
                    serde_json::to_string_pretty(&default_prefs).unwrap(),
                ) {
                    log::error!("Failed to create default preferences.json: {}", e);
                }
            }

            services::window::spawn_app_window(
                &handle,
                "main",
                "index.html",
                base_w,
                base_h,
                "",
                false,
            )
            .expect("Failed to spawn main window");

            services::tray::setup_tray(&handle).expect("Failed to setup tray icon");

            let shortcut_handle = handle.clone();
            let _shortcut = sys_global_shortcut::ShortcutHandle::register(
                sys_global_shortcut::ShortcutConfig {
                    linux_trigger: "SUPER+SHIFT+a".into(),
                    linux_description: format!("{} Capture", crate::constants::APP_NAME).into(),
                    windows_modifiers: 0x0008 | 0x0004, // MOD_WIN | MOD_SHIFT
                    windows_vk: 0x41,                   // VK_A
                    macos_modifiers: 0x0100 | 0x0200,   // cmdKey | shiftKey
                    macos_keycode: 0x00,                // kVK_ANSI_A
                },
                move || services::tray::capture_screen_with_source(&shortcut_handle, "hotkey"),
            );

            match &_shortcut {
                Ok(_) => log::info!("Global shortcut registered successfully"),
                Err(e) => log::warn!("Global shortcut registration failed (non-fatal): {}", e),
            }

            if let Some(window) = handle.get_webview_window("main") {
                let win = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let _ = win.hide();
                        api.prevent_close();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
