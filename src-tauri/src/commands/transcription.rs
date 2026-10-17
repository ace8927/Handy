use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, write_settings, ModelUnloadTimeout};
use serde::Serialize;
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[derive(Serialize, Type)]
pub struct ModelLoadStatus {
    is_loaded: bool,
    current_model: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub fn set_model_unload_timeout(app: AppHandle, timeout: ModelUnloadTimeout) {
    let mut settings = get_settings(&app);
    settings.model_unload_timeout = timeout;
    write_settings(&app, settings);
}

#[tauri::command]
#[specta::specta]
pub fn get_model_load_status(
    transcription_manager: State<Arc<TranscriptionManager>>,
) -> Result<ModelLoadStatus, String> {
    Ok(ModelLoadStatus {
        is_loaded: transcription_manager.is_model_loaded(),
        current_model: transcription_manager.get_current_model(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn unload_model_manually(
    transcription_manager: State<Arc<TranscriptionManager>>,
) -> Result<(), String> {
    transcription_manager.request_unload();
    Ok(())
}


/// Only change in Handy behavior: choose existing local transcriber or a
/// previously configured OpenAI-compatible remote transcription server.
#[tauri::command]
#[specta::specta]
pub fn configure_remote_stt(
    app: AppHandle,
    mode: crate::settings::SttMode,
    url: String,
    model: String,
    api_key: String,
) -> Result<(), String> {
    use crate::managers::audio::AudioRecordingManager;
    use crate::managers::transcription::TranscriptionManager;
    use crate::settings::{get_settings, write_settings, SttMode};
    use std::sync::Arc;

    if app.state::<Arc<AudioRecordingManager>>().is_recording() {
        return Err("Stop the current recording before switching STT mode".to_string());
    }
    if mode == SttMode::Remote {
        if model.trim().is_empty() {
            return Err("Enter a Remote STT model name".to_string());
        }
        // Validate URL before changing the persisted selection.
        crate::remote_transcription::validate_server_url(&url)
            .map_err(|e| e.to_string())?;
    }

    let tm = app.state::<Arc<TranscriptionManager>>();
    if mode == SttMode::Remote {
        // Prevent a concurrent local-model load from racing the unload.
        let _loading_guard = tm.try_start_loading()
            .ok_or_else(|| "Wait for the local model to finish loading".to_string())?;
        let mut settings = get_settings(&app);
        settings.stt_mode = mode;
        settings.remote_stt_url = url.trim().to_owned();
        settings.remote_stt_model = model.trim().to_owned();
        settings.remote_stt_api_keys.insert("server".to_owned(), api_key);
        write_settings(&app, settings);
        tm.request_unload();
    } else {
        let mut settings = get_settings(&app);
        settings.stt_mode = mode;
        settings.remote_stt_url = url.trim().to_owned();
        settings.remote_stt_model = model.trim().to_owned();
        settings.remote_stt_api_keys.insert("server".to_owned(), api_key);
        write_settings(&app, settings);
        // The original lazy-load path loads the local model on the next use.
    }
    Ok(())
}
