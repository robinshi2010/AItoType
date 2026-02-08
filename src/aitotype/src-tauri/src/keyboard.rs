//! 键盘输入模块
//!
//! 使用 enigo 模拟键盘输入

use enigo::{Enigo, Keyboard, Settings};
use arboard::Clipboard;

/// 仅复制文字到系统剪贴板
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("初始化剪贴板失败: {:?}", e))?;

    clipboard.set_text(text)
        .map_err(|e| format!("复制失败: {:?}", e))?;

    Ok(())
}

/// 模拟键盘输入文字
pub fn type_text(text: &str) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("初始化 enigo 失败: {:?}", e))?;

    enigo.text(text)
        .map_err(|e| format!("输入失败: {:?}", e))?;

    Ok(())
}

/// 复制文字到剪贴板并粘贴
pub fn paste_text(text: &str) -> Result<(), String> {
    // 先复制到剪贴板
    copy_to_clipboard(text)?;

    // macOS 使用 AppleScript 模拟 Cmd+V 更加稳定，避免 enigo 在某些环境下的权限检测失效问题
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        use std::thread::sleep;
        use std::time::Duration;

        // 稍微等待一下确保剪贴板已经写入完成
        sleep(Duration::from_millis(100));

        let script = "tell application \"System Events\" to keystroke \"v\" using command down";
        match Command::new("osascript").arg("-e").arg(script).output() {
            Ok(output) if !output.status.success() => {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(format!("AppleScript 执行失败: {}", err));
            }
            Err(e) => return Err(format!("无法运行 osascript: {:?}", e)),
            _ => {}
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // 其他平台继续使用 enigo...
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| format!("初始化 enigo 失败: {:?}", e))?;
        
        use enigo::Key;
        enigo.key(Key::Control, enigo::Direction::Press).ok();
        enigo.key(Key::Unicode('v'), enigo::Direction::Click).ok();
        enigo.key(Key::Control, enigo::Direction::Release).ok();
    }

    Ok(())
}
