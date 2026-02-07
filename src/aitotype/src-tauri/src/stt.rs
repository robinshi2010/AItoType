//! 语音转文字 (STT) 模块
//!
//! 使用 OpenRouter API (支持 Gemini 等多模态模型)

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

/// 默认模型 - 按需求固定使用 Gemini 3 Flash
pub const DEFAULT_MODEL: &str = "google/gemini-3-flash-preview";

/// 默认 API URL
pub const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// STT 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
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
            base_url: DEFAULT_BASE_URL.to_string(),
            api_key: String::new(),
            model: DEFAULT_MODEL.to_string(),
        }
    }
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

/// 转录音频文件
pub async fn transcribe(file_path: &str, config: &SttConfig) -> Result<String, String> {
    let mut file = File::open(file_path)
        .map_err(|e| format!("打开文件失败: {:?}", e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;

    let audio_base64 = STANDARD.encode(&buffer);

    transcribe_with_model(&audio_base64, config, &config.model).await
}

async fn transcribe_with_model(audio_base64: &str, config: &SttConfig, model: &str) -> Result<String, String> {
    let request_body = serde_json::json!({
        "model": model,
        "provider": {
            "allow_fallbacks": true,
            "ignore": ["Google AI Studio"]
        },
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
        .header("X-Title", "AItoType")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {:?}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        let lower = error_text.to_lowercase();
        if lower.contains("user location is not supported") || lower.contains("location is not supported") {
            return Err("当前账号路由到 Google AI Studio 且受地区限制。已尝试绕开该 Provider 但仍失败，请在 OpenRouter 控制台为该模型切换可用 Provider（如 Vertex 路由）或使用可用地区网络。".to_string());
        }
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
