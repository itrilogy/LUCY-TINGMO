//! Persist post-process prompts as Markdown under app_data/prompts/
//! and open that folder in the system file manager.

use crate::settings::{get_settings, LLMPrompt};
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

const PROMPTS_DIR: &str = "prompts";

pub fn prompts_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::portable::app_data_dir(app)
        .map_err(|e| format!("app data dir: {e}"))?
        .join(PROMPTS_DIR);
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("create prompts dir: {e}"))?;
    }
    Ok(dir)
}

fn safe_filename(id: &str, name: &str) -> String {
    let base = if name.trim().is_empty() {
        id.to_string()
    } else {
        name.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else if c.is_whitespace() {
                    '_'
                } else {
                    '-'
                }
            })
            .collect::<String>()
    };
    let base = base.trim_matches('-').trim_matches('_');
    let base = if base.is_empty() { id } else { &base };
    format!("{base}__{id}.md")
}

fn prompt_path(dir: &Path, prompt: &LLMPrompt) -> PathBuf {
    dir.join(safe_filename(&prompt.id, &prompt.name))
}

/// Write all prompts from settings into the prompts directory.
pub fn export_all_prompts(app: &AppHandle) -> Result<String, String> {
    let dir = prompts_dir(app)?;
    let settings = get_settings(app);
    if let Ok(rd) = fs::read_dir(&dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().and_then(|e| e.to_str()) == Some("md") {
                let _ = fs::remove_file(&p);
            }
        }
    }
    for prompt in &settings.post_process_prompts {
        write_prompt_file(&dir, prompt)?;
    }
    debug!(
        "Exported {} prompt(s) to {}",
        settings.post_process_prompts.len(),
        dir.display()
    );
    Ok(dir.display().to_string())
}

fn write_prompt_file(dir: &Path, prompt: &LLMPrompt) -> Result<(), String> {
    let path = prompt_path(dir, prompt);
    let body = format!(
        "---\nid: {}\nname: {}\n---\n\n{}\n",
        prompt.id,
        prompt.name.replace('\n', " "),
        prompt.prompt.trim_end()
    );
    fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Sync a single prompt file after create/update.
pub fn sync_prompt_file(app: &AppHandle, prompt: &LLMPrompt) -> Result<(), String> {
    let dir = prompts_dir(app)?;
    if let Ok(rd) = fs::read_dir(&dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    if stem.ends_with(&format!("__{}", prompt.id)) {
                        let _ = fs::remove_file(&p);
                    }
                }
            }
        }
    }
    write_prompt_file(&dir, prompt)
}

pub fn remove_prompt_file(app: &AppHandle, id: &str) -> Result<(), String> {
    let dir = prompts_dir(app)?;
    if let Ok(rd) = fs::read_dir(&dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    if stem.ends_with(&format!("__{id}")) {
                        let _ = fs::remove_file(&p);
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn open_prompts_folder(app: &AppHandle) -> Result<(), String> {
    let dir = prompts_dir(app)?;
    let _ = export_all_prompts(app);
    let path = dir.to_string_lossy().to_string();
    app.opener()
        .open_path(path, None::<String>)
        .map_err(|e| format!("Failed to open prompts folder: {e}"))
}
