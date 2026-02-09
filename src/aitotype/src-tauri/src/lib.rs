//! AItoType - macOS 语音转文字工具
//!
//! 核心功能:
//! - 录音 (audio 模块)
//! - 语音识别 (stt 模块) - 支持 OpenRouter / SiliconFlow
//! - 键盘输入 (keyboard 模块)

mod audio;
mod keyboard;
mod stt;

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use stt::SttConfig;
use tauri::State;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

fn load_env_files() {
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| {
        let _ = dotenv::dotenv();

        if let Ok(current_dir) = std::env::current_dir() {
            for ancestor in current_dir.ancestors() {
                let poc_env = ancestor.join("poc/.env");
                if poc_env.exists() {
                    let _ = dotenv::from_path(poc_env);
                    break;
                }
            }
        }
    });
}

fn normalize_stt_config(config: SttConfig) -> SttConfig {
    let mut normalized = config;
    normalized.provider = stt::normalize_provider(&normalized.provider);

    if normalized.base_url.trim().is_empty() {
        normalized.base_url = stt::default_base_url_for_provider(&normalized.provider).to_string();
    } else {
        normalized.base_url = normalized.base_url.trim().trim_end_matches('/').to_string();
    }

    if normalized.model.trim().is_empty() {
        normalized.model = stt::default_model_for_provider(&normalized.provider).to_string();
    } else {
        normalized.model = normalized.model.trim().to_string();
    }

    normalized.api_key = normalized.api_key.trim().to_string();
    normalized
}

fn has_resolved_api_key(config: &SttConfig) -> bool {
    if !config.api_key.trim().is_empty() {
        return true;
    }

    let env_key = stt::env_key_for_provider(&config.provider);
    matches!(std::env::var(env_key), Ok(value) if !value.trim().is_empty())
}

/// 应用状态
pub struct AppState {
    stt_config: Mutex<SttConfig>,
    shortcut_plugin_ready: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        load_env_files();

        let mut config = SttConfig::default();
        let env_key = stt::env_key_for_provider(&config.provider);
        if let Ok(value) = std::env::var(env_key) {
            config.api_key = value.trim().to_string();
        }

        Self {
            stt_config: Mutex::new(normalize_stt_config(config)),
            shortcut_plugin_ready: AtomicBool::new(false),
        }
    }
}

fn default_global_shortcut() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Ctrl+Shift+Space"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "Alt+Space"
    }
}

#[derive(Clone, Serialize)]
struct ToggleRecordingEventPayload {
    background: bool,
}

#[derive(Clone, Serialize)]
struct OverlayStatusPayload {
    status: String,
}

fn show_or_create_main_window(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .ok_or_else(|| "main window config not found".to_string())?;

    let win = tauri::WebviewWindowBuilder::from_config(app, window_config)
        .map_err(|e| format!("recreate main window builder failed: {}", e))?
        .build()
        .map_err(|e| format!("recreate main window failed: {}", e))?;

    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}

/// 开始录音
#[tauri::command]
fn start_recording() -> Result<(), String> {
    audio::start_recording()
}

/// 停止录音并返回音频文件路径
#[tauri::command]
fn stop_recording() -> Result<String, String> {
    audio::stop_recording()
}

/// 获取录音状态
#[tauri::command]
fn is_recording() -> bool {
    audio::is_recording()
}

/// 获取音频电平 (0.0 - 1.0)
#[tauri::command]
fn get_audio_level() -> f32 {
    audio::get_audio_level()
}

/// 转录音频文件
#[tauri::command]
async fn transcribe_audio(file_path: String, state: State<'_, AppState>) -> Result<String, String> {
    let config = state
        .stt_config
        .lock()
        .map_err(|e| format!("获取配置失败: {:?}", e))?
        .clone();
    let config = normalize_stt_config(config);

    stt::transcribe(&file_path, &config).await
}

/// 完整流程: 停止录音 -> 转录 -> 返回结果
#[tauri::command]
async fn stop_and_transcribe(state: State<'_, AppState>) -> Result<String, String> {
    // 停止录音
    let file_path = audio::stop_recording()?;

    // 转录
    let config = state
        .stt_config
        .lock()
        .map_err(|e| format!("获取配置失败: {:?}", e))?
        .clone();
    let config = normalize_stt_config(config);

    let transcribe_result = stt::transcribe(&file_path, &config).await;

    // 清理临时录音文件，避免在 /tmp 持续堆积。
    if let Err(e) = std::fs::remove_file(&file_path) {
        eprintln!("清理临时录音文件失败 {}: {:?}", file_path, e);
    }

    transcribe_result
}

/// 模拟键盘输入
#[tauri::command]
fn type_text(text: String) -> Result<(), String> {
    keyboard::type_text(&text)
}

/// 粘贴文字
#[tauri::command]
fn paste_text(text: String) -> Result<(), String> {
    keyboard::paste_text(&text)
}

/// 复制文字到剪贴板
#[tauri::command]
fn copy_to_clipboard(text: String) -> Result<(), String> {
    keyboard::copy_to_clipboard(&text)
}

/// 获取当前 STT 配置
#[tauri::command]
fn get_stt_config(state: State<AppState>) -> Result<SttConfig, String> {
    state
        .stt_config
        .lock()
        .map(|c| normalize_stt_config(c.clone()))
        .map_err(|e| format!("获取配置失败: {:?}", e))
}

/// 保存 STT 配置
#[tauri::command]
fn save_stt_config(
    app: tauri::AppHandle,
    config: SttConfig,
    state: State<AppState>,
) -> Result<(), String> {
    let normalized = normalize_stt_config(config);

    // 更新内存状态
    {
        let mut current = state
            .stt_config
            .lock()
            .map_err(|e| format!("获取配置失败: {:?}", e))?;
        *current = normalized.clone();
    }

    // 持久化到磁盘
    use tauri::Manager;
    if let Ok(path) = app.path().app_config_dir() {
        let config_path = path.join("config.json");
        // 确保目录存在
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Ok(json) = serde_json::to_string_pretty(&normalized) {
            if let Err(e) = std::fs::write(&config_path, json) {
                eprintln!("保存配置失败: {:?}", e);
                return Err(format!("保存配置失败: {}", e));
            }
        }
    }

    Ok(())
}

/// 测试 API 连接
#[tauri::command]
async fn test_connection(state: State<'_, AppState>) -> Result<String, String> {
    let config = state
        .stt_config
        .lock()
        .map_err(|e| format!("获取配置失败: {:?}", e))?
        .clone();
    let config = normalize_stt_config(config);

    if !has_resolved_api_key(&config) {
        let env_key = stt::env_key_for_provider(&config.provider);
        return Err(format!(
            "API Key 不能为空（可通过环境变量 {} 提供）",
            env_key
        ));
    }

    Ok(format!(
        "连接测试成功 - Provider: {}, Model: {}",
        config.provider, config.model
    ))
}

/// 更新全局快捷键
#[tauri::command]
fn update_shortcut(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    shortcut: String,
) -> Result<(), String> {
    if !state.shortcut_plugin_ready.load(Ordering::Acquire) {
        return Err("global shortcut plugin is not ready".to_string());
    }

    let shortcut = shortcut.trim().to_string();

    // 忽略 unregister 错误（可能本来就没有）
    let _ = app.global_shortcut().unregister_all();

    if !shortcut.is_empty() {
        app.global_shortcut()
            .register(shortcut.as_str())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn is_shortcut_ready(state: State<'_, AppState>) -> bool {
    state.shortcut_plugin_ready.load(Ordering::Acquire)
}

#[tauri::command]
fn show_overlay_status(app: tauri::AppHandle, status: String) -> Result<(), String> {
    use tauri::{Emitter, Manager};

    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window not found".to_string())?;

    const OVERLAY_LOGICAL_WIDTH: f64 = 334.0;
    const OVERLAY_LOGICAL_HEIGHT: f64 = 86.0;
    const OVERLAY_BOTTOM_MARGIN: i32 = 280;

    let _ = overlay.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        OVERLAY_LOGICAL_WIDTH,
        OVERLAY_LOGICAL_HEIGHT,
    )));

    let monitor = app
        .get_webview_window("main")
        .and_then(|main| main.current_monitor().ok().flatten())
        .or_else(|| overlay.current_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let (overlay_width, overlay_height) = overlay
            .outer_size()
            .map(|size| (size.width, size.height))
            .unwrap_or_else(|_| {
                let scale = monitor.scale_factor();
                (
                    (OVERLAY_LOGICAL_WIDTH * scale).round() as u32,
                    (OVERLAY_LOGICAL_HEIGHT * scale).round() as u32,
                )
            });
        let x = monitor_pos.x + ((monitor_size.width.saturating_sub(overlay_width)) / 2) as i32;
        let y = (monitor_pos.y
            + (monitor_size.height as i32 - overlay_height as i32 - OVERLAY_BOTTOM_MARGIN))
            .max(monitor_pos.y);
        let _ = overlay.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
            x, y,
        )));
    } else {
        // 回退方案：如果当前拿不到显示器信息，至少保证窗口回到可见区域。
        let _ = overlay.center();
    }

    let _ = overlay.set_always_on_top(true);

    if let Err(e) = overlay.show() {
        eprintln!("show overlay failed: {:?}", e);
        return Ok(());
    }

    if let Err(e) = overlay.emit(
        "overlay-status",
        OverlayStatusPayload {
            status: status.trim().to_lowercase(),
        },
    ) {
        eprintln!("emit overlay-status failed: {:?}", e);
    }

    Ok(())
}

#[tauri::command]
fn check_accessibility_permissions() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        unsafe { AXIsProcessTrusted() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[tauri::command]
fn request_accessibility_permissions() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
            fn CFDictionaryCreate(
                allocator: *const std::ffi::c_void,
                keys: *const *const std::ffi::c_void,
                values: *const *const std::ffi::c_void,
                numValues: isize,
                keyCallBacks: *const std::ffi::c_void,
                valueCallBacks: *const std::ffi::c_void,
            ) -> *const std::ffi::c_void;
            static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
            static kCFBooleanTrue: *const std::ffi::c_void;
            static kCFTypeDictionaryKeyCallBacks: *const std::ffi::c_void;
            static kCFTypeDictionaryValueCallBacks: *const std::ffi::c_void;
            fn CFRelease(obj: *const std::ffi::c_void);
        }

        unsafe {
            let keys = [kAXTrustedCheckOptionPrompt];
            let values = [kCFBooleanTrue];
            let options = CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                kCFTypeDictionaryKeyCallBacks,
                kCFTypeDictionaryValueCallBacks,
            );
            let result = AXIsProcessTrustedWithOptions(options);
            CFRelease(options);
            result
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[tauri::command]
fn open_accessibility_settings() {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

#[tauri::command]
fn hide_overlay(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window not found".to_string())?;

    if let Err(e) = overlay.hide() {
        eprintln!("hide overlay failed: {:?}", e);
    }
    Ok(())
}

#[tauri::command]
fn hide_main_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            use tauri::Manager;
            if window.label() != "main" {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                if let Some(overlay) = window.app_handle().get_webview_window("overlay") {
                    let _ = overlay.hide();
                }
            }
        })
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            load_env_files();
            // --- 加载持久化配置 ---
            use tauri::Manager;
            if let Ok(path) = app.path().app_config_dir() {
                let config_path = path.join("config.json");
                if config_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        if let Ok(saved_config) = serde_json::from_str::<SttConfig>(&content) {
                            let state = app.state::<AppState>();
                            if let Ok(mut guard) = state.stt_config.lock() {
                                *guard = normalize_stt_config(saved_config);
                                println!("✅ 已加载配置文件: {:?}", config_path);
                            };
                        }
                    }
                }
            }

            // --- System Tray ---
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let quit_i = MenuItem::with_id(app, "quit", "Quit AItoType", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(false)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => app.exit(0),
                        "show" => {
                            if let Err(e) = show_or_create_main_window(app) {
                                eprintln!("show main window failed: {}", e);
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // 预热 overlay 窗口，避免首次通过快捷键唤起时出现初始化卡顿。
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ =
                    overlay.set_size(tauri::Size::Logical(tauri::LogicalSize::new(334.0, 86.0)));
                let _ = overlay.show();
                let _ = overlay.hide();
            }

            // --- Global Shortcut ---
            #[cfg(desktop)]
            {
                use tauri::Emitter;
                use tauri_plugin_global_shortcut::ShortcutState;

                let plugin_result = app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(|app, _shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                let background = app
                                    .get_webview_window("main")
                                    .and_then(|w| w.is_focused().ok())
                                    .map(|is_focused| !is_focused)
                                    .unwrap_or(true);
                                let _ = app.emit(
                                    "toggle-recording-event",
                                    ToggleRecordingEventPayload { background },
                                );
                            }
                        })
                        .build(),
                );

                match plugin_result {
                    Ok(_) => {
                        let state = app.state::<AppState>();
                        state.shortcut_plugin_ready.store(true, Ordering::Release);
                        if let Err(e) = app.global_shortcut().register(default_global_shortcut()) {
                            eprintln!(
                                "register default global shortcut ({}) failed: {}",
                                default_global_shortcut(),
                                e
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("init global shortcut plugin failed: {}", e);
                    }
                }
            }

            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            is_recording,
            get_audio_level,
            transcribe_audio,
            stop_and_transcribe,
            type_text,
            paste_text,
            copy_to_clipboard,
            get_stt_config,
            save_stt_config,
            test_connection,
            update_shortcut,
            is_shortcut_ready,
            show_overlay_status,
            hide_overlay,
            check_accessibility_permissions,
            request_accessibility_permissions,
            open_accessibility_settings,
            hide_main_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::RunEvent::Reopen { .. }) {
                if let Err(e) = show_or_create_main_window(app) {
                    eprintln!("reopen main window failed: {}", e);
                }
            }
        });
}
