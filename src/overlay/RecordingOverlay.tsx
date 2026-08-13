import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import "./RecordingOverlay.css";
import { commands, events } from "@/bindings";
import type {
  SessionProgressEvent,
  SessionResultEvent,
  StreamPhase,
  StreamPhaseEvent,
  StreamTextEvent,
  StreamWorkKind,
} from "@/bindings";
import i18n, { syncLanguageFromSettings } from "@/i18n";
import { getLanguageDirection } from "@/lib/utils/rtl";

type OverlayState =
  | "recording"
  | "streaming"
  | "transcribing"
  | "processing"
  | "result";

// Number of reactive bars in the waveform (the simple, smoothed style shared by
// every overlay form). Mic levels arrive as 16 FFT buckets; we take the first N.
const WAVE_BARS = 9;

const RecordingOverlay: React.FC = () => {
  const { t } = useTranslation();
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>("recording");
  const [levels, setLevels] = useState<number[]>(Array(WAVE_BARS).fill(0));
  const [streamText, setStreamText] = useState<StreamTextEvent>({
    committed: "",
    tentative: "",
  });
  const [phase, setPhase] = useState<StreamPhase>("listening");
  const [workKind, setWorkKind] = useState<StreamWorkKind>("transcribing");
  const [elapsed, setElapsed] = useState(0);
  const [session, setSession] = useState(0);
  const [position, setPosition] = useState<"top" | "bottom">("bottom");
  const [overflowing, setOverflowing] = useState(false);
  const [progress, setProgress] = useState<{
    current: number;
    total: number;
  } | null>(null);
  const [resultText, setResultText] = useState("");
  const [resultRaw, setResultRaw] = useState<string | null>(null);
  const [resultView, setResultView] = useState<"final" | "raw" | "both">(
    "final",
  );
  const [resultStatus, setResultStatus] = useState<"ok" | "empty" | "error">(
    "ok",
  );
  const [resultMessage, setResultMessage] = useState<string | null>(null);
  const [copyState, setCopyState] = useState<"idle" | "ok" | "err">("idle");
  /** Compact bar shows only the current segment; expand reveals full transcript. */
  const [liveExpanded, setLiveExpanded] = useState(false);

  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  const capRef = useRef<HTMLDivElement>(null);
  const pinnedRef = useRef(true);
  const direction = getLanguageDirection(i18n.language);

  useEffect(() => {
    const setupEventListeners = async () => {
      const unlistenShow = await listen("show-overlay", async (event) => {
        await syncLanguageFromSettings();
        try {
          const settings = await commands.getAppSettings();
          if (settings.status === "ok") {
            setPosition(
              settings.data.overlay_position === "top" ? "top" : "bottom",
            );
          }
        } catch {
          // Keep the previous/default placement if settings can't be read.
        }
        const overlayState = event.payload as OverlayState;
        setState(overlayState);
        if (overlayState === "recording" || overlayState === "streaming") {
          setStreamText({ committed: "", tentative: "" });
          setProgress(null);
          setResultText("");
          setResultRaw(null);
          setResultView("final");
          setResultStatus("ok");
          setResultMessage(null);
          setCopyState("idle");
          setLiveExpanded(false);
        }
        if (overlayState === "streaming") {
          setPhase("listening");
          setWorkKind("transcribing");
          setElapsed(0);
          setSession((s) => s + 1);
        }
        if (overlayState === "result") {
          setPhase("listening");
          setCopyState("idle");
          setSession((s) => s + 1);
        }
        setIsVisible(true);
      });

      const unlistenHide = await listen("hide-overlay", () => {
        setIsVisible(false);
      });

      const unlistenLevel = await listen<number[]>("mic-level", (event) => {
        const newLevels = event.payload as number[];
        const smoothed = smoothedLevelsRef.current.map((prev, i) => {
          const target = newLevels[i] || 0;
          return prev * 0.7 + target * 0.3;
        });
        smoothedLevelsRef.current = smoothed;
        setLevels(smoothed.slice(0, WAVE_BARS));
      });

      const unlistenStream = await events.streamTextEvent.listen((event) => {
        setStreamText(event.payload);
      });

      const unlistenPhase = await events.streamPhaseEvent.listen((event) => {
        const payload: StreamPhaseEvent = event.payload;
        setPhase(payload.phase);
        if (payload.kind) setWorkKind(payload.kind);
      });

      const unlistenProgress = await events.sessionProgressEvent.listen(
        (event) => {
          const payload: SessionProgressEvent = event.payload;
          setProgress({ current: payload.current, total: payload.total });
          if (payload.cumulative) {
            // Keep existing current-segment line (tentative) if StreamText
            // already set it; only fill committed from progress.
            setStreamText((prev) => ({
              committed: payload.cumulative,
              tentative: prev.tentative,
            }));
          }
          setState((prev) => (prev === "result" ? prev : "streaming"));
          setPhase("working");
          setWorkKind("transcribing");
          setIsVisible(true);
        },
      );

      const unlistenResult = await events.sessionResultEvent.listen((event) => {
        const payload: SessionResultEvent = event.payload;
        setResultText(payload.text ?? "");
        setResultRaw(payload.raw_text ?? null);
        setResultView("final");
        setResultStatus(payload.status ?? "ok");
        setResultMessage(payload.message ?? null);
        setStreamText({ committed: payload.text ?? "", tentative: "" });
        setProgress(null);
        setPhase("listening");
        setState("result");
        setCopyState("idle");
        setIsVisible(true);
      });

      return () => {
        unlistenShow();
        unlistenHide();
        unlistenLevel();
        unlistenStream();
        unlistenPhase();
        unlistenProgress();
        unlistenResult();
      };
    };

    setupEventListeners();
  }, []);

  useEffect(() => {
    if (state !== "streaming" || !isVisible) return;
    const id = setInterval(() => setElapsed((e) => e + 1), 1000);
    return () => clearInterval(id);
  }, [state, isVisible]);

  useLayoutEffect(() => {
    const el = capRef.current;
    if (!el) return;
    setOverflowing(el.scrollHeight > el.clientHeight + 1);
    if (pinnedRef.current) el.scrollTop = el.scrollHeight;
  }, [streamText, resultText, state]);

  useEffect(() => {
    pinnedRef.current = true;
    setOverflowing(false);
  }, [session]);

  const handleStreamScroll = () => {
    const el = capRef.current;
    if (!el) return;
    pinnedRef.current = el.scrollHeight - el.scrollTop - el.clientHeight <= 16;
  };

  const fmtTime = (s: number) =>
    `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;

  const handleCopy = async () => {
    const text = resultText || streamText.committed;
    if (!text.trim()) return;
    try {
      const res = await commands.copyTextToClipboard(text);
      if (res.status === "ok") {
        setCopyState("ok");
        setTimeout(() => setCopyState("idle"), 1500);
      } else {
        setCopyState("err");
      }
    } catch {
      setCopyState("err");
    }
  };

  const handlePasteFront = async () => {
    const text = resultText || streamText.committed;
    if (!text.trim()) return;
    try {
      const res = await commands.pasteTextToFrontmost(text);
      if (res.status === "ok") {
        setCopyState("ok");
        setTimeout(() => setCopyState("idle"), 1500);
      } else {
        setCopyState("err");
      }
    } catch {
      setCopyState("err");
    }
  };

  const handleDismiss = () => {
    void commands.dismissOverlay();
  };

  const waveform = (
    <div className="swave">
      {levels.map((v, i) => (
        <i
          key={i}
          style={{
            height: `${Math.max(3, Math.min(18, 3 + Math.pow(v, 0.7) * 15))}px`,
          }}
        />
      ))}
    </div>
  );

  const cancelBtn = (
    <button
      className="sx"
      aria-label="cancel"
      onClick={() => commands.cancelOperation()}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M4 4 L12 12 M12 4 L4 12"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
        />
      </svg>
    </button>
  );

  const listeningRow = (showTimer: boolean, showCancel: boolean) => (
    <div className="sbase">
      <div className="sbase-l">
        <span className="sdot" />
      </div>
      {waveform}
      <div className="sbase-r">
        {showTimer && <span className="stimer">{fmtTime(elapsed)}</span>}
        {showCancel && cancelBtn}
      </div>
    </div>
  );

  const workingRow = (label: string, showCancel: boolean) => (
    <div className="sbase">
      <div className="sbase-l">
        <span className="sspinner" />
      </div>
      <span className="swork-label">{label}</span>
      <div className="sbase-r">{showCancel && cancelBtn}</div>
    </div>
  );

  const workLabel = () => {
    if (progress && progress.current > 0) {
      // Live VAD mode reports total==current as segments complete.
      if (progress.total > progress.current) {
        return t("overlay.segmentProgress", {
          current: progress.current,
          total: progress.total,
        });
      }
      return t("overlay.segmentLive", { current: progress.current });
    }
    if (workKind === "polishing") return t("overlay.processing");
    return t("overlay.transcribing");
  };

  // ---- Result panel: expanded transcript + Copy (no auto-paste) ----
  if (state === "result") {
    const text = resultText || streamText.committed;
    const isEmpty = resultStatus === "empty" || !text.trim();
    const isError = resultStatus === "error";
    const canAct = !isEmpty && !isError && !!text.trim();
    const hasRaw = !!(resultRaw && resultRaw.trim() && resultRaw !== text);
    const displayText = isError
      ? t("overlay.errorResult", {
          message: resultMessage || t("overlay.errorUnknown"),
        })
      : isEmpty
        ? t("overlay.emptyResult")
        : text;
    return (
      <div dir={direction} className={`ov-stage ${position}`}>
        <div
          key={session}
          className={`scard open result ${isVisible ? "" : "leaving"} ${
            isError ? "result-error" : isEmpty ? "result-empty" : ""
          }`}
        >
          {hasRaw && canAct ? (
            <div className="sview-tabs" role="tablist">
              {(
                [
                  ["final", t("overlay.viewFinal")],
                  ["raw", t("overlay.viewRaw")],
                  ["both", t("overlay.viewBoth")],
                ] as const
              ).map(([id, label]) => (
                <button
                  key={id}
                  type="button"
                  role="tab"
                  className={`sview-tab ${resultView === id ? "active" : ""}`}
                  aria-selected={resultView === id}
                  onClick={() => setResultView(id)}
                >
                  {label}
                </button>
              ))}
            </div>
          ) : null}
          <div className="stext stext-result">
            <div className="stext-clip">
              <div
                className={`stext-cap result-cap ${overflowing ? "overflowing" : ""}`}
                ref={capRef}
                onScroll={handleStreamScroll}
              >
                {isError || isEmpty ? (
                  <p className="pre-wrap">
                    <span className="committed muted">{displayText}</span>
                  </p>
                ) : resultView === "raw" && hasRaw ? (
                  <p className="pre-wrap">
                    <span className="committed muted">{resultRaw}</span>
                  </p>
                ) : resultView === "both" && hasRaw ? (
                  <div className="scompare">
                    <div>
                      <div className="scompare-label">
                        {t("overlay.viewRaw")}
                      </div>
                      <p className="pre-wrap">
                        <span className="committed muted">{resultRaw}</span>
                      </p>
                    </div>
                    <div>
                      <div className="scompare-label">
                        {t("overlay.viewFinal")}
                      </div>
                      <p className="pre-wrap">
                        <span className="committed">{text}</span>
                      </p>
                    </div>
                  </div>
                ) : (
                  <p className="pre-wrap">
                    <span className="committed">{text}</span>
                  </p>
                )}
              </div>
            </div>
          </div>
          <div
            className={[
              "result-bar",
              canAct ? "result-bar-actions" : "result-bar-dismiss-only",
            ].join(" ")}
          >
            {canAct ? (
              <>
                <button
                  className="sbtn primary"
                  type="button"
                  onClick={handleCopy}
                >
                  {copyState === "ok"
                    ? t("overlay.copied")
                    : copyState === "err"
                      ? t("overlay.copyFailed")
                      : t("overlay.copy")}
                </button>
                <button
                  className="sbtn"
                  type="button"
                  onClick={handlePasteFront}
                >
                  {t("overlay.pasteFront")}
                </button>
                <button className="sbtn" type="button" onClick={handleDismiss}>
                  {t("overlay.dismiss")}
                </button>
              </>
            ) : (
              <button
                className="sbtn result-dismiss"
                type="button"
                onClick={handleDismiss}
              >
                <svg
                  className="result-dismiss-icon"
                  viewBox="0 0 16 16"
                  aria-hidden="true"
                >
                  <path
                    d="M4 4 L12 12 M12 4 L4 12"
                    stroke="currentColor"
                    strokeWidth="1.6"
                    strokeLinecap="round"
                    fill="none"
                  />
                </svg>
                <span>{t("overlay.dismiss")}</span>
              </button>
            )}
          </div>
        </div>
      </div>
    );
  }

  // ---- Live / progress overlay ----
  if (state === "streaming") {
    const currentLine =
      streamText.tentative.trim() ||
      // Fallback: last slice of committed when tentative empty
      streamText.committed.trim();
    const hasText =
      streamText.committed.length > 0 || streamText.tentative.length > 0;
    const working = phase === "working";
    const open = hasText;
    const collapsed = working && !hasText;
    const showFull = liveExpanded && hasText;

    return (
      <div dir={direction} className={`ov-stage ${position}`}>
        <div
          key={session}
          className={[
            "scard",
            open || showFull ? "open" : "",
            // Same outer size as end-of-recording result panel.
            showFull ? "live-expanded" : "",
            collapsed ? "working" : "",
            isVisible ? "" : "leaving",
          ]
            .filter(Boolean)
            .join(" ")}
        >
          <div className="stext">
            <div className="stext-clip">
              <div
                className={`stext-cap ${showFull ? "live-full" : "live-current"} ${
                  overflowing ? "overflowing" : ""
                }`}
                ref={capRef}
                onScroll={handleStreamScroll}
              >
                {showFull ? (
                  <p className="pre-wrap">
                    <span className="committed">{streamText.committed}</span>
                    {!working && <span className="scaret" />}
                  </p>
                ) : (
                  <p>
                    <span className="committed current-only">
                      {currentLine || (working ? "" : "…")}
                    </span>
                    {!working && hasText && <span className="scaret" />}
                  </p>
                )}
              </div>
            </div>
          </div>
          {/*
            Idle (no speech yet): classic pill — [dot | wave | cancel], symmetric.
            With text / working: [expand?] · [status] · [cancel at right edge].
          */}
          <div
            className={[
              "live-bar",
              !hasText && !working ? "live-bar-idle" : "live-bar-active",
            ].join(" ")}
          >
            {!hasText && !working ? (
              <>
                <div className="live-side live-side-l">
                  <span className="sdot" />
                </div>
                <div className="live-center">{waveform}</div>
                <div className="live-side live-side-r">{cancelBtn}</div>
              </>
            ) : (
              <>
                <div className="live-side live-side-l">
                  {hasText ? (
                    <button
                      className="sbtn"
                      type="button"
                      onClick={() => setLiveExpanded((v) => !v)}
                    >
                      {liveExpanded
                        ? t("overlay.collapse")
                        : t("overlay.expand")}
                    </button>
                  ) : null}
                </div>
                <div className="live-center">
                  {working ? (
                    <>
                      <span className="sspinner" />
                      <span className="swork-label">{workLabel()}</span>
                    </>
                  ) : (
                    <>
                      <span className="sdot" />
                      {waveform}
                      <span className="stimer live-timer">
                        {fmtTime(elapsed)}
                      </span>
                    </>
                  )}
                </div>
                <div className="live-side live-side-r">{cancelBtn}</div>
              </>
            )}
          </div>
        </div>
      </div>
    );
  }

  // ---- Minimal overlay ----
  const working = state === "transcribing" || state === "processing";
  const compactLabel =
    state === "processing"
      ? t("overlay.processing")
      : progress && progress.total > 1
        ? t("overlay.segmentProgress", {
            current: progress.current,
            total: progress.total,
          })
        : t("overlay.transcribing");

  return (
    <div
      dir={direction}
      className={`ov-stage ${position} ov-fade ${isVisible ? "show" : ""}`}
    >
      <div
        className={`scard compact ${working && isVisible ? "cworking" : ""}`}
      >
        {working ? workingRow(compactLabel, true) : listeningRow(false, true)}
      </div>
    </div>
  );
};

export default RecordingOverlay;
