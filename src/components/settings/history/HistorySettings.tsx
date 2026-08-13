import React, { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { readFile } from "@tauri-apps/plugin-fs";
import {
  Check,
  ChevronDown,
  ChevronRight,
  ChevronsDownUp,
  ChevronsUpDown,
  Copy,
  FolderOpen,
  Pencil,
  RotateCcw,
  Star,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  commands,
  events,
  type HistoryEntry,
  type HistoryUpdatePayload,
} from "@/bindings";
import { useOsType } from "@/hooks/useOsType";
import { formatDateTime } from "@/utils/dateFormat";
import { AudioPlayer, AudioPlayerGroup } from "../../ui/AudioPlayer";
import { Button } from "../../ui/Button";

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer disabled:cursor-not-allowed disabled:text-text/20 ${
      active
        ? "text-logo-primary hover:text-logo-primary/80"
        : "text-text/50 hover:text-logo-primary"
    }`}
    title={title}
  >
    {children}
  </button>
);

const PAGE_SIZE = 30;

interface OpenRecordingsButtonProps {
  onClick: () => void;
  label: string;
}

const OpenRecordingsButton: React.FC<OpenRecordingsButtonProps> = ({
  onClick,
  label,
}) => (
  <Button
    onClick={onClick}
    variant="secondary"
    size="sm"
    className="flex items-center gap-2"
    title={label}
  >
    <FolderOpen className="w-4 h-4" />
    <span>{label}</span>
  </Button>
);

export const HistorySettings: React.FC = () => {
  const { t } = useTranslation();
  const osType = useOsType();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasMore, setHasMore] = useState(true);
  /** Expanded entry ids; first item defaults expanded, rest collapsed. */
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());
  const expandedInitRef = useRef(false);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const entriesRef = useRef<HistoryEntry[]>([]);
  const loadingRef = useRef(false);

  // Keep ref in sync for use in IntersectionObserver callback
  useEffect(() => {
    entriesRef.current = entries;
  }, [entries]);

  const loadPage = useCallback(async (cursor?: number) => {
    const isFirstPage = cursor === undefined;
    if (!isFirstPage && loadingRef.current) return;
    loadingRef.current = true;

    if (isFirstPage) setLoading(true);

    try {
      const result = await commands.getHistoryEntries(
        cursor ?? null,
        PAGE_SIZE,
      );
      if (result.status === "ok") {
        const { entries: newEntries, has_more } = result.data;
        setEntries((prev) =>
          isFirstPage ? newEntries : [...prev, ...newEntries],
        );
        setHasMore(has_more);
        // First load: only the newest entry expanded.
        if (isFirstPage && newEntries.length > 0 && !expandedInitRef.current) {
          expandedInitRef.current = true;
          setExpandedIds(new Set([newEntries[0].id]));
        }
      }
    } catch (error) {
      console.error("Failed to load history entries:", error);
    } finally {
      setLoading(false);
      loadingRef.current = false;
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadPage();
  }, [loadPage]);

  // Infinite scroll via IntersectionObserver
  useEffect(() => {
    if (loading) return;

    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;

    const observer = new IntersectionObserver(
      (observerEntries) => {
        const first = observerEntries[0];
        if (first.isIntersecting) {
          const lastEntry = entriesRef.current[entriesRef.current.length - 1];
          if (lastEntry) {
            loadPage(lastEntry.id);
          }
        }
      },
      { threshold: 0 },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loading, hasMore, loadPage]);

  // Listen for new entries added from the transcription pipeline
  useEffect(() => {
    const unlisten = events.historyUpdatePayload.listen((event) => {
      const payload: HistoryUpdatePayload = event.payload;
      if (payload.action === "added") {
        setEntries((prev) => [payload.entry, ...prev]);
        // New recording becomes the expanded "first" entry.
        setExpandedIds((prev) => {
          const next = new Set(prev);
          next.add(payload.entry.id);
          return next;
        });
      } else if (payload.action === "updated") {
        setEntries((prev) =>
          prev.map((e) => (e.id === payload.entry.id ? payload.entry : e)),
        );
      }
      // "deleted" and "toggled" are handled by optimistic updates only,
      // so we intentionally ignore them here to avoid double-mutation.
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggleSaved = async (id: number) => {
    // Optimistic update
    setEntries((prev) =>
      prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
    );
    try {
      const result = await commands.toggleHistoryEntrySaved(id);
      if (result.status !== "ok") {
        // Revert on failure
        setEntries((prev) =>
          prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
        );
      }
    } catch (error) {
      console.error("Failed to toggle saved status:", error);
      // Revert on failure
      setEntries((prev) =>
        prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
      );
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  };

  const getAudioUrl = useCallback(
    async (fileName: string) => {
      try {
        const result = await commands.getAudioFilePath(fileName);
        if (result.status === "ok") {
          if (osType === "linux") {
            const fileData = await readFile(result.data);
            const blob = new Blob([fileData], { type: "audio/wav" });
            return URL.createObjectURL(blob);
          }
          return convertFileSrc(result.data, "asset");
        }
        return null;
      } catch (error) {
        console.error("Failed to get audio file path:", error);
        return null;
      }
    },
    [osType],
  );

  const deleteAudioEntry = async (id: number) => {
    // Optimistically remove
    setEntries((prev) => prev.filter((e) => e.id !== id));
    try {
      const result = await commands.deleteHistoryEntry(id);
      if (result.status !== "ok") {
        // Reload on failure
        loadPage();
      }
    } catch (error) {
      console.error("Failed to delete entry:", error);
      loadPage();
    }
  };

  const retryHistoryEntry = async (id: number) => {
    const result = await commands.retryHistoryEntryTranscription(id);
    if (result.status !== "ok") {
      throw new Error(String(result.error));
    }
  };

  const openRecordingsFolder = async () => {
    try {
      const result = await commands.openRecordingsFolder();
      if (result.status !== "ok") {
        throw new Error(String(result.error));
      }
    } catch (error) {
      console.error("Failed to open recordings folder:", error);
    }
  };

  const toggleExpanded = (id: number) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const expandAll = () => {
    setExpandedIds(new Set(entries.map((e) => e.id)));
  };

  const collapseAll = () => {
    setExpandedIds(new Set());
  };

  const renameEntry = async (id: number, title: string) => {
    const result = await commands.updateHistoryEntryTitle(id, title);
    if (result.status !== "ok") {
      throw new Error(String(result.error));
    }
    setEntries((prev) =>
      prev.map((e) => (e.id === id ? result.data : e)),
    );
  };

  let content: React.ReactNode;

  if (loading) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {t("settings.history.loading")}
      </div>
    );
  } else if (entries.length === 0) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {t("settings.history.empty")}
      </div>
    );
  } else {
    content = (
      <>
        <AudioPlayerGroup>
          <div className="divide-y divide-mid-gray/20">
            {entries.map((entry) => (
              <HistoryEntryComponent
                key={entry.id}
                entry={entry}
                expanded={expandedIds.has(entry.id)}
                onToggleExpand={() => toggleExpanded(entry.id)}
                onToggleSaved={() => toggleSaved(entry.id)}
                onCopyText={() => copyToClipboard(entry.transcription_text)}
                onRename={(title) => renameEntry(entry.id, title)}
                getAudioUrl={getAudioUrl}
                deleteAudio={deleteAudioEntry}
                retryTranscription={retryHistoryEntry}
              />
            ))}
          </div>
        </AudioPlayerGroup>
        {/* Sentinel for infinite scroll */}
        <div ref={sentinelRef} className="h-1" />
      </>
    );
  }

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="space-y-2">
        <div className="px-4 flex items-center justify-between gap-2 flex-wrap">
          <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide">
            {t("settings.history.title")}
          </h2>
          <div className="flex items-center gap-1.5 flex-wrap">
            {entries.length > 0 && (
              <>
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  className="flex items-center gap-1"
                  onClick={expandAll}
                  title={t("settings.history.expandAll")}
                >
                  <ChevronsUpDown className="w-3.5 h-3.5" />
                  <span className="text-xs">{t("settings.history.expandAll")}</span>
                </Button>
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  className="flex items-center gap-1"
                  onClick={collapseAll}
                  title={t("settings.history.collapseAll")}
                >
                  <ChevronsDownUp className="w-3.5 h-3.5" />
                  <span className="text-xs">{t("settings.history.collapseAll")}</span>
                </Button>
              </>
            )}
            <OpenRecordingsButton
              onClick={openRecordingsFolder}
              label={t("settings.history.openFolder")}
            />
          </div>
        </div>
        <div className="bg-background border border-mid-gray/20 rounded-lg overflow-visible">
          {content}
        </div>
      </div>
    </div>
  );
};

interface HistoryEntryProps {
  entry: HistoryEntry;
  expanded: boolean;
  onToggleExpand: () => void;
  onToggleSaved: () => void;
  onCopyText: () => void;
  onRename: (title: string) => Promise<void>;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
  retryTranscription: (id: number) => Promise<void>;
}

const HistoryEntryComponent: React.FC<HistoryEntryProps> = ({
  entry,
  expanded,
  onToggleExpand,
  onToggleSaved,
  onCopyText,
  onRename,
  getAudioUrl,
  deleteAudio,
  retryTranscription,
}) => {
  const { t, i18n } = useTranslation();
  const [showCopied, setShowCopied] = useState(false);
  const [retrying, setRetrying] = useState(false);
  const [wrongWord, setWrongWord] = useState("");
  const [correctWord, setCorrectWord] = useState("");
  const [savingMarker, setSavingMarker] = useState(false);
  const [textView, setTextView] = useState<"final" | "raw" | "both">("final");
  const [renaming, setRenaming] = useState(false);
  const [renameDraft, setRenameDraft] = useState(entry.title);
  const [renamingBusy, setRenamingBusy] = useState(false);

  const hasTranscription = entry.transcription_text.trim().length > 0;
  const postText = entry.post_processed_text?.trim() || "";
  const hasCompare =
    !!postText && postText !== entry.transcription_text.trim();
  const displayPrimary = hasCompare ? postText : entry.transcription_text;
  const formattedDate = formatDateTime(String(entry.timestamp), i18n.language);
  const displayTitle = entry.title?.trim() || formattedDate;
  const previewText = (displayPrimary || "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 80);

  const handleCorrectWord = async () => {
    if (!wrongWord.trim() || !correctWord.trim()) return;
    setSavingMarker(true);
    try {
      const res = await commands.correctAsrWord(
        wrongWord.trim(),
        correctWord.trim(),
        "history",
      );
      if (res.status === "ok") {
        toast.success(t("settings.history.correctWordSaved"));
        setWrongWord("");
        setCorrectWord("");
      } else {
        toast.error(String(res.error));
      }
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSavingMarker(false);
    }
  };

  const handleLoadAudio = useCallback(
    () => getAudioUrl(entry.file_name),
    [getAudioUrl, entry.file_name],
  );

  const handleCopyText = () => {
    if (!hasTranscription && !postText) {
      return;
    }

    const toCopy =
      textView === "raw" && hasCompare
        ? entry.transcription_text
        : displayPrimary;
    void navigator.clipboard.writeText(toCopy).catch(() => onCopyText());
    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  const handleDeleteEntry = async () => {
    try {
      await deleteAudio(entry.id);
    } catch (error) {
      console.error("Failed to delete entry:", error);
      toast.error(t("settings.history.deleteError"));
    }
  };

  const handleRetranscribe = async () => {
    try {
      setRetrying(true);
      await retryTranscription(entry.id);
    } catch (error) {
      console.error("Failed to re-transcribe:", error);
      toast.error(t("settings.history.retranscribeError"));
    } finally {
      setRetrying(false);
    }
  };

  const [postProcessing, setPostProcessing] = useState(false);
  const handlePostProcess = async () => {
    try {
      setPostProcessing(true);
      const res = await commands.postProcessHistoryEntry(entry.id);
      if (res.status === "ok") {
        toast.success(t("settings.history.postProcessDone"));
      } else {
        toast.error(String(res.error));
      }
    } catch (error) {
      console.error("Failed to post-process:", error);
      toast.error(t("settings.history.postProcessError"));
    } finally {
      setPostProcessing(false);
    }
  };

  const startRename = () => {
    setRenameDraft(entry.title || formattedDate);
    setRenaming(true);
  };

  const commitRename = async () => {
    const next = renameDraft.trim();
    if (!next) {
      toast.error(t("settings.history.renameEmpty"));
      return;
    }
    if (next === entry.title) {
      setRenaming(false);
      return;
    }
    setRenamingBusy(true);
    try {
      await onRename(next);
      setRenaming(false);
      toast.success(t("settings.history.renameDone"));
    } catch (e) {
      toast.error(String(e));
    } finally {
      setRenamingBusy(false);
    }
  };

  const actionButtons = (
    <div className="flex items-center shrink-0">
      <IconButton
        onClick={startRename}
        disabled={retrying || renaming}
        title={t("settings.history.rename")}
      >
        <Pencil width={15} height={15} />
      </IconButton>
      <IconButton
        onClick={handleCopyText}
        disabled={!hasTranscription || retrying}
        title={t("settings.history.copyToClipboard")}
      >
        {showCopied ? (
          <Check width={16} height={16} />
        ) : (
          <Copy width={16} height={16} />
        )}
      </IconButton>
      <IconButton
        onClick={onToggleSaved}
        disabled={retrying}
        active={entry.saved}
        title={
          entry.saved
            ? t("settings.history.unsave")
            : t("settings.history.save")
        }
      >
        <Star
          width={16}
          height={16}
          fill={entry.saved ? "currentColor" : "none"}
        />
      </IconButton>
      <IconButton
        onClick={handleRetranscribe}
        disabled={retrying || postProcessing}
        title={t("settings.history.retranscribe")}
      >
        <RotateCcw
          width={16}
          height={16}
          style={
            retrying
              ? { animation: "spin 1s linear infinite reverse" }
              : undefined
          }
        />
      </IconButton>
      <IconButton
        onClick={() => void handlePostProcess()}
        disabled={
          retrying || postProcessing || (!hasTranscription && !postText)
        }
        title={t("settings.history.postProcess")}
      >
        <span className="text-[10px] font-semibold px-0.5">
          {postProcessing ? "…" : "AI"}
        </span>
      </IconButton>
      <IconButton
        onClick={handleDeleteEntry}
        disabled={retrying}
        title={t("settings.history.delete")}
      >
        <Trash2 width={16} height={16} />
      </IconButton>
    </div>
  );

  return (
    <div className={`px-3 py-2 flex flex-col ${expanded ? "gap-3 pb-4" : "gap-0"}`}>
      {/* Header row: expand · title · actions */}
      <div className="flex items-center gap-1.5 min-w-0">
        <button
          type="button"
          onClick={onToggleExpand}
          className="p-1 rounded-md text-text/50 hover:text-logo-primary shrink-0 cursor-pointer"
          title={
            expanded
              ? t("settings.history.collapse")
              : t("settings.history.expand")
          }
          aria-expanded={expanded}
        >
          {expanded ? (
            <ChevronDown width={16} height={16} />
          ) : (
            <ChevronRight width={16} height={16} />
          )}
        </button>

        <div className="flex-1 min-w-0">
          {renaming ? (
            <div className="flex items-center gap-1.5">
              <input
                autoFocus
                className="flex-1 min-w-0 text-sm font-medium px-2 py-1 rounded border border-logo-primary/40 bg-surface"
                value={renameDraft}
                disabled={renamingBusy}
                onChange={(e) => setRenameDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void commitRename();
                  if (e.key === "Escape") setRenaming(false);
                }}
                onBlur={() => {
                  if (!renamingBusy) void commitRename();
                }}
              />
            </div>
          ) : (
            <button
              type="button"
              onClick={onToggleExpand}
              className="w-full text-start min-w-0 cursor-pointer"
            >
              <p className="text-sm font-medium truncate" title={displayTitle}>
                {displayTitle}
              </p>
              {!expanded && previewText ? (
                <p className="text-xs text-text/45 truncate mt-0.5">
                  {previewText}
                  {previewText.length >= 80 ? "…" : ""}
                </p>
              ) : (
                <p className="text-[11px] text-text/40 truncate mt-0.5">
                  {formattedDate}
                </p>
              )}
            </button>
          )}
        </div>

        {actionButtons}
      </div>

      {expanded && (
        <>
          {hasCompare && !retrying ? (
            <div className="flex flex-wrap gap-1 text-xs ps-6">
              {(
                [
                  ["final", t("settings.history.viewFinal")],
                  ["raw", t("settings.history.viewRaw")],
                  ["both", t("settings.history.viewBoth")],
                ] as const
              ).map(([id, label]) => (
                <button
                  key={id}
                  type="button"
                  className={`px-2 py-0.5 rounded-full border ${
                    textView === id
                      ? "border-logo-primary/50 bg-logo-primary/10 text-logo-primary"
                      : "border-mid-gray/30 text-text/60"
                  }`}
                  onClick={() => setTextView(id)}
                >
                  {label}
                </button>
              ))}
            </div>
          ) : null}

          <div
            className={`italic text-sm pb-1 ps-6 ${
              retrying
                ? ""
                : hasTranscription || postText
                  ? "text-text/90 select-text cursor-text whitespace-pre-wrap break-words"
                  : "text-text/40"
            }`}
            style={
              retrying
                ? { animation: "transcribe-pulse 3s ease-in-out infinite" }
                : undefined
            }
          >
            {retrying && (
              <style>{`
                @keyframes transcribe-pulse {
                  0%, 100% { color: color-mix(in srgb, var(--color-text) 40%, transparent); }
                  50% { color: color-mix(in srgb, var(--color-text) 90%, transparent); }
                }
              `}</style>
            )}
            {retrying ? (
              t("settings.history.transcribing")
            ) : !hasTranscription && !postText ? (
              t("settings.history.transcriptionFailed")
            ) : textView === "both" && hasCompare ? (
              <div className="space-y-3 not-italic">
                <div>
                  <div className="text-[10px] uppercase tracking-wide text-text/40 mb-1">
                    {t("settings.history.viewRaw")}
                  </div>
                  <p className="text-text/60 whitespace-pre-wrap break-words">
                    {entry.transcription_text}
                  </p>
                </div>
                <div>
                  <div className="text-[10px] uppercase tracking-wide text-text/40 mb-1">
                    {t("settings.history.viewFinal")}
                  </div>
                  <p className="whitespace-pre-wrap break-words">{postText}</p>
                </div>
              </div>
            ) : textView === "raw" && hasCompare ? (
              entry.transcription_text
            ) : (
              displayPrimary
            )}
          </div>

          {(hasTranscription || !!postText) && !retrying ? (
            <div className="flex flex-wrap gap-2 items-center text-xs ps-6">
              <span className="text-text/50">
                {t("settings.history.correctWord")}:
              </span>
              <input
                className="px-2 py-1 rounded border border-mid-gray/40 bg-mid-gray/10 min-w-[6rem]"
                value={wrongWord}
                onChange={(e) => setWrongWord(e.target.value)}
                placeholder={t("settings.history.wrongWord")}
              />
              <span>→</span>
              <input
                className="px-2 py-1 rounded border border-mid-gray/40 bg-mid-gray/10 min-w-[6rem]"
                value={correctWord}
                onChange={(e) => setCorrectWord(e.target.value)}
                placeholder={t("settings.history.rightWord")}
              />
              <Button
                type="button"
                size="sm"
                disabled={
                  savingMarker || !wrongWord.trim() || !correctWord.trim()
                }
                onClick={() => void handleCorrectWord()}
              >
                {t("settings.history.saveMarker")}
              </Button>
            </div>
          ) : null}

          <div className="ps-6">
            <AudioPlayer onLoadRequest={handleLoadAudio} className="w-full" />
          </div>
        </>
      )}
    </div>
  );
};
