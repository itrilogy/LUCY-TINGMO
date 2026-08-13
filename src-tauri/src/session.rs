//! Lightweight recording session orchestrator (design M1).
//!
//! One active session owns the product mode, phase, and structured identity
//! for a single capture → ASR → result cycle. Managers remain infrastructure;
//! this module is the business-facing state center that `actions` / cancel
//! paths consult so mode decisions and logs stay consistent.

use crate::audio_toolkit::VadPolicy;
use crate::settings::TranscriptionMode;
use log::info;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Product-facing capture / ASR strategy for one session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    /// Stop recording first, then auto-split long audio and ASR.
    BatchAutoSplit,
    /// VAD segment ends enqueue live semi-realtime ASR.
    VadSegmented,
    /// Model-native streaming (when the selected model advertises it).
    NativeStream,
}

impl SessionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionMode::BatchAutoSplit => "batch_auto_split",
            SessionMode::VadSegmented => "vad_segmented",
            SessionMode::NativeStream => "native_stream",
        }
    }
}

/// Lifecycle phase of a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    Idle,
    Recording,
    Transcribing,
    PostProcessing,
    Result,
    Cancelled,
    Error,
}

impl SessionPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionPhase::Idle => "idle",
            SessionPhase::Recording => "recording",
            SessionPhase::Transcribing => "transcribing",
            SessionPhase::PostProcessing => "post_processing",
            SessionPhase::Result => "result",
            SessionPhase::Cancelled => "cancelled",
            SessionPhase::Error => "error",
        }
    }
}

/// Snapshot of the active (or last finished) session for logs / debug UI.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct SessionSnapshot {
    pub id: u64,
    pub mode: SessionMode,
    pub phase: SessionPhase,
    pub binding_id: String,
    pub model_id: String,
    pub post_process: bool,
    pub streaming_cap: bool,
    /// Elapsed ms since session begin (best-effort).
    pub elapsed_ms: u64,
}

/// Mutable active session.
#[derive(Debug)]
struct ActiveSession {
    id: u64,
    mode: SessionMode,
    phase: SessionPhase,
    binding_id: String,
    model_id: String,
    post_process: bool,
    streaming_cap: bool,
    started_at: Instant,
}

impl ActiveSession {
    fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            id: self.id,
            mode: self.mode,
            phase: self.phase,
            binding_id: self.binding_id.clone(),
            model_id: self.model_id.clone(),
            post_process: self.post_process,
            streaming_cap: self.streaming_cap,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
        }
    }
}

/// Parameters required to open a session at recording start.
#[derive(Clone, Debug)]
pub struct SessionBegin {
    pub binding_id: String,
    pub model_id: String,
    pub mode: SessionMode,
    pub post_process: bool,
    pub streaming_cap: bool,
}

/// App-global hub: at most one active session (matches single-pipeline UX).
pub struct SessionHub {
    active: Mutex<Option<ActiveSession>>,
}

impl SessionHub {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }

    /// Resolve product mode from settings + model streaming capability.
    ///
    /// `BatchAutoSplit` never auto-promotes to native stream (user must pick
    /// `NativeStream`). Unsupported `NativeStream` falls back to semi-realtime.
    pub fn resolve_mode(
        transcription_mode: TranscriptionMode,
        model_supports_streaming: bool,
    ) -> SessionMode {
        match transcription_mode {
            TranscriptionMode::VadSegmented => SessionMode::VadSegmented,
            TranscriptionMode::NativeStream if model_supports_streaming => {
                SessionMode::NativeStream
            }
            // User asked for native realtime but model cannot — use VAD semi-realtime.
            TranscriptionMode::NativeStream => SessionMode::VadSegmented,
            TranscriptionMode::BatchAutoSplit => SessionMode::BatchAutoSplit,
        }
    }

    /// VAD policy for capture given session mode and the user VAD toggle.
    pub fn vad_policy_for(mode: SessionMode, vad_enabled: bool) -> VadPolicy {
        match mode {
            SessionMode::VadSegmented => VadPolicy::Offline,
            SessionMode::NativeStream if vad_enabled => VadPolicy::Streaming,
            SessionMode::NativeStream => VadPolicy::Disabled,
            SessionMode::BatchAutoSplit if vad_enabled => VadPolicy::Offline,
            SessionMode::BatchAutoSplit => VadPolicy::Disabled,
        }
    }

    /// Whether this mode drives the app-level segment pipeline.
    pub fn uses_segment_pipeline(mode: SessionMode) -> bool {
        matches!(mode, SessionMode::VadSegmented)
    }

    /// Whether this mode should start model-native streaming.
    pub fn uses_native_stream(mode: SessionMode) -> bool {
        matches!(mode, SessionMode::NativeStream)
    }

    /// Begin a new session, replacing any stale active one.
    pub fn begin(&self, params: SessionBegin) -> SessionSnapshot {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let session = ActiveSession {
            id,
            mode: params.mode,
            phase: SessionPhase::Recording,
            binding_id: params.binding_id,
            model_id: params.model_id,
            post_process: params.post_process,
            streaming_cap: params.streaming_cap,
            started_at: Instant::now(),
        };
        let snap = session.snapshot();
        *self.active.lock().unwrap() = Some(session);
        info!(
            "session_id={} mode={} event=session_begin phase={} model='{}' binding={} streaming_cap={} post_process={}",
            snap.id,
            snap.mode.as_str(),
            snap.phase.as_str(),
            snap.model_id,
            snap.binding_id,
            snap.streaming_cap,
            snap.post_process
        );
        snap
    }

    pub fn active(&self) -> Option<SessionSnapshot> {
        self.active.lock().unwrap().as_ref().map(|s| s.snapshot())
    }

    pub fn set_phase(&self, phase: SessionPhase) -> Option<SessionSnapshot> {
        let mut guard = self.active.lock().unwrap();
        let session = guard.as_mut()?;
        session.phase = phase;
        let snap = session.snapshot();
        info!(
            "session_id={} mode={} event=session_phase phase={} elapsed_ms={}",
            snap.id,
            snap.mode.as_str(),
            snap.phase.as_str(),
            snap.elapsed_ms
        );
        Some(snap)
    }

    /// Mark cancelled and clear the active session.
    pub fn cancel_active(&self) -> Option<SessionSnapshot> {
        let mut guard = self.active.lock().unwrap();
        let Some(mut session) = guard.take() else {
            return None;
        };
        session.phase = SessionPhase::Cancelled;
        let snap = session.snapshot();
        info!(
            "session_id={} mode={} event=session_cancel elapsed_ms={}",
            snap.id,
            snap.mode.as_str(),
            snap.elapsed_ms
        );
        Some(snap)
    }

    /// Finish successfully (or with empty/error result) and clear.
    pub fn finish(&self, phase: SessionPhase) -> Option<SessionSnapshot> {
        let mut guard = self.active.lock().unwrap();
        let Some(mut session) = guard.take() else {
            return None;
        };
        session.phase = phase;
        let snap = session.snapshot();
        info!(
            "session_id={} mode={} event=session_finish phase={} elapsed_ms={}",
            snap.id,
            snap.mode.as_str(),
            snap.phase.as_str(),
            snap.elapsed_ms
        );
        Some(snap)
    }

    /// Drop a failed recording start without leaving a sticky session.
    pub fn abandon_begin(&self, id: u64) {
        let mut guard = self.active.lock().unwrap();
        if guard.as_ref().map(|s| s.id) == Some(id) {
            let snap = guard.take().map(|s| s.snapshot());
            if let Some(s) = snap {
                info!(
                    "session_id={} mode={} event=session_abandon",
                    s.id,
                    s.mode.as_str()
                );
            }
        }
    }
}

impl Default for SessionHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_mode_prefers_vad_segmented() {
        assert_eq!(
            SessionHub::resolve_mode(TranscriptionMode::VadSegmented, true),
            SessionMode::VadSegmented
        );
        assert_eq!(
            SessionHub::resolve_mode(TranscriptionMode::VadSegmented, false),
            SessionMode::VadSegmented
        );
    }

    #[test]
    fn resolve_mode_native_stream_only_when_selected_and_capable() {
        assert_eq!(
            SessionHub::resolve_mode(TranscriptionMode::BatchAutoSplit, true),
            SessionMode::BatchAutoSplit,
            "batch no longer auto-promotes to native stream"
        );
        assert_eq!(
            SessionHub::resolve_mode(TranscriptionMode::NativeStream, true),
            SessionMode::NativeStream
        );
        assert_eq!(
            SessionHub::resolve_mode(TranscriptionMode::NativeStream, false),
            SessionMode::VadSegmented,
            "unsupported native stream falls back to VAD semi-realtime"
        );
    }

    #[test]
    fn vad_policy_matrix() {
        assert_eq!(
            SessionHub::vad_policy_for(SessionMode::VadSegmented, false),
            VadPolicy::Offline
        );
        assert_eq!(
            SessionHub::vad_policy_for(SessionMode::NativeStream, true),
            VadPolicy::Streaming
        );
        assert_eq!(
            SessionHub::vad_policy_for(SessionMode::BatchAutoSplit, false),
            VadPolicy::Disabled
        );
    }

    #[test]
    fn hub_lifecycle_begin_transcribe_finish() {
        let hub = SessionHub::new();
        let s = hub.begin(SessionBegin {
            binding_id: "transcribe".into(),
            model_id: "test-model".into(),
            mode: SessionMode::BatchAutoSplit,
            post_process: false,
            streaming_cap: false,
        });
        assert_eq!(s.phase, SessionPhase::Recording);
        assert!(hub.active().is_some());
        hub.set_phase(SessionPhase::Transcribing);
        assert_eq!(
            hub.active().unwrap().phase,
            SessionPhase::Transcribing
        );
        let done = hub.finish(SessionPhase::Result).unwrap();
        assert_eq!(done.phase, SessionPhase::Result);
        assert!(hub.active().is_none());
    }

    #[test]
    fn cancel_clears_active() {
        let hub = SessionHub::new();
        hub.begin(SessionBegin {
            binding_id: "x".into(),
            model_id: "m".into(),
            mode: SessionMode::VadSegmented,
            post_process: true,
            streaming_cap: false,
        });
        let c = hub.cancel_active().unwrap();
        assert_eq!(c.phase, SessionPhase::Cancelled);
        assert!(hub.active().is_none());
    }
}
