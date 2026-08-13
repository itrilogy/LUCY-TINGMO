//! Live VAD-segmented semi-realtime transcription.
//!
//! While recording, speech segments are queued and transcribed one-by-one.
//! L1 pause punctuation is applied per completed segment; L2 local punc may
//! refine the *completed* segment display asynchronously when enabled.
//! Compact overlay shows the current segment; expanded view uses cumulative.

use crate::audio_toolkit::join_segment_texts;
use crate::managers::transcription::{
    SessionProgressEvent, StreamTextEvent, TranscribeCallOptions, TranscriptionManager,
};
use crate::settings::{get_settings, TextPunctuationMode};
use crate::text_punctuate::{join_display_segments, punctuate_segment};
use log::{debug, error, info, warn};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tauri_specta::Event;

struct SegmentJob {
    pcm: Vec<f32>,
    silence_ms: u64,
    force_cut: bool,
}

enum SegmentCmd {
    Segment(SegmentJob),
    /// Wait for all queued segments, then reply with joined display text.
    Finalize(Sender<String>),
    Cancel,
}

/// Serial segment ASR worker for one recording session.
pub struct SegmentPipeline {
    tx: Mutex<Option<Sender<SegmentCmd>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    accepted: Arc<AtomicU32>,
    session_id: AtomicU64,
}

impl SegmentPipeline {
    pub fn new() -> Self {
        Self {
            tx: Mutex::new(None),
            worker: Mutex::new(None),
            accepted: Arc::new(AtomicU32::new(0)),
            session_id: AtomicU64::new(0),
        }
    }

    pub fn is_active(&self) -> bool {
        self.tx.lock().unwrap().is_some()
    }

    pub fn start(&self, app: AppHandle, transcription: Arc<TranscriptionManager>) {
        let mut tx_guard = self.tx.lock().unwrap();
        if tx_guard.is_some() {
            warn!("SegmentPipeline::start called while already active");
            return;
        }
        self.accepted.store(0, Ordering::Relaxed);
        let sid = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.session_id.store(sid, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel::<SegmentCmd>();
        *tx_guard = Some(tx);
        drop(tx_guard);

        let accepted = Arc::clone(&self.accepted);
        let handle = thread::spawn(move || {
            segment_worker(rx, app, transcription, accepted, sid);
        });
        *self.worker.lock().unwrap() = Some(handle);
        info!(
            "session_id={} mode=vad_segmented event=pipeline_start",
            sid
        );
    }

    pub fn push_segment(&self, pcm: Vec<f32>, silence_ms: u64, force_cut: bool) {
        if pcm.is_empty() {
            return;
        }
        let n = self.accepted.fetch_add(1, Ordering::Relaxed) + 1;
        let sid = self.session_id.load(Ordering::Relaxed);

        if let Some(tx) = self.tx.lock().unwrap().as_ref() {
            debug!(
                "session_id={} mode=vad_segmented event=enqueue segment={} samples={} secs={:.2} silence_ms={} force_cut={}",
                sid,
                n,
                pcm.len(),
                pcm.len() as f64 / 16_000.0,
                silence_ms,
                force_cut
            );
            if tx
                .send(SegmentCmd::Segment(SegmentJob {
                    pcm,
                    silence_ms,
                    force_cut,
                }))
                .is_err()
            {
                warn!(
                    "session_id={} mode=vad_segmented event=enqueue_fail segment={}",
                    sid, n
                );
            }
        }
    }

    pub fn finalize(&self) -> String {
        let sid = self.session_id.load(Ordering::Relaxed);
        let accepted = self.accepted.load(Ordering::Relaxed);
        let tx = self.tx.lock().unwrap().take();
        let Some(tx) = tx else {
            return String::new();
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        if tx.send(SegmentCmd::Finalize(reply_tx)).is_err() {
            warn!(
                "session_id={} mode=vad_segmented event=finalize_send_fail",
                sid
            );
            self.join_worker();
            return String::new();
        }
        drop(tx);
        let text = reply_rx.recv().unwrap_or_default();
        self.join_worker();
        info!(
            "session_id={} mode=vad_segmented event=pipeline_finalize segments={} result_chars={}",
            sid,
            accepted,
            text.chars().count()
        );
        text
    }

    pub fn cancel(&self) {
        let sid = self.session_id.load(Ordering::Relaxed);
        if let Some(tx) = self.tx.lock().unwrap().take() {
            let _ = tx.send(SegmentCmd::Cancel);
        }
        self.join_worker();
        self.accepted.store(0, Ordering::Relaxed);
        info!(
            "session_id={} mode=vad_segmented event=pipeline_cancel",
            sid
        );
    }

    fn join_worker(&self) {
        if let Some(h) = self.worker.lock().unwrap().take() {
            let _ = h.join();
        }
    }
}

impl Default for SegmentPipeline {
    fn default() -> Self {
        Self::new()
    }
}

fn run_one_segment(
    transcription: &TranscriptionManager,
    job: SegmentJob,
    prev_context: &mut Option<String>,
    display_parts: &mut Vec<String>,
    raw_parts: &mut Vec<String>,
    done: &mut u32,
    accepted: &AtomicU32,
    app: &AppHandle,
    session_id: u64,
    punct_mode: TextPunctuationMode,
) {
    let opts = TranscribeCallOptions {
        unload_after: false,
        context_prompt: prev_context.clone(),
    };
    match transcription.transcribe_with(job.pcm, opts) {
        Ok(text) => {
            let raw = text.trim().to_string();
            if !raw.is_empty() {
                *prev_context = Some(raw.clone());
                raw_parts.push(raw.clone());
                // L1 (+ L2 if mode and model registered) on this completed segment.
                let seg = punctuate_segment(&raw, job.silence_ms, job.force_cut, punct_mode);
                display_parts.push(seg.display.clone());
                *done = done.saturating_add(1);
                let total = accepted.load(Ordering::Relaxed).max(*done);
                let cumulative = join_display_segments(display_parts);
                let _ = SessionProgressEvent {
                    current: *done,
                    total,
                    cumulative: cumulative.clone(),
                }
                .emit(app);
                // Compact overlay uses `tentative` as the *current segment* line.
                let _ = StreamTextEvent {
                    committed: cumulative,
                    tentative: seg.display,
                }
                .emit(app);
                debug!(
                    "session_id={} mode=vad_segmented event=segment_done current={} total={} silence_ms={} force_cut={} chars={}",
                    session_id,
                    *done,
                    total,
                    job.silence_ms,
                    job.force_cut,
                    raw.chars().count()
                );
            } else {
                *done = done.saturating_add(1);
            }
        }
        Err(e) => {
            error!(
                "session_id={} mode=vad_segmented event=segment_error err={}",
                session_id, e
            );
            *done = done.saturating_add(1);
            let total = accepted.load(Ordering::Relaxed).max(*done);
            let cumulative = join_display_segments(display_parts);
            let _ = SessionProgressEvent {
                current: *done,
                total,
                cumulative: cumulative.clone(),
            }
            .emit(app);
        }
    }
}

fn segment_worker(
    rx: Receiver<SegmentCmd>,
    app: AppHandle,
    transcription: Arc<TranscriptionManager>,
    accepted: Arc<AtomicU32>,
    session_id: u64,
) {
    let mut display_parts: Vec<String> = Vec::new();
    let mut raw_parts: Vec<String> = Vec::new();
    let mut done: u32 = 0;
    let mut prev_context: Option<String> = None;
    let punct_mode = get_settings(&app).text_punctuation_mode;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            SegmentCmd::Segment(job) => {
                run_one_segment(
                    &transcription,
                    job,
                    &mut prev_context,
                    &mut display_parts,
                    &mut raw_parts,
                    &mut done,
                    &accepted,
                    &app,
                    session_id,
                    punct_mode,
                );
            }
            SegmentCmd::Finalize(reply) => {
                while let Ok(more) = rx.try_recv() {
                    match more {
                        SegmentCmd::Segment(job) => {
                            run_one_segment(
                                &transcription,
                                job,
                                &mut prev_context,
                                &mut display_parts,
                                &mut raw_parts,
                                &mut done,
                                &accepted,
                                &app,
                                session_id,
                                punct_mode,
                            );
                        }
                        SegmentCmd::Cancel => {
                            let _ = reply.send(String::new());
                            transcription.maybe_unload_immediately("segment pipeline cancel");
                            return;
                        }
                        SegmentCmd::Finalize(_) => {}
                    }
                }
                let final_text = if display_parts.is_empty() {
                    join_segment_texts(&raw_parts)
                } else {
                    join_display_segments(&display_parts)
                };
                transcription.maybe_unload_immediately("segment pipeline finalize");
                let _ = reply.send(final_text);
                return;
            }
            SegmentCmd::Cancel => {
                transcription.maybe_unload_immediately("segment pipeline cancel");
                return;
            }
        }
    }
}
