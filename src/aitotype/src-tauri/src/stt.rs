//! 语音转文字 (STT) 模块
//!
//! 支持 OpenRouter 与 SiliconFlow 两个 Provider

use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

pub const PROVIDER_OPENROUTER: &str = "openrouter";
pub const PROVIDER_SILICONFLOW: &str = "siliconflow";

/// OpenRouter 默认模型
pub const DEFAULT_OPENROUTER_MODEL: &str = "google/gemini-3-flash-preview";
/// SiliconFlow 默认模型（免费 ASR）
pub const DEFAULT_SILICONFLOW_MODEL: &str = "TeleAI/TeleSpeechASR";

/// OpenRouter 默认 API URL
pub const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
/// SiliconFlow 默认 API URL
pub const DEFAULT_SILICONFLOW_BASE_URL: &str = "https://api.siliconflow.cn/v1";
pub const CONNECTION_ERROR_AUTH_FAILED: &str = "auth_failed";
pub const CONNECTION_ERROR_NETWORK_FAILED: &str = "network_failed";
pub const CONNECTION_ERROR_MODEL_NOT_FOUND: &str = "model_not_found";
pub const CONNECTION_ERROR_QUOTA_EXCEEDED: &str = "quota_exceeded";
pub const CONNECTION_ERROR_UNKNOWN: &str = "unknown";

fn default_provider() -> String {
    PROVIDER_OPENROUTER.to_string()
}

fn default_record_mode() -> String {
    "toggle".to_string()
}




pub fn normalize_provider(provider: &str) -> String {
    let normalized = provider.trim().to_lowercase();
    if normalized == PROVIDER_SILICONFLOW {
        PROVIDER_SILICONFLOW.to_string()
    } else {
        PROVIDER_OPENROUTER.to_string()
    }
}

pub fn default_base_url_for_provider(provider: &str) -> &'static str {
    if provider == PROVIDER_SILICONFLOW {
        DEFAULT_SILICONFLOW_BASE_URL
    } else {
        DEFAULT_OPENROUTER_BASE_URL
    }
}

pub fn default_model_for_provider(provider: &str) -> &'static str {
    if provider == PROVIDER_SILICONFLOW {
        DEFAULT_SILICONFLOW_MODEL
    } else {
        DEFAULT_OPENROUTER_MODEL
    }
}


pub fn env_key_for_provider(provider: &str) -> &'static str {
    if provider == PROVIDER_SILICONFLOW {
        "SILICONFLOW_API_KEY"
    } else {
        "OPENROUTER_API_KEY"
    }
}

/// STT 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Provider: openrouter / siliconflow
    #[serde(default = "default_provider")]
    pub provider: String,
    /// API 基础 URL
    pub base_url: String,
    /// API Key
    pub api_key: String,
    /// 模型名称
    pub model: String,
    /// 是否自动写入
    #[serde(default)]
    pub auto_write: bool,
    /// 录音模式: toggle / hold
    #[serde(default = "default_record_mode")]
    pub record_mode: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            base_url: DEFAULT_OPENROUTER_BASE_URL.to_string(),
            api_key: String::new(),
            model: DEFAULT_OPENROUTER_MODEL.to_string(),
            auto_write: false,
            record_mode: default_record_mode(),
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

fn read_audio_file(file_path: &str) -> Result<Vec<u8>, String> {
    let mut file = File::open(file_path).map_err(|e| format!("打开文件失败: {:?}", e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件失败: {:?}", e))?;
    Ok(buffer)
}

fn normalize_base_url(base_url: &str, provider: &str) -> String {
    let source = if base_url.trim().is_empty() {
        default_base_url_for_provider(provider)
    } else {
        base_url.trim()
    };
    source.trim_end_matches('/').to_string()
}

fn resolve_api_key(config: &SttConfig, provider: &str) -> Result<String, String> {
    if !config.api_key.trim().is_empty() {
        return Ok(config.api_key.trim().to_string());
    }

    let env_key = env_key_for_provider(provider);
    match std::env::var(env_key) {
        Ok(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(format!(
            "API Key 不能为空（可通过环境变量 {} 提供）",
            env_key
        )),
    }
}



fn classify_http_error(status: StatusCode, body: &str) -> String {
    let lower = body.to_lowercase();

    let error_type = if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        CONNECTION_ERROR_AUTH_FAILED
    } else if status == StatusCode::PAYMENT_REQUIRED
        || status == StatusCode::TOO_MANY_REQUESTS
        || lower.contains("insufficient")
        || lower.contains("quota")
        || lower.contains("credit")
        || lower.contains("rate limit")
        || lower.contains("余额不足")
        || lower.contains("额度不足")
    {
        CONNECTION_ERROR_QUOTA_EXCEEDED
    } else if status == StatusCode::NOT_FOUND
        || lower.contains("model not found")
        || lower.contains("unknown model")
        || lower.contains("invalid model")
        || lower.contains("does not exist")
        || lower.contains("模型不存在")
    {
        CONNECTION_ERROR_MODEL_NOT_FOUND
    } else {
        CONNECTION_ERROR_UNKNOWN
    };

    match error_type {
        CONNECTION_ERROR_AUTH_FAILED => format!(
            "{}|鉴权失败，请检查 API Key 是否正确",
            CONNECTION_ERROR_AUTH_FAILED
        ),
        CONNECTION_ERROR_QUOTA_EXCEEDED => format!(
            "{}|额度不足或请求过于频繁，请检查账户余额与速率限制",
            CONNECTION_ERROR_QUOTA_EXCEEDED
        ),
        CONNECTION_ERROR_MODEL_NOT_FOUND => format!(
            "{}|模型不存在，请检查模型名称是否正确",
            CONNECTION_ERROR_MODEL_NOT_FOUND
        ),
        _ => format!(
            "{}|API 返回错误 {}: {}",
            CONNECTION_ERROR_UNKNOWN,
            status.as_u16(),
            body
        ),
    }
}

fn classify_network_error(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        return format!(
            "{}|网络超时（10 秒），请检查网络连接",
            CONNECTION_ERROR_NETWORK_FAILED
        );
    }
    if err.is_connect() {
        return format!(
            "{}|网络连接失败，请检查网络或代理设置",
            CONNECTION_ERROR_NETWORK_FAILED
        );
    }
    format!(
        "{}|请求失败: {}",
        CONNECTION_ERROR_NETWORK_FAILED,
        err
    )
}

pub async fn test_connection(config: &SttConfig) -> Result<(), String> {
    let provider = normalize_provider(&config.provider);
    if provider == PROVIDER_SILICONFLOW {
        test_siliconflow_connection(config).await
    } else {
        test_openrouter_connection(config).await
    }
}


async fn test_openrouter_connection(config: &SttConfig) -> Result<(), String> {
    let provider = normalize_provider(&config.provider);
    let api_key = resolve_api_key(config, &provider)?;
    let model = if config.model.trim().is_empty() {
        default_model_for_provider(&provider)
    } else {
        config.model.trim()
    };

    let request_body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "max_tokens": 1,
        "temperature": 0
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建客户端失败: {:?}", e))?;

    let url = format!(
        "{}/chat/completions",
        normalize_base_url(&config.base_url, &provider)
    );
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://github.com/aitotype")
        .header("X-Title", "AItoType")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| classify_network_error(&e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(classify_http_error(status, &error_text));
    }

    Ok(())
}

async fn test_siliconflow_connection(config: &SttConfig) -> Result<(), String> {
    let provider = PROVIDER_SILICONFLOW.to_string();
    let api_key = resolve_api_key(config, &provider)?;
    let model = if config.model.trim().is_empty() {
        default_model_for_provider(&provider).to_string()
    } else {
        config.model.trim().to_string()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建客户端失败: {:?}", e))?;

    let url = format!("{}/models", normalize_base_url(&config.base_url, &provider));
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| classify_network_error(&e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(classify_http_error(status, &error_text));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("{}|解析 models 响应失败: {:?}", CONNECTION_ERROR_UNKNOWN, e))?;

    let model_exists = value
        .get("data")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().any(|item| {
                item.get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| id.eq_ignore_ascii_case(&model))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !model_exists {
        return Err(format!(
            "{}|模型不存在，请检查模型名称是否正确",
            CONNECTION_ERROR_MODEL_NOT_FOUND
        ));
    }

    Ok(())
}

/// 转录音频文件
pub async fn transcribe(file_path: &str, config: &SttConfig) -> Result<String, String> {
    let provider = normalize_provider(&config.provider);
    if provider == PROVIDER_SILICONFLOW {
        transcribe_with_siliconflow(file_path, config).await
    } else {
        transcribe_with_openrouter(file_path, config).await
    }
}

async fn transcribe_with_openrouter(file_path: &str, config: &SttConfig) -> Result<String, String> {
    let provider = normalize_provider(&config.provider);
    let model = if config.model.trim().is_empty() {
        default_model_for_provider(&provider)
    } else {
        config.model.trim()
    };
    let api_key = resolve_api_key(config, &provider)?;
    let audio_bytes = read_audio_file(file_path)?;
    let audio_base64 = STANDARD.encode(&audio_bytes);

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

    let url = format!(
        "{}/chat/completions",
        normalize_base_url(&config.base_url, &provider)
    );
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
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
        if lower.contains("user location is not supported")
            || lower.contains("location is not supported")
        {
            return Err("当前账号路由到 Google AI Studio 且受地区限制。已尝试绕开该 Provider 但仍失败，请在 OpenRouter 控制台为该模型切换可用 Provider（如 Vertex 路由）或使用可用地区网络。".to_string());
        }
        return Err(format!("API 返回错误 {}: {}", status, error_text));
    }

    let result: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {:?}", e))?;

    if let Some(error) = result.error {
        return Err(format!("API 错误: {}", error.message));
    }

    result
        .choices
        .and_then(|c| c.first().cloned())
        .and_then(|c| c.message.content)
        .ok_or_else(|| "未获取到转录结果".to_string())
}

async fn transcribe_with_siliconflow(
    file_path: &str,
    config: &SttConfig,
) -> Result<String, String> {
    let provider = PROVIDER_SILICONFLOW.to_string();
    let model = if config.model.trim().is_empty() {
        DEFAULT_SILICONFLOW_MODEL
    } else {
        config.model.trim()
    };
    let api_key = resolve_api_key(config, &provider)?;
    let audio_bytes = read_audio_file(file_path)?;
    let url = format!(
        "{}/audio/transcriptions",
        normalize_base_url(&config.base_url, &provider)
    );

    let file_part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("构建音频请求体失败: {:?}", e))?;

    let form = reqwest::multipart::Form::new()
        .text("model", model.to_string())
        .part("file", file_part);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建客户端失败: {:?}", e))?;

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("请求失败: {:?}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!(
            "SiliconFlow API 返回错误 {}: {}",
            status, error_text
        ));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {:?}", e))?;

    value
        .get("text")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("未获取到转录结果: {}", value))
}
