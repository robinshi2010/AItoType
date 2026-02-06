//! AitoType 技术验证项目
//! 
//! 本项目用于验证 AitoType 的核心技术可行性：
//! 1. 模拟键盘输入 (enigo)
//! 2. 麦克风录音 (cpal)
//! 3. 调用 Whisper API (reqwest)
//! 
//! 运行方式:
//! ```bash
//! cargo run -- keyboard   # 测试键盘输入
//! cargo run -- record     # 测试录音
//! cargo run -- api        # 测试 API 调用
//! cargo run -- full       # 完整链路测试
//! ```

mod keyboard;
mod audio;
mod api;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("AitoType 技术验证工具");
        println!("======================");
        println!("用法: cargo run -- <command>");
        println!("");
        println!("命令:");
        println!("  keyboard  - 测试模拟键盘输入 (需要辅助功能权限)");
        println!("  record    - 测试麦克风录音 (需要麦克风权限)");
        println!("  api       - 测试 Whisper API 调用");
        println!("  full      - 完整链路测试: 录音 -> API -> 输入");
        return;
    }
    
    match args[1].as_str() {
        "keyboard" => keyboard::test_keyboard_input(),
        "record" => audio::test_recording(),
        "api" => api::test_api(),
        "full" => full_test(),
        _ => println!("未知命令: {}", args[1]),
    }
}

fn full_test() {
    println!("🚀 完整链路测试");
    println!("================");
    println!("步骤: 录音 5 秒 -> 调用 API -> 模拟输入");
    println!("");
    
    // 1. 录音
    println!("📢 开始录音 5 秒...");
    let audio_path = audio::record_to_file(5);
    
    match audio_path {
        Ok(path) => {
            println!("✅ 录音完成: {}", path);
            
            // 2. 调用 API
            println!("🌐 调用 Whisper API...");
            match api::transcribe_file(&path) {
                Ok(text) => {
                    println!("✅ 识别结果: {}", text);
                    
                    // 3. 模拟输入
                    println!("⌨️  3 秒后模拟输入，请点击一个文本框...");
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    keyboard::type_text(&text);
                    println!("✅ 输入完成！");
                }
                Err(e) => println!("❌ API 调用失败: {}", e),
            }
        }
        Err(e) => println!("❌ 录音失败: {}", e),
    }
}
