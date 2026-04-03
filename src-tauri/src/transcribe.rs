use reqwest::header::{AUTHORIZATION, HeaderMap};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

pub async fn transcribe(
    api_url: &str,
    api_key: &str,
    model_id: &str,
    wav_bytes: Vec<u8>,
    vocabulary: &[String],
) -> Result<String, String> {
    let url = format!("{}/audio/transcriptions", api_url);

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {}", api_key).parse().map_err(|e| format!("Invalid API key header: {}", e))?,
    );

    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", model_id.to_string());

    if !vocabulary.is_empty() {
        let prompt = vocabulary.join(", ");
        form = form.text("initial_prompt", prompt);
    }

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let result: TranscriptionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(result.text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transcription_response() {
        let json = r#"{"text": " Hello, world! "}"#;
        let resp: TranscriptionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text.trim(), "Hello, world!");
    }

    #[test]
    fn test_parse_empty_text_response() {
        let json = r#"{"text": ""}"#;
        let resp: TranscriptionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text, "");
    }

    #[test]
    fn test_parse_response_with_extra_fields() {
        let json = r#"{"text": "hello", "language": "en", "duration": 1.5}"#;
        let resp: TranscriptionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.text, "hello");
    }
}
