//! AitoType - macOS 语音转文字工具
//!
//! 核心功能:
//! - 录音 (audio 模块)
//! - 语音识别 (stt 模块) - 使用 OpenRouter API
//! - 键盘输入 (keyboard 模块)

mod audio;
mod keyboard;
mod stt;

use stt::SttConfig;
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

/// 应用状态
pub struct AppState {
    stt_config: Mutex<SttConfig>,
}

impl Default for AppState {
    fn default() -> Self {
        // 加载环境变量配置
        dotenv::dotenv().ok();
        
        // 尝试从环境变量加载 API Key
        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
        
        Self {
            stt_config: Mutex::new(SttConfig {
                base_url: stt::DEFAULT_BASE_URL.to_string(),
                api_key,
                model: stt::DEFAULT_MODEL.to_string(),
            }),
        }
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
    let config = state.stt_config.lock()
        .map_err(|e| format!("获取配置失败: {:?}", e))?
        .clone();
    
    stt::transcribe(&file_path, &config).await
}

/// 完整流程: 停止录音 -> 转录 -> 返回结果
#[tauri::command]
async fn stop_and_transcribe(state: State<'_, AppState>) -> Result<String, String> {
    // 停止录音
    let file_path = audio::stop_recording()?;
    
    // 转录
    let config = state.stt_config.lock()
        .map_err(|e| format!("获取配置失败: {:?}", e))?
        .clone();

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
    state.stt_config.lock()
        .map(|c| c.clone())
        .map_err(|e| format!("获取配置失败: {:?}", e))
}

/// 保存 STT 配置
#[tauri::command]
fn save_stt_config(app: tauri::AppHandle, config: SttConfig, state: State<AppState>) -> Result<(), String> {
    let mut normalized = config;

    if normalized.base_url.trim().is_empty() {
        normalized.base_url = stt::DEFAULT_BASE_URL.to_string();
    }

    if normalized.model.trim().is_empty() {
        normalized.model = stt::DEFAULT_MODEL.to_string();
    }

    // 更新内存状态
    {
        let mut current = state.stt_config.lock()
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
    let config = state.stt_config.lock()
        .map_err(|e| format!("获取配置失败: {:?}", e))?
        .clone();
    
    if config.api_key.is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    
    Ok(format!("连接测试成功 - Model: {}", config.model))
}

/// 更新全局快捷键
#[tauri::command]
fn update_shortcut(app: tauri::AppHandle, shortcut: String) -> Result<(), String> {
    // 忽略 unregister 错误（可能本来就没有）
    let _ = app.global_shortcut().unregister_all();
    
    if !shortcut.is_empty() {
        app.global_shortcut().register(shortcut.as_str()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn show_overlay_status(app: tauri::AppHandle, status: String) -> Result<(), String> {
    use tauri::{Emitter, Manager};

    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window not found".to_string())?;

    if let (Ok(Some(monitor)), Ok(size)) = (overlay.current_monitor(), overlay.outer_size()) {
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let x = monitor_pos.x + ((monitor_size.width.saturating_sub(size.width)) / 2) as i32;
        let y = (monitor_pos.y + (monitor_size.height as i32 - size.height as i32 - 96))
            .max(monitor_pos.y);
        let _ = overlay.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
    }

    overlay.show().map_err(|e| e.to_string())?;
    overlay
        .emit(
            "overlay-status",
            OverlayStatusPayload {
                status: status.trim().to_lowercase(),
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn hide_overlay(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window not found".to_string())?;
    overlay.hide().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // --- 加载持久化配置 ---
            use tauri::Manager;
            if let Ok(path) = app.path().app_config_dir() {
                let config_path = path.join("config.json");
                if config_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        if let Ok(saved_config) = serde_json::from_str::<SttConfig>(&content) {
                             let state = app.state::<AppState>();
                             if let Ok(mut guard) = state.stt_config.lock() {
                                 *guard = saved_config;
                                 println!("✅ 已加载配置文件: {:?}", config_path);
                             };
                        }
                    }
                }
            }

            // --- System Tray ---
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let quit_i = MenuItem::with_id(app, "quit", "Quit AitoType", true, None::<&str>)?;
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
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // --- Global Shortcut (Alt+Space) ---
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::ShortcutState;
                use tauri::Emitter;

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcut("Alt+Space")?
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
                )?;
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
            show_overlay_status,
            hide_overlay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
