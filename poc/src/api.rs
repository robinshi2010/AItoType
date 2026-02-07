//! OpenRouter API 调用模块
//! 
//! 使用 OpenRouter 的 Gemini 模型进行语音识别

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::Read;

/// 默认模型
const DEFAULT_MODEL: &str = "google/gemini-3-flash-preview";

/// OpenRouter 响应格式
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    error: Option<OpenRouterError>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterError {
    message: String,
}

/// 测试 API 调用
pub fn test_api() {
    // 加载 .env 文件
    dotenv::dotenv().ok();
    
    println!("🌐 API 测试");
    println!("============");
    println!();
    
    // 检查 API Key
    let has_openrouter = env::var("OPENROUTER_API_KEY").is_ok();
    
    println!("检测到的 API Key:");
    println!("  OpenRouter: {}", if has_openrouter { "✅" } else { "❌" });
    println!();
    
    if !has_openrouter {
        println!("❌ 未找到 API Key");
        println!();
        println!("请在 .env 文件中设置:");
        println!("  OPENROUTER_API_KEY=your_key");
        return;
    }
    
    // 查找测试音频文件
    let recent_recording = find_recent_recording();
    
    match recent_recording {
        Some(file) => {
            println!("📁 使用测试文件: {}", file);
            println!("🔄 调用 API 中...");
            println!();
            
            match transcribe_file(&file) {
                Ok(text) => {
                    println!("✅ 识别成功！");
                    println!("📝 结果: {}", text);
                }
                Err(e) => {
                    println!("❌ 调用失败: {}", e);
                }
            }
        }
        None => {
            println!("💡 没有测试音频文件，请先运行录音测试:");
            println!("   cargo run -- record");
        }
    }
}

/// 查找最近的录音文件
fn find_recent_recording() -> Option<String> {
    let entries = std::fs::read_dir("/tmp").ok()?;
    
    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("aitotype_recording_")
        })
        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .map(|e| e.path().to_string_lossy().to_string())
}

/// 转录音频文件
pub fn transcribe_file(file_path: &str) -> Result<String, String> {
    // 加载 .env 文件
    dotenv::dotenv().ok();
    
    let api_key = env::var("OPENROUTER_API_KEY")
        .map_err(|_| "未设置 OPENROUTER_API_KEY".to_string())?;
    
    println!("📡 使用 OpenRouter API ({})...", DEFAULT_MODEL);
    transcribe_with_openrouter(file_path, &api_key)
}

/// 使用 OpenRouter 转录
fn transcribe_with_openrouter(file_path: &str, api_key: &str) -> Result<String, String> {
    // 读取音频文件
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开文件失败: {:?}", e))?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;
    
    // 使用 base64 编码音频
    let audio_base64 = STANDARD.encode(&buffer);
    
    // 构建请求体
    let request_body = serde_json::json!({
        "model": DEFAULT_MODEL,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "请精准转录这段音频内容。保持原义，不要翻译，如果是中文就直接输出中文。只输出转录文字，不要输出任何解释。"
                    },
                    {
                        "type": "input_audio",
                        "input_audio": {
                            "data": audio_base64,
                            "format": "wav"
                        }
                    }
                ]
            }
        ]
    });

    // 发送请求
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建客户端失败: {:?}", e))?;
        
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://github.com/aitotype") 
        .header("X-Title", "AitoType")
        .json(&request_body) 
        .send()
        .map_err(|e| format!("请求失败: {:?}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().unwrap_or_default();
        return Err(format!("API 返回错误 {}: {}", status, error_text));
    }
    
    let result: OpenRouterResponse = response.json()
        .map_err(|e| format!("解析响应失败: {:?}", e))?;
        
    if let Some(error) = result.error {
        return Err(format!("OpenRouter 错误: {}", error.message));
    }
    
    if let Some(choice) = result.choices.first() {
        if let Some(content) = &choice.message.content {
            return Ok(content.clone());
        }
    }
    
    Err("OpenRouter 未返回内容".to_string())
}
