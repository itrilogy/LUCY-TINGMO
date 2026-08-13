use crate::managers::transcription::TranscriptionManager;
use crate::overlay::hide_recording_overlay;
use crate::settings::{get_settings, write_settings, ModelUnloadTimeout};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

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
    transcription_manager: State<TranscriptionManager>,
) -> Result<ModelLoadStatus, String> {
    Ok(ModelLoadStatus {
        is_loaded: transcription_manager.is_model_loaded(),
        current_model: transcription_manager.get_current_model(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn unload_model_manually(
    transcription_manager: State<TranscriptionManager>,
) -> Result<(), String> {
    transcription_manager
        .unload_model()
        .map_err(|e| format!("Failed to unload model: {}", e))
}

/// Copy transcription text to the system clipboard without simulating paste.
/// Product path after recording: overlay "Copy" only (no auto-paste).
#[tauri::command]
#[specta::specta]
pub fn copy_text_to_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))
}

/// Explicit "paste to frontmost app" — never automatic (product decision D8/P1-T1).
/// Always copies the given text to the clipboard first, then pastes; leaves the
/// text on the clipboard (does not restore prior clipboard contents).
#[tauri::command]
#[specta::specta]
pub fn paste_text_to_frontmost(app: AppHandle, text: String) -> Result<(), String> {
    crate::utils::paste_to_frontmost(text, app)
}

/// Dismiss the recording/result overlay after the user is done.
#[tauri::command]
#[specta::specta]
pub fn dismiss_overlay(app: AppHandle) {
    hide_recording_overlay(&app);
}
