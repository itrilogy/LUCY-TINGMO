//! Long-audio segmentation for batch ASR.
//!
//! Many models (e.g. SenseVoice) are trained on ~30 s windows. Feeding multi-minute
//! PCM in one shot degrades accuracy badly. This module plans sample ranges so the
//! orchestrator can run the engine per segment and join the text.

use crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE;

/// Default max segment length when the model does not advertise a limit.
pub const DEFAULT_MAX_AUDIO_MS: u64 = 30_000;

/// Look-back window (ms) near the max boundary used to prefer a quiet cut.
pub const DEFAULT_BOUNDARY_SEARCH_MS: u64 = 2_000;

/// Never emit a segment shorter than this (except the final tail).
pub const DEFAULT_MIN_SEGMENT_MS: u64 = 5_000;

/// Sample-level look-back into the previous tile so word onsets at a cut are
/// not clipped. Text join strips the duplicated boundary via
/// [`strip_boundary_overlap`].
pub const DEFAULT_OVERLAP_MS: u64 = 400;

/// Max chars of adjacent-segment prefix/suffix considered when de-duplicating.
pub const DEFAULT_DEDUP_CHARS: usize = 24;

/// RMS window (ms) for silence/energy probing at cut candidates.
const ENERGY_WINDOW_MS: u64 = 100;

#[inline]
pub fn samples_for_ms(ms: u64) -> usize {
    (WHISPER_SAMPLE_RATE as u64 * ms / 1000) as usize
}

/// Inclusive-exclusive sample ranges covering `total_samples` with fixed-size
/// windows and optional sample overlap between consecutive windows.
///
/// Prefer [`plan_energy_aware_ranges`] for production batch splits (avoids
/// double-transcribing overlap). This fixed planner is useful for tests and
/// when PCM energy is unavailable.
pub fn plan_fixed_ranges(
    total_samples: usize,
    max_samples: usize,
    overlap_samples: usize,
) -> Vec<(usize, usize)> {
    if total_samples == 0 {
        return Vec::new();
    }
    let max_samples = max_samples.max(1);
    if total_samples <= max_samples {
        return vec![(0, total_samples)];
    }

    let step = max_samples.saturating_sub(overlap_samples).max(1);
    let mut ranges = Vec::new();
    let mut start = 0usize;
    while start < total_samples {
        let end = (start + max_samples).min(total_samples);
        ranges.push((start, end));
        if end >= total_samples {
            break;
        }
        start = start.saturating_add(step);
        // Guard against infinite loop if step somehow became 0 (defensive).
        if start >= total_samples {
            break;
        }
    }
    ranges
}

/// Plan non-overlapping ranges of at most `max_samples`, preferring to cut at
/// the quietest short window near each max boundary so words are less often
/// sliced mid-utterance.
pub fn plan_energy_aware_ranges(
    audio: &[f32],
    max_samples: usize,
    search_tail_samples: usize,
    min_segment_samples: usize,
) -> Vec<(usize, usize)> {
    let total = audio.len();
    if total == 0 {
        return Vec::new();
    }
    let max_samples = max_samples.max(1);
    if total <= max_samples {
        return vec![(0, total)];
    }

    let min_segment_samples = min_segment_samples.min(max_samples);
    let energy_window = samples_for_ms(ENERGY_WINDOW_MS).max(1);

    let mut ranges = Vec::new();
    let mut start = 0usize;

    while start < total {
        let remaining = total - start;
        if remaining <= max_samples {
            ranges.push((start, total));
            break;
        }

        let hard_end = start + max_samples;
        // Search for a quiet cut in [hard_end - search_tail, hard_end], but keep
        // the segment at least min_segment_samples long when possible.
        let earliest_cut = (start + min_segment_samples).min(hard_end);
        let search_from = hard_end.saturating_sub(search_tail_samples).max(earliest_cut);

        let cut = find_quietest_cut(audio, search_from, hard_end, energy_window)
            .unwrap_or(hard_end)
            .clamp(earliest_cut, hard_end);

        // Ensure forward progress even on degenerate silence/energy.
        let cut = if cut <= start {
            hard_end
        } else {
            cut
        };

        ranges.push((start, cut));
        start = cut;
    }

    ranges
}

/// Convenience planner with production defaults for a chosen max duration.
///
/// Cuts are energy-aware and non-overlapping first; then each subsequent
/// range starts [`DEFAULT_OVERLAP_MS`] earlier so boundary phonemes are
/// covered twice. [`join_segment_texts`] removes the duplicated text.
pub fn plan_ranges_for_max_ms(audio: &[f32], max_audio_ms: u64) -> Vec<(usize, usize)> {
    let max_samples = samples_for_ms(max_audio_ms.max(1_000));
    let search_tail = samples_for_ms(DEFAULT_BOUNDARY_SEARCH_MS);
    let min_seg = samples_for_ms(DEFAULT_MIN_SEGMENT_MS);
    let overlap = samples_for_ms(DEFAULT_OVERLAP_MS);
    let base = plan_energy_aware_ranges(audio, max_samples, search_tail, min_seg);
    apply_leading_overlap(base, overlap)
}

/// Expand non-overlapping cuts so segment *i>0* starts `overlap_samples`
/// before the previous cut (clamped to 0 / previous start+1).
pub fn apply_leading_overlap(
    ranges: Vec<(usize, usize)>,
    overlap_samples: usize,
) -> Vec<(usize, usize)> {
    if overlap_samples == 0 || ranges.len() <= 1 {
        return ranges;
    }
    let mut out = Vec::with_capacity(ranges.len());
    for (i, (start, end)) in ranges.iter().copied().enumerate() {
        if i == 0 {
            out.push((start, end));
            continue;
        }
        let prev_start = out[i - 1].0;
        let expanded = start.saturating_sub(overlap_samples);
        // Keep strict forward progress vs previous start; allow overlapping
        // the previous end so the boundary is double-covered.
        let new_start = expanded.max(prev_start.saturating_add(1)).min(start);
        out.push((new_start, end));
    }
    out
}

/// Join per-segment transcriptions. Latin-adjacent segments get a space;
/// CJK / other scripts are concatenated without an extra separator.
/// Adjacent segments that share a boundary phrase (from sample overlap) are
/// de-duplicated via longest common suffix/prefix match.
pub fn join_segment_texts(parts: &[String]) -> String {
    let mut out = String::new();
    for part in parts {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        if out.is_empty() {
            out.push_str(t);
            continue;
        }
        let trimmed_next = strip_boundary_overlap(&out, t, DEFAULT_DEDUP_CHARS);
        if trimmed_next.is_empty() {
            continue;
        }
        if needs_inter_segment_space(&out, trimmed_next) {
            out.push(' ');
        }
        out.push_str(trimmed_next);
    }
    out
}

/// If `prev` ends with a prefix of `next` (up to `max_chars`), return the
/// remainder of `next` after that shared span. Character-based (works for CJK).
pub fn strip_boundary_overlap<'a>(prev: &str, next: &'a str, max_chars: usize) -> &'a str {
    if prev.is_empty() || next.is_empty() || max_chars == 0 {
        return next;
    }
    let prev_chars: Vec<char> = prev.chars().collect();
    let next_chars: Vec<char> = next.chars().collect();
    let max_k = max_chars
        .min(prev_chars.len())
        .min(next_chars.len());
    if max_k == 0 {
        return next;
    }
    // Prefer longer matches; require at least 2 chars to avoid eating single
    // punctuation that legitimately repeats at a boundary.
    for k in (2..=max_k).rev() {
        if prev_chars[prev_chars.len() - k..] == next_chars[..k] {
            return slice_after_chars(next, k);
        }
    }
    next
}

/// Byte-offset slice of `s` after skipping `k` unicode scalars.
fn slice_after_chars(s: &str, k: usize) -> &str {
    match s.char_indices().nth(k) {
        Some((byte_idx, _)) => &s[byte_idx..],
        None => "",
    }
}

fn needs_inter_segment_space(left: &str, right: &str) -> bool {
    let Some(lc) = left.chars().rev().find(|c| !c.is_whitespace()) else {
        return false;
    };
    let Some(rc) = right.chars().find(|c| !c.is_whitespace()) else {
        return false;
    };
    is_latin_alnum(lc) && is_latin_alnum(rc)
}

fn is_latin_alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

/// Return the start index of the quietest `window_samples` block in `[from, to)`.
fn find_quietest_cut(
    audio: &[f32],
    from: usize,
    to: usize,
    window_samples: usize,
) -> Option<usize> {
    if to <= from || window_samples == 0 || to > audio.len() {
        return None;
    }
    if to - from < window_samples {
        return Some(from);
    }

    let mut best_start = from;
    let mut best_energy = f32::INFINITY;
    // Step by half-window for speed on long search tails.
    let step = (window_samples / 2).max(1);

    let mut start = from;
    while start + window_samples <= to {
        let end = start + window_samples;
        let energy = rms(&audio[start..end]);
        if energy < best_energy {
            best_energy = energy;
            // Cut at the middle of the quiet window.
            best_start = start + window_samples / 2;
        }
        start += step;
    }

    Some(best_start.clamp(from, to))
}

fn rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = frame.iter().map(|s| s * s).sum();
    (sum_sq / frame.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_audio_is_single_range() {
        let ranges = plan_fixed_ranges(16_000, 30 * 16_000, 0);
        assert_eq!(ranges, vec![(0, 16_000)]);
    }

    #[test]
    fn fixed_ranges_cover_full_audio_without_gaps_when_no_overlap() {
        let total = 100_000;
        let max = 30_000;
        let ranges = plan_fixed_ranges(total, max, 0);
        assert!(ranges.len() >= 4);
        assert_eq!(ranges.first().unwrap().0, 0);
        assert_eq!(ranges.last().unwrap().1, total);
        for w in ranges.windows(2) {
            assert_eq!(w[0].1, w[1].0);
        }
        for (s, e) in &ranges {
            assert!(e - s <= max);
            assert!(*e > *s);
        }
    }

    #[test]
    fn energy_aware_splits_long_tone() {
        // 90s of low-level noise at 16 kHz.
        let total = 90 * 16_000;
        let mut audio = vec![0.01f32; total];
        // Insert a quieter gap around 30s so the cut prefers it.
        let quiet_at = 30 * 16_000;
        for s in audio.iter_mut().skip(quiet_at).take(8_000) {
            *s = 0.0001;
        }
        let max = 35 * 16_000;
        let ranges = plan_energy_aware_ranges(&audio, max, 5 * 16_000, 5 * 16_000);
        assert!(ranges.len() >= 2);
        assert_eq!(ranges[0].0, 0);
        assert_eq!(ranges.last().unwrap().1, total);
        for w in ranges.windows(2) {
            assert_eq!(w[0].1, w[1].0);
        }
        // First cut should land near the quiet region, not forced at hard max only.
        let first_end = ranges[0].1;
        assert!(first_end > 25 * 16_000);
        assert!(first_end <= max);
    }

    #[test]
    fn plan_ranges_for_default_30s_on_1808s_scale() {
        // Matches the install-build failure sample length order of magnitude.
        let total = 1_808 * 16_000;
        let audio = vec![0.0f32; total];
        let ranges = plan_ranges_for_max_ms(&audio, DEFAULT_MAX_AUDIO_MS);
        assert!(ranges.len() >= 60);
        assert_eq!(ranges[0].0, 0);
        assert_eq!(ranges.last().unwrap().1, total);
        let max_samples = samples_for_ms(DEFAULT_MAX_AUDIO_MS);
        let max_with_overlap = max_samples + samples_for_ms(DEFAULT_OVERLAP_MS);
        for (s, e) in &ranges {
            assert!(e - s <= max_with_overlap);
        }
    }

    #[test]
    fn plan_ranges_for_xinyuan_road_fixture_duration() {
        // 代码研究/fixtures/xinyuan-road-9-16k.wav ≈ 4638.12s @ 16 kHz.
        // Full PCM allocation (~280MB f32) is avoided; length-only math + sparse probe.
        let total = 74_209_963usize; // frames from ffprobe conversion
        let max_samples = samples_for_ms(DEFAULT_MAX_AUDIO_MS);
        // Fixed non-overlap lower bound on segment count.
        let min_segments = total.div_ceil(max_samples);
        assert!(min_segments >= 155);

        // Energy planner with zeros: quiet everywhere, still covers full range.
        let audio = vec![0.0f32; total.min(max_samples * 4)]; // smoke on 4 windows
        let ranges = plan_ranges_for_max_ms(&audio, DEFAULT_MAX_AUDIO_MS);
        assert!(ranges.len() >= 4);
        assert_eq!(ranges[0].0, 0);
        assert_eq!(ranges.last().unwrap().1, audio.len());
        let max_with_overlap = max_samples + samples_for_ms(DEFAULT_OVERLAP_MS);
        for (s, e) in &ranges {
            assert!(e - s <= max_with_overlap);
        }
    }

    #[test]
    fn join_inserts_space_for_latin_only() {
        let parts = vec!["hello".into(), "world".into()];
        assert_eq!(join_segment_texts(&parts), "hello world");

        let cjk = vec!["你好".into(), "世界".into()];
        assert_eq!(join_segment_texts(&cjk), "你好世界");

        let mixed = vec!["hello".into(), "世界".into()];
        assert_eq!(join_segment_texts(&mixed), "hello世界");
    }

    #[test]
    fn join_skips_empty_parts() {
        let parts = vec!["a".into(), "  ".into(), "b".into()];
        assert_eq!(join_segment_texts(&parts), "a b");
    }

    #[test]
    fn apply_leading_overlap_expands_later_starts() {
        let base = vec![(0, 1000), (1000, 2000), (2000, 2500)];
        let overlapped = apply_leading_overlap(base, 100);
        assert_eq!(overlapped[0], (0, 1000));
        assert_eq!(overlapped[1], (900, 2000));
        assert_eq!(overlapped[2], (1900, 2500));
    }

    #[test]
    fn strip_boundary_overlap_removes_shared_cjk_tail() {
        let prev = "今天天气很好我们去公园";
        let next = "我们去公园散步";
        assert_eq!(strip_boundary_overlap(prev, next, 24), "散步");
    }

    #[test]
    fn strip_boundary_overlap_keeps_unrelated() {
        assert_eq!(strip_boundary_overlap("hello", "world", 24), "world");
        // Single-char match is ignored (min 2).
        assert_eq!(strip_boundary_overlap("ab", "bc", 24), "bc");
    }

    #[test]
    fn join_dedups_overlapping_boundary_text() {
        let parts = vec!["今天天气很好我们去公园".into(), "我们去公园散步".into()];
        assert_eq!(join_segment_texts(&parts), "今天天气很好我们去公园散步");
    }

    #[test]
    fn plan_ranges_for_max_ms_applies_overlap() {
        let total = 90 * 16_000;
        let audio = vec![0.01f32; total];
        let ranges = plan_ranges_for_max_ms(&audio, DEFAULT_MAX_AUDIO_MS);
        assert!(ranges.len() >= 2);
        // Adjacent ranges should overlap by roughly DEFAULT_OVERLAP_MS samples.
        let overlap = samples_for_ms(DEFAULT_OVERLAP_MS);
        for w in ranges.windows(2) {
            assert!(w[1].0 < w[0].1, "expected leading overlap between tiles");
            assert!(w[0].1.saturating_sub(w[1].0) <= overlap + 1);
        }
    }
}
