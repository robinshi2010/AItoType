//! 转写日志模块
//!
//! 记录每次转写与润色结果，按天写入 JSON 文件。

use crate::corrections::CorrectionHit;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// 单条转写日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeLogEntry {
    pub timestamp: String,
    pub stt_provider: String,
    pub stt_model: String,
    pub stt_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_correction_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_correction_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction_hits: Option<Vec<CorrectionHit>>,
    pub enhancement_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhancement_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhancement_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhancement_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhancement_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhancement_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhancement_duration_ms: Option<u64>,
    pub final_text: String,
}

/// 获取日志目录
fn log_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;

    app.path()
        .app_log_dir()
        .map_err(|e| format!("获取日志目录失败: {:?}", e))
}

/// 获取今日日志文件路径
fn today_log_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = log_dir(app)?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    Ok(dir.join(format!("transcribe_{}.json", today)))
}

/// 追加一条日志
///
/// 读取现有 JSON 数组后追加写回；失败不影响主流程。
pub fn append_log(app: &tauri::AppHandle, entry: TranscribeLogEntry) {
    let path = match today_log_path(app) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("append_log: {}", e);
            return;
        }
    };

    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("append_log: create_dir_all failed {:?}: {:?}", parent, e);
            return;
        }
    }

    let mut entries: Vec<TranscribeLogEntry> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default();
    entries.push(entry);

    match serde_json::to_string_pretty(&entries) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("append_log: write failed {:?}: {:?}", path, e);
            }
        }
        Err(e) => {
            eprintln!("append_log: serialize failed: {:?}", e);
        }
    }
}

/// 获取日志目录（给前端展示）
pub fn get_log_dir_path(app: &tauri::AppHandle) -> Result<String, String> {
    log_dir(app).map(|path| path.to_string_lossy().to_string())
}

/// 使用系统文件管理器打开日志目录
pub fn open_log_dir(app: &tauri::AppHandle) -> Result<(), String> {
    let dir = log_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建日志目录失败: {:?}", e))?;

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut command = Command::new("open");
        command.arg(&dir);
        command
    };

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut command = Command::new("explorer");
        command.arg(&dir);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = {
        let mut command = Command::new("xdg-open");
        command.arg(&dir);
        command
    };

    let status = cmd
        .status()
        .map_err(|e| format!("打开日志目录失败: {:?}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("打开日志目录失败，退出码: {:?}", status.code()))
    }
}
