//! L1 pause-based punctuation and L2 hooks for semi-realtime text.
//!
//! L1: map trailing silence after a VAD segment to ， / 。 / newline.
//! L2: optional local punctuation model on *completed* segment raw text
//!     (pluggable; default is a no-op until a model is registered).
//! L3: LLM post-process lives on history / explicit user action (not here).

use crate::settings::TextPunctuationMode;
use log::debug;
use std::sync::Mutex;

// TextPunctuationMode lives in settings.rs (specta / store).

/// Defaults for pause → punctuation mapping (ms of trailing silence).
/// VAD hangover is ~450ms; map that band to a full stop (utterance end).
pub const PAUSE_COMMA_MS: u64 = 300;
pub const PAUSE_PERIOD_MS: u64 = 400;
pub const PAUSE_PARAGRAPH_MS: u64 = 1500;

/// One finished speech segment for the pipeline.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SegmentText {
    /// Unpunctuated (or lightly cleaned) ASR text.
    pub raw: String,
    /// Display text after L1 (+ optional L2).
    pub display: String,
    pub silence_ms: u64,
    pub force_cut: bool,
}

/// Apply L1 pause rules to a raw segment.
pub fn apply_pause_rules(raw: &str, silence_ms: u64, force_cut: bool) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    // Already ends with CJK/Latin sentence punctuation — leave as-is.
    if ends_with_sentence_punct(t) {
        return t.to_string();
    }

    if force_cut {
        // Hard window cut mid-flow: soft break, not a fake full stop.
        return format!("{t}…");
    }

    let punct = if silence_ms >= PAUSE_PARAGRAPH_MS {
        "。\n"
    } else if silence_ms >= PAUSE_PERIOD_MS {
        "。"
    } else if silence_ms >= PAUSE_COMMA_MS {
        "，"
    } else {
        // Below hangover-ish short gap: still a segment boundary; prefer comma.
        "，"
    };

    format!("{t}{punct}")
}

fn ends_with_sentence_punct(s: &str) -> bool {
    matches!(
        s.chars().rev().find(|c| !c.is_whitespace()),
        Some('。' | '？' | '！' | '；' | '.' | '?' | '!' | '…' | '，' | ',')
    )
}

/// Join display segments for full transcript (expanded live view / finalize).
///
/// Each completed VAD / semi-realtime segment becomes its own line so the
/// overlay and history stay readable; trailing whitespace per segment is
/// trimmed, and empty segments are skipped.
pub fn join_display_segments(parts: &[String]) -> String {
    let mut lines: Vec<&str> = Vec::with_capacity(parts.len());
    for part in parts {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        // A segment may already contain internal newlines (e.g. long-pause L1);
        // keep those, but still separate *segments* with a blank? No — one
        // newline between segments is enough; preserve internal newlines as-is.
        lines.push(t);
    }
    lines.join("\n")
}

/// L2: local punctuation on raw ASR text (full transcript or one segment).
pub trait LocalPunctuator: Send + Sync {
    fn refine(&self, raw: &str) -> String;
}

/// Interim L2 until a downloaded ONNX/sherpa punct model is registered.
/// Discourse-marker heuristics for Chinese ASR dumps — not semantic NLP.
pub struct HeuristicPunctuator;

impl LocalPunctuator for HeuristicPunctuator {
    fn refine(&self, raw: &str) -> String {
        refine_heuristic(raw)
    }
}

fn refine_heuristic(raw: &str) -> String {
    let mut t = raw.trim().to_string();
    if t.is_empty() {
        return t;
    }
    // Strip trailing junk ellipses from force-cuts before re-punctuating.
    while t.ends_with('…') {
        t.pop();
    }
    t = t.trim().to_string();

    // Insert sentence breaks before common discourse openers (mid-string).
    const BREAKS: &[&str] = &[
        "但是",
        "然而",
        "因此",
        "所以",
        "另外",
        "此外",
        "第一",
        "第二",
        "第三",
        "首先",
        "其次",
        "最后",
        "总之",
        "接下来",
        "下面",
        "各位",
        "同志们",
        "谢谢大家",
    ];
    for b in BREAKS {
        let needle = *b;
        // Avoid breaking at start
        if let Some(idx) = t.find(needle) {
            if idx > 0 {
                let prev = t[..idx].chars().last();
                if prev.is_some_and(|c| !matches!(c, '。' | '？' | '！' | '；' | '\n' | '，' | ','))
                {
                    t = format!("{}。{}", &t[..idx], &t[idx..]);
                }
            }
        }
    }

    // Question particles near end of clauses
    t = t.replace("吗。", "吗？");
    t = t.replace("呢。", "呢？");
    if t.ends_with('吗') || t.ends_with('呢') || t.ends_with('么') {
        t.push('？');
    } else if !ends_with_sentence_punct(&t) {
        t.push('。');
    }
    t
}

static LOCAL_PUNCTUATOR: Mutex<Option<Box<dyn LocalPunctuator>>> = Mutex::new(None);

/// Register L2 (real ONNX model later). Pass `None` to clear.
#[allow(dead_code)]
pub fn set_local_punctuator(p: Option<Box<dyn LocalPunctuator>>) {
    *LOCAL_PUNCTUATOR.lock().unwrap() = p;
}

/// Install the built-in heuristic L2 if nothing is registered yet.
pub fn ensure_default_punctuator() {
    let mut g = LOCAL_PUNCTUATOR.lock().unwrap();
    if g.is_none() {
        *g = Some(Box::new(HeuristicPunctuator));
        debug!("L2: registered HeuristicPunctuator (replace with ONNX punc when available)");
    }
}

#[allow(dead_code)]
pub fn local_punctuator_available() -> bool {
    LOCAL_PUNCTUATOR.lock().unwrap().is_some()
}

/// Pure batch / history path: **no L1** — raw ASR then L2 only.
///
/// - `None`: return raw
/// - `Rules` / `LocalPunc`: run L2 refine on full text (Rules on batch skips
///   pause mapping; pause L1 is live-only)
pub fn punctuate_batch_transcript(raw: &str, mode: TextPunctuationMode) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    match mode {
        TextPunctuationMode::None => raw.to_string(),
        TextPunctuationMode::Rules | TextPunctuationMode::LocalPunc => {
            ensure_default_punctuator();
            if let Some(ref p) = *LOCAL_PUNCTUATOR.lock().unwrap() {
                let out = p.refine(raw);
                if out.trim().is_empty() {
                    raw.to_string()
                } else {
                    out
                }
            } else {
                raw.to_string()
            }
        }
    }
}

/// Run L1 and optional L2 on a finished segment.
pub fn punctuate_segment(
    raw: &str,
    silence_ms: u64,
    force_cut: bool,
    mode: TextPunctuationMode,
) -> SegmentText {
    let raw = raw.trim().to_string();
    let display = match mode {
        TextPunctuationMode::None => raw.clone(),
        TextPunctuationMode::Rules => apply_pause_rules(&raw, silence_ms, force_cut),
        TextPunctuationMode::LocalPunc => {
            // Live: L1 for immediate display shape, L2 on *raw* for better clauses.
            ensure_default_punctuator();
            let ruled = apply_pause_rules(&raw, silence_ms, force_cut);
            if let Some(ref p) = *LOCAL_PUNCTUATOR.lock().unwrap() {
                let refined = p.refine(&raw);
                if refined.trim().is_empty() {
                    ruled
                } else if ends_with_sentence_punct(&refined) {
                    refined
                } else if force_cut {
                    format!("{}…", refined.trim())
                } else if silence_ms >= PAUSE_PARAGRAPH_MS {
                    format!("{}。\n", refined.trim())
                } else if silence_ms >= PAUSE_PERIOD_MS {
                    format!("{}。", refined.trim())
                } else {
                    refined
                }
            } else {
                ruled
            }
        }
    };
    SegmentText {
        raw,
        display,
        silence_ms,
        force_cut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_pause_gets_comma() {
        let s = apply_pause_rules("今天开会", 350, false);
        assert!(s.ends_with('，'), "{s}");
    }

    #[test]
    fn long_pause_gets_period() {
        // VAD hangover ~450ms maps to period under default thresholds.
        let s = apply_pause_rules("今天开会", 450, false);
        assert!(s.ends_with('。'), "{s}");
    }

    #[test]
    fn paragraph_pause_newline() {
        let s = apply_pause_rules("第一点", 1600, false);
        assert!(s.contains('\n'), "{s}");
    }

    #[test]
    fn force_cut_ellipsis() {
        let s = apply_pause_rules("说了很久还没停", 0, true);
        assert!(s.ends_with('…'), "{s}");
    }

    #[test]
    fn join_display() {
        let parts = vec!["你好，".into(), "世界。".into()];
        assert_eq!(join_display_segments(&parts), "你好，\n世界。");
    }

    #[test]
    fn join_display_skips_empty() {
        let parts = vec!["第一段。".into(), "  ".into(), "第二段。".into()];
        assert_eq!(join_display_segments(&parts), "第一段。\n第二段。");
    }

    #[test]
    fn batch_l2_adds_period_and_breaks() {
        ensure_default_punctuator();
        let raw = "今天开会讨论标准然后我们看数字化谢谢大家";
        let out = punctuate_batch_transcript(raw, TextPunctuationMode::LocalPunc);
        assert!(out.contains('。') || out.contains('？'), "{out}");
        assert!(out.contains("谢谢大家") || out.ends_with('。'), "{out}");
    }

    #[test]
    fn batch_none_is_identity() {
        let raw = "没有标点";
        assert_eq!(
            punctuate_batch_transcript(raw, TextPunctuationMode::None),
            raw
        );
    }
}
