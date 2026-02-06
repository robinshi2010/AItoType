# OpenRouter Gemini Integration Guide

*Validated on: 2026-02-06*
*Model: google/gemini-3-flash-preview*

This document details how to successfully integrate Google's Gemini models (specifically Gemini 3 Flash Preview) via OpenRouter for audio transcription and multimodal tasks. This approach was validated in the `poc` (Proof of Concept) phase.

## 1. Overview

Direct integration with Google's Vertex AI can be complex due to authentication and region restrictions. OpenRouter provides a unified, OpenAI-compatible API interface that simplifies access to Gemini models while supporting their native multimodal capabilities (audio, images, etc.).

## 2. API Configuration

- **Base URL**: `https://openrouter.ai/api/v1/chat/completions`
- **Method**: `POST`
- **Headers**:
    - `Authorization`: `Bearer YOUR_OPENROUTER_API_KEY`
    - `Content-Type`: `application/json`
    - `HTTP-Referer`: `https://your-site.com` (Optional, required for rankings)
    - `X-Title`: `Your App Name` (Optional)

## 3. Audio Input Format (Crucial)

To send audio to Gemini via OpenRouter, you **MUST** use the `input_audio` content type. The standard OpenAI `image_url` hack does not work reliably for audio on this model.

### JSON Structure

```json
{
  "model": "google/gemini-3-flash-preview",
  "messages": [
    {
      "role": "user",
      "content": [
        {
          "type": "text",
          "text": "Please transcribe this audio content accurately. If it is Chinese, output Chinese. Output only the transcription."
        },
        {
          "type": "input_audio",
          "input_audio": {
            "data": "BASE64_ENCODED_STRING", 
            "format": "wav"
          }
        }
      ]
    }
  ]
}
```

### Key Requirements

1.  **Type**: Must be `"type": "input_audio"`.
2.  **Data**: The `data` field must contain the **raw Base64 string** of the audio file.
    *   **Do NOT** include the data URI prefix (e.g., `data:audio/wav;base64,`). Just the raw encoded string.
3.  **Format**: The `format` field must specify the audio format (e.g., `"wav"`, `"mp3"`).

## 4. Rust Implementation Example

Here is the validated Rust implementation using `reqwest` and `serde_json`:

```rust
use std::fs::File;
use std::io::Read;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::json;

fn transcribe_with_openrouter(file_path: &str, api_key: &str) -> Result<String, String> {
    // 1. Read audio file
    let mut file = File::open(file_path)
        .map_err(|e| format!("Failed to open file: {:?}", e))?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read file: {:?}", e))?;
    
    // 2. Encode to Base64 (Standard)
    let audio_base64 = STANDARD.encode(&buffer);
    
    // 3. Construct Request Body with input_audio
    let request_body = json!({
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
                            "data": audio_base64, // Raw base64 string
                            "format": "wav"
                        }
                    }
                ]
            }
        ]
    });
    
    // 4. Send Request
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build client: {:?}", e))?;

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://github.com/aitotype")
        .header("X-Title", "AitoType")
        .json(&request_body) 
        .send()
        .map_err(|e| format!("Request failed: {:?}", e))?;
    
    // ... Handle response parsing ...
}
```

## 5. Model Performance Notes

-   **Model**: `google/gemini-3-flash-preview`
-   **Audio Understanding**: Excellent. Verified with Chinese speech ("测试测试，12345，上山打老虎").
-   **Latency**: Very low (Flash tier).
-   **Cost**: Significantly cheaper than OpenAI Whisper API on a per-minute basis (token-based pricing).

## 6. Troubleshooting

-   **404 Not Found**: If the model ID is incorrect or the model is temporarily unavailable on OpenRouter. Check OpenRouter's model list.
-   **400 Bad Request**: Usually indicates incorrect JSON structure. Ensure `input_audio` is used and `data` does not have the URI prefix.
-   **Hallucinations (Outputting text unrelated to audio)**: Usually means the model failed to process the audio attachment properly (e.g. wrong format or encoding) and is replying only to the text prompt.
