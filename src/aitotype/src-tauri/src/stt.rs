//! 语音转文字 (STT) 模块
//!
//! 支持多个 API 提供商: OpenRouter, Groq, OpenAI, 阿里云

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

/// STT 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// 提供商类型
    pub provider: String,
    /// API 基础 URL
    pub base_url: String,
    /// API Key
    pub api_key: String,
    /// 模型名称
    pub model: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            provider: "groq".to_string(),
            base_url: "https://api.groq.com/openai/v1".to_string(),
            api_key: String::new(),
            model: "whisper-large-v3-turbo".to_string(),
        }
    }
}

/// 根据 provider 返回默认 base URL。
pub fn default_base_url(provider: &str) -> &'static str {
    match provider {
        "groq" => "https://api.groq.com/openai/v1",
        "openai" => "https://api.openai.com/v1",
        "openrouter" => "https://openrouter.ai/api/v1",
        "aliyun" => "https://dashscope.aliyuncs.com/api/v1",
        "custom" => "",
        _ => "",
    }
}

/// 根据 provider 返回默认模型。
pub fn default_model(provider: &str) -> &'static str {
    match provider {
        "groq" => "whisper-large-v3-turbo",
        "openai" => "whisper-1",
        "openrouter" => "google/gemini-3-flash-preview",
        "aliyun" => "qwen-audio-turbo",
        "custom" => "whisper-1",
        _ => "whisper-large-v3-turbo",
    }
}

/// OpenAI 兼容格式响应
#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// OpenRouter/LLM 响应格式
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Option<Vec<ChatChoice>>,
    error: Option<ApiError>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

/// 阿里云 DashScope 响应格式
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

/// 转录音频文件
/// 根据配置选择不同的 API 提供商
pub async fn transcribe(file_path: &str, config: &SttConfig) -> Result<String, String> {
    match config.provider.as_str() {
        "openrouter" => transcribe_openrouter(file_path, config).await,
        "aliyun" => transcribe_aliyun(file_path, config).await,
        "groq" | "openai" | "custom" => transcribe_openai_compatible(file_path, config).await,
        _ => Err(format!("不支持的提供商: {}", config.provider)),
    }
}

/// 使用 OpenRouter API (支持 Gemini 等多模态模型)
async fn transcribe_openrouter(file_path: &str, config: &SttConfig) -> Result<String, String> {
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开文件失败: {:?}", e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;

    let audio_base64 = STANDARD.encode(&buffer);

    let request_body = serde_json::json!({
        "model": config.model,
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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建客户端失败: {:?}", e))?;

    let url = format!("{}/chat/completions", config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://github.com/aitotype")
        .header("X-Title", "AitoType")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {:?}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API 返回错误 {}: {}", status, error_text));
    }

    let result: ChatCompletionResponse = response.json().await
        .map_err(|e| format!("解析响应失败: {:?}", e))?;

    if let Some(error) = result.error {
        return Err(format!("API 错误: {}", error.message));
    }

    result.choices
        .and_then(|c| c.first().cloned())
        .and_then(|c| c.message.content)
        .ok_or_else(|| "未获取到转录结果".to_string())
}

/// 使用阿里云 DashScope API
async fn transcribe_aliyun(file_path: &str, config: &SttConfig) -> Result<String, String> {
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开文件失败: {:?}", e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;

    let audio_base64 = STANDARD.encode(&buffer);

    let request_body = serde_json::json!({
        "model": config.model,
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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建客户端失败: {:?}", e))?;

    let response = client
        .post("https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {:?}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API 返回错误 {}: {}", status, error_text));
    }

    let result: DashScopeResponse = response.json().await
        .map_err(|e| format!("解析响应失败: {:?}", e))?;

    if let Some(code) = result.code {
        return Err(format!("API 错误 [{}]: {}", code, result.message.unwrap_or_default()));
    }

    if let Some(output) = result.output {
        if let Some(choices) = output.choices {
            if let Some(choice) = choices.first() {
                if let Some(content_val) = &choice.message.content {
                    if let Some(s) = content_val.as_str() {
                        return Ok(s.to_string());
                    }
                    if let Some(arr) = content_val.as_array() {
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

/// 使用 OpenAI 兼容 API (OpenAI, Groq, 自定义)
async fn transcribe_openai_compatible(file_path: &str, config: &SttConfig) -> Result<String, String> {
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开文件失败: {:?}", e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;

    let file_part = multipart::Part::bytes(buffer)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("创建请求失败: {:?}", e))?;

    let form = multipart::Form::new()
        .part("file", file_part)
        .text("model", config.model.clone());

    let client = reqwest::Client::new();
    let url = format!("{}/audio/transcriptions", config.base_url);
    
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("请求失败: {:?}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API 返回错误 {}: {}", status, error_text));
    }

    let result: TranscriptionResponse = response.json().await
        .map_err(|e| format!("解析响应失败: {:?}", e))?;

    Ok(result.text)
}
