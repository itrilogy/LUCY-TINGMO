//! ASR correction markers (易错词 / 标识物) knowledge base.
//!
//! Persistent `error → correct` pairs used for:
//! 1. Rule-based pre-correction after raw ASR
//! 2. Glossary injection into LLM post-process prompts (`${glossary}`)
//! 3. Incremental merge of newly discovered pairs from LLM output

use log::{debug, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const MARKERS_FILE: &str = "asr_markers.json";
const STORE_VERSION: u32 = 1;

/// Confidence of a marker entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarkerConfidence {
    /// Human- or pipeline-verified; safe for rule replacement.
    #[default]
    Verified,
    /// Proposed by LLM; shown in UI until accepted.
    Pending,
}

/// How `asr_errors` patterns match the transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarkerMatchMode {
    /// Exact substring (default; longest patterns first).
    #[default]
    Exact,
    /// Case-insensitive substring (mainly Latin; CJK is unchanged).
    CaseInsensitive,
    /// Full Rust/PCRE-like regex; replacement may use `$1` capture groups.
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AsrMarker {
    /// One or more mis-recognized surface forms.
    pub asr_errors: Vec<String>,
    /// Canonical correct form.
    pub correct: String,
    /// Taxonomy bucket (org / term / person / place / code / format / other).
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub note: Option<String>,
    /// First discovery scene / source label.
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub confidence: MarkerConfidence,
    /// Match strategy for `asr_errors` (default exact).
    #[serde(default)]
    pub match_mode: MarkerMatchMode,
}

fn default_category() -> String {
    "other".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MarkerChangelogEntry {
    pub date: String,
    pub summary: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AsrMarkerStore {
    #[serde(default = "default_store_version")]
    pub version: u32,
    #[serde(default)]
    pub markers: Vec<AsrMarker>,
    #[serde(default)]
    pub changelog: Vec<MarkerChangelogEntry>,
}

fn default_store_version() -> u32 {
    STORE_VERSION
}

impl Default for AsrMarkerStore {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            markers: Vec::new(),
            changelog: Vec::new(),
        }
    }
}

static MARKERS_CACHE: Mutex<Option<AsrMarkerStore>> = Mutex::new(None);

fn markers_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    }
    Ok(dir.join(MARKERS_FILE))
}

pub fn load_store(app: &AppHandle) -> Result<AsrMarkerStore, String> {
    {
        let cache = MARKERS_CACHE.lock().unwrap();
        if let Some(ref s) = *cache {
            return Ok(s.clone());
        }
    }
    let path = markers_path(app)?;
    let store = load_store_from_path(&path)?;
    *MARKERS_CACHE.lock().unwrap() = Some(store.clone());
    Ok(store)
}

fn load_store_from_path(path: &Path) -> Result<AsrMarkerStore, String> {
    if !path.exists() {
        return Ok(AsrMarkerStore::default());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("Failed to read markers: {e}"))?;
    if raw.trim().is_empty() {
        return Ok(AsrMarkerStore::default());
    }
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse markers JSON: {e}"))
}

pub fn save_store(app: &AppHandle, store: &AsrMarkerStore) -> Result<(), String> {
    let path = markers_path(app)?;
    let raw = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Failed to serialize markers: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("Failed to write markers: {e}"))?;
    *MARKERS_CACHE.lock().unwrap() = Some(store.clone());
    debug!("Saved {} ASR markers to {}", store.markers.len(), path.display());
    Ok(())
}

pub fn invalidate_cache() {
    *MARKERS_CACHE.lock().unwrap() = None;
}

/// Rule replacement using verified markers (longest error forms first).
/// Supports exact, case-insensitive, and regex match modes per marker.
pub fn apply_rule_corrections(text: &str, store: &AsrMarkerStore) -> String {
    struct Pair<'a> {
        pattern: &'a str,
        correct: &'a str,
        mode: MarkerMatchMode,
        rank: usize,
    }

    let mut pairs: Vec<Pair<'_>> = Vec::new();
    for m in &store.markers {
        if m.confidence != MarkerConfidence::Verified {
            continue;
        }
        if m.correct.trim().is_empty() {
            continue;
        }
        for err in &m.asr_errors {
            let e = err.trim();
            if e.is_empty() {
                continue;
            }
            // Exact/CI: skip no-op identity. Regex may intentionally reformat.
            if m.match_mode != MarkerMatchMode::Regex && e == m.correct {
                continue;
            }
            pairs.push(Pair {
                pattern: e,
                correct: m.correct.as_str(),
                mode: m.match_mode,
                rank: e.chars().count(),
            });
        }
    }
    // Longer mismatches first to avoid partial clobber (exact/CI).
    pairs.sort_by(|a, b| b.rank.cmp(&a.rank));

    let mut out = text.to_string();
    let mut applied = 0u32;
    for p in pairs {
        let before = out.clone();
        match p.mode {
            MarkerMatchMode::Exact => {
                if out.contains(p.pattern) {
                    out = out.replace(p.pattern, p.correct);
                }
            }
            MarkerMatchMode::CaseInsensitive => {
                out = replace_case_insensitive(&out, p.pattern, p.correct);
            }
            MarkerMatchMode::Regex => match Regex::new(p.pattern) {
                Ok(re) => {
                    let replaced = re.replace_all(&out, p.correct);
                    out = replaced.into_owned();
                }
                Err(e) => {
                    warn!(
                        "ASR marker regex invalid, skipped pattern {:?}: {}",
                        p.pattern, e
                    );
                }
            },
        }
        if out != before {
            applied += 1;
        }
    }
    if applied > 0 {
        info!("ASR marker rules applied {} replacement pattern(s)", applied);
    }
    out
}

/// Case-insensitive substring replace (left-to-right, non-overlapping).
fn replace_case_insensitive(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_string();
    }
    let hay_lower: Vec<char> = haystack.to_lowercase().chars().collect();
    let needle_lower: Vec<char> = needle.to_lowercase().chars().collect();
    let hay_chars: Vec<char> = haystack.chars().collect();
    if needle_lower.len() > hay_lower.len() {
        return haystack.to_string();
    }

    let mut out = String::with_capacity(haystack.len());
    let mut i = 0usize;
    while i < hay_chars.len() {
        let end = i + needle_lower.len();
        if end <= hay_chars.len() && hay_lower[i..end] == needle_lower[..] {
            out.push_str(replacement);
            i = end;
        } else {
            out.push(hay_chars[i]);
            i += 1;
        }
    }
    out
}

/// Compact glossary block for `${glossary}` prompt injection (verified + pending).
pub fn format_glossary_for_prompt(store: &AsrMarkerStore) -> String {
    if store.markers.is_empty() {
        return "(empty — no markers yet)".to_string();
    }
    let mut lines = vec![
        "| ASR misrecognition | Correct form | Category | Confidence |".to_string(),
        "|---|---|---|---|".to_string(),
    ];
    for m in &store.markers {
        let conf = match m.confidence {
            MarkerConfidence::Verified => "verified",
            MarkerConfidence::Pending => "pending",
        };
        let errors = m.asr_errors.join(" / ");
        lines.push(format!(
            "| {} | {} | {} | {} |",
            escape_md_cell(&errors),
            escape_md_cell(&m.correct),
            escape_md_cell(&m.category),
            conf
        ));
    }
    lines.join("\n")
}

/// Markdown document suitable for Obsidian / vault notes (export side of sync).
///
/// Import the table body with `import_asr_markers_markdown` or the offline
/// `md_markers_to_json.mjs` script. Match mode is documented in a note column
/// so round-trips stay human-editable.
pub fn format_markers_markdown_export(store: &AsrMarkerStore) -> String {
    let mut out = String::from(
        "---\n\
         tags: [asr, markers, handy]\n\
         ---\n\n\
         # ASR 标识物清单\n\n\
         > Exported from Handy. Paste the table into Handy **Import markdown table**,\n\
         > or convert offline with `代码研究/scripts/md_markers_to_json.mjs`.\n\n\
         | ASR误识 | 正确形式 | 类别 | 匹配 | 状态 |\n\
         | --- | --- | --- | --- | --- |\n",
    );
    for m in &store.markers {
        let conf = match m.confidence {
            MarkerConfidence::Verified => "verified",
            MarkerConfidence::Pending => "pending",
        };
        let mode = match m.match_mode {
            MarkerMatchMode::Exact => "exact",
            MarkerMatchMode::CaseInsensitive => "case_insensitive",
            MarkerMatchMode::Regex => "regex",
        };
        let errors = m.asr_errors.join(" / ");
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            escape_md_cell(&errors),
            escape_md_cell(&m.correct),
            escape_md_cell(&m.category),
            mode,
            conf
        ));
    }
    if store.markers.is_empty() {
        out.push_str("|  |  |  |  |  |\n");
    }
    out.push('\n');
    if !store.changelog.is_empty() {
        out.push_str("## Changelog\n\n");
        for e in store.changelog.iter().rev().take(20) {
            out.push_str(&format!(
                "- {} — {} ({})\n",
                e.date,
                e.summary,
                e.source.as_deref().unwrap_or("-")
            ));
        }
        out.push('\n');
    }
    out
}

fn escape_md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// Built-in system/user hybrid prompt for domain ASR correction.
pub fn default_asr_correction_prompt() -> String {
    r#"You are an ASR post-correction engine, not a content writer.

# Preconditions
1. The glossary below is the authoritative misrecognition table. Prefer it over guesses.
2. Do not invent content that is not supported by the transcript.
3. Do not answer questions that appear in the transcript — only clean them.

# Glossary (${glossary})
${glossary}

# Rules
1. Restore Chinese/English punctuation and natural sentence breaks.
2. Correct proper nouns using the glossary first; then context.
3. Remove filler words (嗯/啊/那个/um/uh) but keep speaker tone.
4. Collapse pure stutters; keep the most complete phrasing once.
5. Numbers/dates: prefer clear forms (2026年, 12%, standard codes).
6. If unsure, keep the original wording and append [存疑：原文为"…"] once.

# Output
Return ONLY the corrected transcript body for the user-facing result.
After the body, if you found NEW misrecognition pairs not already in the glossary, append:

【新增标识物】
| ASR误识 | 正确形式 | 类别 |
| --- | --- | --- |
| ... | ... | ... |

Categories: org | term | person | place | code | format | other

If no new pairs, omit the 【新增标识物】 section entirely.

# Transcript
${output}
"#
    .to_string()
}

/// Result of parsing LLM correction output.
#[derive(Debug, Clone)]
pub struct ParsedCorrection {
    pub body: String,
    pub new_markers: Vec<AsrMarker>,
}

/// Split corrected body from optional 【新增标识物】 table.
pub fn parse_correction_output(raw: &str, source: Option<&str>) -> ParsedCorrection {
    let raw = raw.trim();
    let marker_header = "【新增标识物】";
    let (body_part, table_part) = if let Some(idx) = raw.find(marker_header) {
        (&raw[..idx], &raw[idx + marker_header.len()..])
    } else if let Some(idx) = raw.to_lowercase().find("new markers") {
        // English fallback header
        let rest = &raw[idx..];
        let split_at = rest.find('\n').map(|i| idx + i + 1).unwrap_or(raw.len());
        (&raw[..idx], &raw[split_at..])
    } else {
        return ParsedCorrection {
            body: raw.to_string(),
            new_markers: Vec::new(),
        };
    };

    let body = body_part.trim().to_string();
    let new_markers = parse_marker_table(table_part, source);
    ParsedCorrection { body, new_markers }
}

fn parse_marker_table(table: &str, source: Option<&str>) -> Vec<AsrMarker> {
    let mut out = Vec::new();
    for line in table.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        if cells.len() < 2 {
            continue;
        }
        // Skip header / separator rows
        let c0 = cells[0];
        let c1 = cells[1];
        if c0.contains("ASR") || c0.contains("---") || c0.contains("误识") {
            continue;
        }
        if c1.contains("---") || c1.contains("正确") {
            continue;
        }
        if c0.is_empty() || c1.is_empty() {
            continue;
        }
        let category = cells
            .get(2)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(default_category);
        let errors: Vec<String> = c0
            .split(['/', '、', ',', '，'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if errors.is_empty() {
            continue;
        }
        out.push(AsrMarker {
            asr_errors: errors,
            correct: c1.to_string(),
            category,
            note: None,
            source: source.map(|s| s.to_string()),
            confidence: MarkerConfidence::Pending,
            match_mode: MarkerMatchMode::Exact,
        });
    }
    out
}

/// Merge candidates into store. Skips duplicates (same correct + overlapping error).
/// Returns number of newly inserted markers.
pub fn merge_markers(
    store: &mut AsrMarkerStore,
    candidates: Vec<AsrMarker>,
    force_confidence: Option<MarkerConfidence>,
    changelog_source: Option<&str>,
) -> u32 {
    let mut added = 0u32;
    for mut c in candidates {
        if let Some(conf) = force_confidence {
            c.confidence = conf;
        }
        if c.correct.trim().is_empty() || c.asr_errors.is_empty() {
            continue;
        }
        if marker_exists(store, &c) {
            continue;
        }
        store.markers.push(c);
        added += 1;
    }
    if added > 0 {
        let today = chrono_date_string();
        store.changelog.push(MarkerChangelogEntry {
            date: today,
            summary: format!("Added {added} marker(s)"),
            source: changelog_source.map(|s| s.to_string()),
            count: added,
        });
        info!("Merged {added} new ASR marker(s) into store");
    }
    added
}

fn marker_exists(store: &AsrMarkerStore, candidate: &AsrMarker) -> bool {
    for m in &store.markers {
        if m.correct != candidate.correct {
            continue;
        }
        for e in &candidate.asr_errors {
            if m.asr_errors.iter().any(|x| x == e) {
                return true;
            }
        }
    }
    false
}

pub fn chrono_date_string() -> String {
    // Local date YYYY-MM-DD without pulling chrono dependency if unavailable —
    // use system time formatting via humantime-less approach.
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Approximate UTC date; good enough for changelog labels.
    // 86400 seconds per day; Unix epoch is 1970-01-01.
    let days = secs / 86400;
    // Civil date from days since epoch (Howard Hinnant algorithm, simplified)
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Accept all pending markers (promote to verified).
pub fn accept_all_pending(store: &mut AsrMarkerStore) -> u32 {
    let mut n = 0u32;
    for m in &mut store.markers {
        if m.confidence == MarkerConfidence::Pending {
            m.confidence = MarkerConfidence::Verified;
            n += 1;
        }
    }
    if n > 0 {
        store.changelog.push(MarkerChangelogEntry {
            date: chrono_date_string(),
            summary: format!("Accepted {n} pending marker(s)"),
            source: Some("ui".to_string()),
            count: n,
        });
    }
    n
}

/// Delete marker by index.
pub fn remove_marker_at(store: &mut AsrMarkerStore, index: usize) -> Result<(), String> {
    if index >= store.markers.len() {
        return Err(format!("Marker index {index} out of range"));
    }
    let removed = store.markers.remove(index);
    store.changelog.push(MarkerChangelogEntry {
        date: chrono_date_string(),
        summary: format!("Removed marker → {}", removed.correct),
        source: Some("ui".to_string()),
        count: 1,
    });
    Ok(())
}

/// Upsert a single marker from UI (verified by default).
pub fn upsert_marker(store: &mut AsrMarkerStore, marker: AsrMarker) -> bool {
    if marker_exists(store, &marker) {
        // Extend errors on matching correct form
        for m in &mut store.markers {
            if m.correct == marker.correct {
                for e in &marker.asr_errors {
                    if !m.asr_errors.contains(e) {
                        m.asr_errors.push(e.clone());
                    }
                }
                if marker.confidence == MarkerConfidence::Verified {
                    m.confidence = MarkerConfidence::Verified;
                }
                return false;
            }
        }
    }
    store.markers.push(marker);
    true
}

/// Inject `${glossary}` (and leave `${output}` for existing pipeline).
pub fn inject_glossary(prompt_template: &str, glossary: &str) -> String {
    if prompt_template.contains("${glossary}") {
        prompt_template.replace("${glossary}", glossary)
    } else if glossary != "(empty — no markers yet)" && !glossary.is_empty() {
        // Append glossary block when template forgot the placeholder but store has data.
        format!(
            "{prompt_template}\n\n# Glossary (auto-injected)\n{glossary}\n"
        )
    } else {
        prompt_template.to_string()
    }
}

/// Whether this prompt id/content should run the ASR correction parse path.
pub fn is_asr_correction_prompt(prompt_id: &str, prompt_body: &str) -> bool {
    prompt_id == "default_asr_domain_correct"
        || prompt_body.contains("${glossary}")
        || prompt_body.contains("【新增标识物】")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_rules_longest_first() {
        let store = AsrMarkerStore {
            version: 1,
            markers: vec![
                AsrMarker {
                    asr_errors: vec!["国家烟草转卖局".into(), "转卖".into()],
                    correct: "国家烟草专卖局".into(),
                    category: "org".into(),
                    note: None,
                    source: None,
                    confidence: MarkerConfidence::Verified,
                    match_mode: MarkerMatchMode::Exact,
                },
                AsrMarker {
                    asr_errors: vec!["转卖".into()],
                    correct: "专卖".into(),
                    category: "term".into(),
                    note: None,
                    source: None,
                    confidence: MarkerConfidence::Verified,
                    match_mode: MarkerMatchMode::Exact,
                },
            ],
            changelog: vec![],
        };
        // Long form should win when present as a contiguous phrase.
        let out = apply_rule_corrections("访问国家烟草转卖局", &store);
        assert!(out.contains("国家烟草专卖局"), "got {out}");
    }

    #[test]
    fn apply_rules_case_insensitive() {
        let store = AsrMarkerStore {
            version: 1,
            markers: vec![AsrMarker {
                asr_errors: vec!["handy".into()],
                correct: "Handy".into(),
                category: "term".into(),
                note: None,
                source: None,
                confidence: MarkerConfidence::Verified,
                match_mode: MarkerMatchMode::CaseInsensitive,
            }],
            changelog: vec![],
        };
        let out = apply_rule_corrections("I use HANDY daily and handy rocks", &store);
        assert_eq!(out, "I use Handy daily and Handy rocks");
    }

    #[test]
    fn apply_rules_regex_capture() {
        let store = AsrMarkerStore {
            version: 1,
            markers: vec![AsrMarker {
                asr_errors: vec![r"标{2,}".into()],
                correct: "标".into(),
                category: "format".into(),
                note: None,
                source: None,
                confidence: MarkerConfidence::Verified,
                match_mode: MarkerMatchMode::Regex,
            }],
            changelog: vec![],
        };
        let out = apply_rule_corrections("出现标标标问题", &store);
        assert_eq!(out, "出现标问题");
    }

    #[test]
    fn parse_new_markers_table() {
        let raw = r#"今天去职工进修学院开会。

【新增标识物】
| ASR误识 | 正确形式 | 类别 |
| --- | --- | --- |
| 职培精 | 职工进修学院 | org |
| 标体细 | 标准体系 | term |
"#;
        let parsed = parse_correction_output(raw, Some("test"));
        assert!(parsed.body.contains("职工进修学院"));
        assert!(!parsed.body.contains("新增标识物"));
        assert_eq!(parsed.new_markers.len(), 2);
        assert_eq!(parsed.new_markers[0].correct, "职工进修学院");
        assert_eq!(parsed.new_markers[0].confidence, MarkerConfidence::Pending);
    }

    #[test]
    fn merge_skips_duplicates() {
        let mut store = AsrMarkerStore::default();
        let c = AsrMarker {
            asr_errors: vec!["a".into()],
            correct: "A".into(),
            category: "other".into(),
            note: None,
            source: None,
            confidence: MarkerConfidence::Pending,
            match_mode: MarkerMatchMode::Exact,
        };
        assert_eq!(merge_markers(&mut store, vec![c.clone()], None, None), 1);
        assert_eq!(merge_markers(&mut store, vec![c], None, None), 0);
        assert_eq!(store.markers.len(), 1);
    }

    #[test]
    fn inject_glossary_placeholder() {
        let p = "G:\n${glossary}\n\nT:\n${output}";
        let out = inject_glossary(p, "table");
        assert!(out.contains("table"));
        assert!(!out.contains("${glossary}"));
        assert!(out.contains("${output}"));
    }
}
