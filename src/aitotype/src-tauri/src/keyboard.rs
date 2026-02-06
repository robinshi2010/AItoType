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

    // 模拟 Cmd+V 粘贴
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("初始化 enigo 失败: {:?}", e))?;

    // macOS 使用 Meta (Command) 键
    use enigo::Key;
    
    enigo.key(Key::Meta, enigo::Direction::Press)
        .map_err(|e| format!("按键失败: {:?}", e))?;
    
    enigo.key(Key::Unicode('v'), enigo::Direction::Click)
        .map_err(|e| format!("按键失败: {:?}", e))?;
    
    enigo.key(Key::Meta, enigo::Direction::Release)
        .map_err(|e| format!("按键失败: {:?}", e))?;

    Ok(())
}
