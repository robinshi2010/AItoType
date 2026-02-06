//! 键盘模拟模块
//! 
//! 使用 enigo crate 模拟键盘输入

use enigo::{Enigo, Keyboard, Settings};
use std::{thread, time::Duration};

/// 测试键盘输入功能
pub fn test_keyboard_input() {
    println!("⌨️  键盘输入测试");
    println!("================");
    println!("");
    println!("⚠️  首次运行需要在 系统偏好设置 -> 隐私与安全性 -> 辅助功能 中授权");
    println!("");
    println!("3 秒后将模拟输入文字，请点击一个文本输入框...");
    
    thread::sleep(Duration::from_secs(3));
    
    match Enigo::new(&Settings::default()) {
        Ok(mut enigo) => {
            // 测试英文
            if let Err(e) = enigo.text("Hello from AitoType! ") {
                println!("❌ 英文输入失败: {:?}", e);
                return;
            }
            
            thread::sleep(Duration::from_millis(100));
            
            // 测试中文
            if let Err(e) = enigo.text("你好世界！这是一段中文测试。") {
                println!("❌ 中文输入失败: {:?}", e);
                return;
            }
            
            println!("");
            println!("✅ 键盘输入测试成功！");
        }
        Err(e) => {
            println!("❌ 初始化 Enigo 失败: {:?}", e);
            println!("");
            println!("💡 请检查是否已授予辅助功能权限:");
            println!("   系统偏好设置 -> 隐私与安全性 -> 辅助功能 -> 添加 Terminal/终端");
        }
    }
}

/// 输入指定文字
pub fn type_text(text: &str) {
    match Enigo::new(&Settings::default()) {
        Ok(mut enigo) => {
            if let Err(e) = enigo.text(text) {
                println!("❌ 输入失败: {:?}", e);
            }
        }
        Err(e) => {
            println!("❌ 初始化失败: {:?}", e);
        }
    }
}
