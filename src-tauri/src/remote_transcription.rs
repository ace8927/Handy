//! Minimal OpenAI-compatible STT client. The caller already has Handy's
//! 16-kHz mono float32 microphone buffer; no extra recorder or server needed.
use crate::settings::AppSettings;
use anyhow::{bail, Context, Result};
use reqwest::blocking::multipart::{Form, Part};
use serde::Deserialize;
use std::io::Cursor;
use std::time::Duration;

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Accept the server's OpenAI base URL (`http://host:port/v1`) or the
/// full audio/transcriptions URL, as well as a server root URL.
pub(crate) fn validate_server_url(base: &str) -> Result<String> {
    let base = base.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(base).context("Invalid Remote STT server URL")?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        bail!("Remote STT URL must be an HTTP(S) server URL without credentials, query or fragment");
    }
    if base.ends_with("/audio/transcriptions") {
        Ok(base.to_owned())
    } else if base.ends_with("/v1") {
        Ok(format!("{base}/audio/transcriptions"))
    } else {
        Ok(format!("{base}/v1/audio/transcriptions"))
    }
}

fn encode_wav(pcm: &[f32]) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for &s in pcm {
            let s = if s.is_finite() { s.clamp(-1.0, 1.0) } else { 0.0 };
            writer.write_sample((s * i16::MAX as f32) as i16)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

pub fn transcribe(settings: &AppSettings, pcm: &[f32]) -> Result<String> {
    if settings.remote_stt_model.trim().is_empty() {
        bail!("Remote STT model is not configured");
    }
    let endpoint = validate_server_url(&settings.remote_stt_url)?;
    let wav = encode_wav(pcm)?;
    let audio = Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;
    let mut form = Form::new()
        .text("model", settings.remote_stt_model.trim().to_owned())
        .text("response_format", "json")
        .part("file", audio);
    if settings.selected_language != "auto" && !settings.selected_language.is_empty() {
        form = form.text("language", settings.selected_language.clone());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let mut req = client.post(endpoint).multipart(form);
    if let Some(api_key) = settings.remote_stt_api_keys.get("server") {
        if !api_key.trim().is_empty() {
            req = req.bearer_auth(api_key);
        }
    }
    let response = req.send().context("Remote STT request failed")?;
    let status = response.status();
    if !status.is_success() {
        // Avoid logging arbitrary server response bodies (may contain tokens).
        bail!("Remote STT server returned HTTP {status}");
    }
    let result: TranscriptionResponse = response
        .json()
        .context("Remote STT response must be JSON with a 'text' field")?;
    Ok(result.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_base_url_or_full_endpoint() {
        assert_eq!(validate_server_url("http://localhost:8000/v1/").unwrap(), "http://localhost:8000/v1/audio/transcriptions");
        assert_eq!(validate_server_url("http://localhost:8000").unwrap(), "http://localhost:8000/v1/audio/transcriptions");
        assert_eq!(validate_server_url("https://stt.example/v1/audio/transcriptions").unwrap(), "https://stt.example/v1/audio/transcriptions");
        assert!(validate_server_url("file:///tmp/x").is_err());
    }
    #[test]
    fn wav_has_16khz_mono_header() {
        let wav = encode_wav(&[0.0, 0.5, -0.5]).unwrap();
        let reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, 16000);
        assert_eq!(reader.duration(), 3);
    }
}
