use crate::markers::{
    accept_all_pending, chrono_date_string, invalidate_cache, load_store, remove_marker_at,
    save_store, upsert_marker, AsrMarker, AsrMarkerStore, MarkerChangelogEntry, MarkerConfidence,
};
use std::fs;
use tauri::{AppHandle, Manager};

#[tauri::command]
#[specta::specta]
pub fn get_asr_marker_store(app: AppHandle) -> Result<AsrMarkerStore, String> {
    load_store(&app)
}

#[tauri::command]
#[specta::specta]
pub fn save_asr_marker_store(app: AppHandle, store: AsrMarkerStore) -> Result<(), String> {
    save_store(&app, &store)
}

#[tauri::command]
#[specta::specta]
pub fn add_asr_marker(app: AppHandle, marker: AsrMarker) -> Result<AsrMarkerStore, String> {
    let mut store = load_store(&app)?;
    let is_new = upsert_marker(&mut store, marker);
    if is_new {
        store.changelog.push(MarkerChangelogEntry {
            date: chrono_date_string(),
            summary: "Added marker from UI".to_string(),
            source: Some("ui".to_string()),
            count: 1,
        });
    }
    save_store(&app, &store)?;
    Ok(store)
}

#[tauri::command]
#[specta::specta]
pub fn remove_asr_marker(app: AppHandle, index: usize) -> Result<AsrMarkerStore, String> {
    let mut store = load_store(&app)?;
    remove_marker_at(&mut store, index)?;
    save_store(&app, &store)?;
    Ok(store)
}

#[tauri::command]
#[specta::specta]
pub fn accept_pending_asr_markers(app: AppHandle) -> Result<AsrMarkerStore, String> {
    let mut store = load_store(&app)?;
    let n = accept_all_pending(&mut store);
    if n > 0 {
        save_store(&app, &store)?;
    }
    Ok(store)
}

#[tauri::command]
#[specta::specta]
pub fn set_asr_marker_confidence(
    app: AppHandle,
    index: usize,
    confidence: MarkerConfidence,
) -> Result<AsrMarkerStore, String> {
    let mut store = load_store(&app)?;
    let m = store
        .markers
        .get_mut(index)
        .ok_or_else(|| format!("Marker index {index} out of range"))?;
    m.confidence = confidence;
    save_store(&app, &store)?;
    Ok(store)
}

#[tauri::command]
#[specta::specta]
pub fn clear_asr_marker_cache() {
    invalidate_cache();
}

#[tauri::command]
#[specta::specta]
pub fn get_asr_markers_file_path(app: AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    Ok(dir.join("asr_markers.json").display().to_string())
}

/// Import markers from a Markdown table dump (e.g. Obsidian 标识物清单).
/// Accepts lines like: `| 误识 | 正确 | 类别 |`
#[tauri::command]
#[specta::specta]
pub fn import_asr_markers_markdown(
    app: AppHandle,
    markdown: String,
    source: Option<String>,
) -> Result<AsrMarkerStore, String> {
    let parsed = crate::markers::parse_correction_output(
        &format!("body\n\n【新增标识物】\n{markdown}"),
        source.as_deref(),
    );
    let mut store = load_store(&app)?;
    let n = crate::markers::merge_markers(
        &mut store,
        parsed.new_markers,
        Some(MarkerConfidence::Pending),
        source.as_deref().or(Some("markdown_import")),
    );
    if n > 0 {
        save_store(&app, &store)?;
    }
    Ok(store)
}

/// Export the marker store as an Obsidian-friendly Markdown note body.
#[tauri::command]
#[specta::specta]
pub fn export_asr_markers_markdown(app: AppHandle) -> Result<String, String> {
    let store = load_store(&app)?;
    Ok(crate::markers::format_markers_markdown_export(&store))
}

/// Write markers Markdown next to `asr_markers.json` (for vault folder sync / copy).
/// Returns the absolute path written.
#[tauri::command]
#[specta::specta]
pub fn export_asr_markers_markdown_file(app: AppHandle) -> Result<String, String> {
    let store = load_store(&app)?;
    let md = crate::markers::format_markers_markdown_export(&store);
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    }
    let path = dir.join("asr_markers.md");
    fs::write(&path, md).map_err(|e| format!("Failed to write markers markdown: {e}"))?;
    Ok(path.display().to_string())
}

/// Quick “one-word correction” from history/UI: wrong → correct (verified).
#[tauri::command]
#[specta::specta]
pub fn correct_asr_word(
    app: AppHandle,
    wrong: String,
    correct: String,
    category: Option<String>,
) -> Result<AsrMarkerStore, String> {
    let wrong = wrong.trim().to_string();
    let correct = correct.trim().to_string();
    if wrong.is_empty() || correct.is_empty() {
        return Err("wrong and correct must be non-empty".into());
    }
    let mut store = load_store(&app)?;
    let marker = AsrMarker {
        asr_errors: vec![wrong],
        correct,
        category: category.unwrap_or_else(|| "other".to_string()),
        note: Some("history/UI one-word correction".into()),
        source: Some("history_correct".into()),
        confidence: MarkerConfidence::Verified,
        match_mode: crate::markers::MarkerMatchMode::Exact,
    };
    upsert_marker(&mut store, marker);
    store.changelog.push(MarkerChangelogEntry {
        date: chrono_date_string(),
        summary: "One-word correction from UI".into(),
        source: Some("history_correct".into()),
        count: 1,
    });
    save_store(&app, &store)?;
    Ok(store)
}
