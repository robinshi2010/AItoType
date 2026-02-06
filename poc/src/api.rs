//! Whisper API 调用模块
//! 
//! 支持 OpenAI、Groq 和阿里云的语音识别 API

use reqwest::blocking::multipart;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// 阿里云 OpenAI 兼容格式响应
#[derive(Debug, Deserialize)]
struct AliyunChatResponse {
    choices: Option<Vec<AliyunChoice>>,
    error: Option<AliyunError>,
}

#[derive(Debug, Deserialize)]
struct AliyunChoice {
    message: AliyunMessage,
}

#[derive(Debug, Deserialize)]
struct AliyunMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct AliyunError {
    message: String,
}

/// 测试 API 调用
pub fn test_api() {
    // 加载 .env 文件
    dotenv::dotenv().ok();
    
    println!("🌐 API 测试");
    println!("============");
    println!("");
    
    // 检查可用的 API Key
    let has_openrouter = env::var("OPENROUTER_API_KEY").is_ok();
    let has_aliyun = env::var("ALIYUN_API_KEY").is_ok();
    let has_groq = env::var("GROQ_API_KEY").is_ok();
    let has_openai = env::var("OPENAI_API_KEY").is_ok();
    
    println!("检测到的 API Key:");
    println!("  OpenRouter: {}", if has_openrouter { "✅" } else { "❌" });
    println!("  阿里云:     {}", if has_aliyun { "✅" } else { "❌" });
    println!("  Groq:       {}", if has_groq { "✅" } else { "❌" });
    println!("  OpenAI:     {}", if has_openai { "✅" } else { "❌" });
    println!("");
    
    if !has_openrouter && !has_aliyun && !has_groq && !has_openai {
        println!("❌ 未找到任何 API Key");
        println!("");
        println!("请在 .env 文件中设置:");
        println!("  OPENROUTER_API_KEY=your_key # OpenRouter");
        println!("  ALIYUN_API_KEY=your_key     # 阿里云百炼");
        println!("  GROQ_API_KEY=your_key       # Groq");
        println!("  OPENAI_API_KEY=your_key     # OpenAI");
        return;
    }
    
    // 查找测试音频文件
    let recent_recording = find_recent_recording();
    
    match recent_recording {
        Some(file) => {
            println!("📁 使用测试文件: {}", file);
            println!("🔄 调用 API 中...");
            println!("");
            
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
/// 
/// 自动选择可用的 API 提供商
/// OpenRouter 响应格式 (标准 OpenAI 格式)
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

/// 转录音频文件
/// 
/// 自动选择可用的 API 提供商
pub fn transcribe_file(file_path: &str) -> Result<String, String> {
    // 加载 .env 文件
    dotenv::dotenv().ok();
    
    // 优先使用 OpenRouter (Gemini 3 Flash Preview)
    if let Ok(api_key) = env::var("OPENROUTER_API_KEY") {
        println!("📡 使用 OpenRouter API (Gemini 3 Flash Preview)...");
        // 先尝试多模态直接调用，如果失败则回退（目前假设支持）
        return transcribe_with_openrouter(file_path, &api_key);
    }

    // 其次使用阿里云（国内访问快）- 使用 Qwen2-Audio 模型
    if let Ok(api_key) = env::var("ALIYUN_API_KEY") {
        println!("📡 使用阿里云 DashScope Qwen2-Audio API...");
        return transcribe_with_aliyun_qwen_audio(file_path, &api_key);
    }
    
    // 其次使用 Groq（速度快、便宜）
    if let Ok(api_key) = env::var("GROQ_API_KEY") {
        println!("📡 使用 Groq API...");
        return transcribe_with_openai_compatible(
            file_path, 
            &api_key,
            "https://api.groq.com/openai/v1/audio/transcriptions",
            "whisper-large-v3-turbo"
        );
    }
    
    // 最后使用 OpenAI
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        println!("📡 使用 OpenAI API...");
        return transcribe_with_openai_compatible(
            file_path,
            &api_key,
            "https://api.openai.com/v1/audio/transcriptions",
            "whisper-1"
        );
    }
    
    Err("未设置任何 API Key (OPENROUTER_API_KEY, ALIYUN_API_KEY, GROQ_API_KEY 或 OPENAI_API_KEY)".to_string())
}

/// 使用 OpenRouter 转录 (支持 Gemini 多模态)
fn transcribe_with_openrouter(file_path: &str, api_key: &str) -> Result<String, String> {
    // 读取音频文件
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开文件失败: {:?}", e))?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;
    
    // 使用 base64 编码音频
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let audio_base64 = STANDARD.encode(&buffer);
    
    // 根据 OpenRouter 官方文档，使用 input_audio 类型传递音频
    // data 是纯 base64 字符串（不带 data:audio/wav;base64, 前缀）
    // format 指定音频格式
    let request_body = serde_json::json!({
        "model": "google/gemini-3-flash-preview",
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
        // 添加 Referer 为了 OpenRouter 统计
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

/// 阿里云 DashScope 原生响应格式
#[derive(Debug, Deserialize)]
struct DashScopeResponse {
    output: Option<DashScopeOutput>,
    message: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DashScopeOutput {
    choices: Option<Vec<DashScopeChoice>>,
}

#[derive(Debug, Deserialize)]
struct DashScopeChoice {
    message: DashScopeMessage,
}

#[derive(Debug, Deserialize)]
struct DashScopeMessage {
    content: Option<serde_json::Value>, 
}

/// 使用阿里云 Qwen2-Audio 模型转录 (DashScope 原生 API)
/// 
/// 相比于 OpenAI 兼容接口，DashScope 原生接口对多模态支持更好
fn transcribe_with_aliyun_qwen_audio(file_path: &str, api_key: &str) -> Result<String, String> {
    // 读取音频文件
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开文件失败: {:?}", e))?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;
    
    // 使用 base64 编码音频
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let audio_base64 = STANDARD.encode(&buffer);
    
    // 构建 DashScope 原生请求体 - Multimodal Generation
    let request_body = serde_json::json!({
        "model": "qwen-audio-turbo",
        "input": {
            "messages": [
                {
                    "role": "user",
                    "content": [
                        { "audio": format!("data:audio/wav;base64,{}", audio_base64) },
                        { "text": "请将这段音频内容转写为文字，不要添加任何标点符号以外的解释性文字。" }
                    ]
                }
            ]
        },
        "parameters": {}
    });
    
    // 发送请求到阿里云 DashScope
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {:?}", e))?;
    
    let response = client
        .post("https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("请求失败: {:?}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().unwrap_or_default();
        return Err(format!("API 返回错误 {}: {}", status, error_text));
    }
    
    let result: DashScopeResponse = response.json()
        .map_err(|e| format!("解析响应失败: {:?}", e))?;
    
    if let Some(code) = result.code {
        return Err(format!("API 错误 [{}]: {}", code, result.message.unwrap_or_default()));
    }
    
    if let Some(output) = result.output {
        if let Some(choices) = output.choices {
            if let Some(choice) = choices.first() {
                if let Some(content_val) = &choice.message.content {
                   // content 可能是 string 或 list
                   if let Some(s) = content_val.as_str() {
                       return Ok(s.to_string());
                   }
                   if let Some(arr) = content_val.as_array() {
                       // 提取 list 中的 text
                       let mut text = String::new();
                       for item in arr {
                           if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                               text.push_str(t);
                           }
                       }
                       if !text.is_empty() {
                           return Ok(text);
                       }
                   }
                   return Ok(content_val.to_string());
                }
            }
        }
    }
    
    Err("未获取到转录结果".to_string())
}

/// 使用 OpenAI 兼容 API 转录 (OpenAI, Groq 等)
fn transcribe_with_openai_compatible(
    file_path: &str, 
    api_key: &str,
    base_url: &str,
    model: &str
) -> Result<String, String> {
    // 读取文件
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开文件失败: {:?}", e))?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;
    
    // 构建 multipart 请求
    let file_part = multipart::Part::bytes(buffer)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("创建请求失败: {:?}", e))?;
    
    let form = multipart::Form::new()
        .part("file", file_part)
        .text("model", model.to_string());
    
    // 发送请求
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(base_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .map_err(|e| format!("请求失败: {:?}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().unwrap_or_default();
        return Err(format!("API 返回错误 {}: {}", status, error_text));
    }
    
    let result: TranscriptionResponse = response.json()
        .map_err(|e| format!("解析响应失败: {:?}", e))?;
    
    Ok(result.text)
}
