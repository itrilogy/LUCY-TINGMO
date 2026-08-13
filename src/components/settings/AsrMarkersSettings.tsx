import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import type {
  AsrMarker,
  AsrMarkerStore,
  MarkerConfidence,
  MarkerMatchMode,
} from "@/bindings";
import { SettingContainer } from "../ui/SettingContainer";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { useSettings } from "../../hooks/useSettings";

interface AsrMarkersSettingsProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const AsrMarkersSettings: React.FC<AsrMarkersSettingsProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [store, setStore] = useState<AsrMarkerStore | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [errorForm, setErrorForm] = useState("");
  const [correctForm, setCorrectForm] = useState("");
  const [category, setCategory] = useState("term");
  const [matchMode, setMatchMode] = useState<MarkerMatchMode>("exact");
  const [filePath, setFilePath] = useState("");
  const [importMd, setImportMd] = useState("");
  const [showFormatHelp, setShowFormatHelp] = useState(false);

  const ruleEnabled = getSetting("asr_markers_rule_enabled") ?? true;
  const injectGlossary = getSetting("asr_markers_inject_glossary") ?? true;
  const autoAccept = getSetting("asr_markers_auto_accept_new") ?? false;

  const refresh = useCallback(async () => {
    setError(null);
    const res = await commands.getAsrMarkerStore();
    if (res.status === "ok") {
      setStore(res.data);
    } else {
      setError(String(res.error));
    }
    const pathRes = await commands.getAsrMarkersFilePath();
    if (pathRes.status === "ok") setFilePath(pathRes.data);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleAdd = async () => {
    const errors = errorForm
      .split(/[/、,，]/)
      .map((s) => s.trim())
      .filter(Boolean);
    if (!errors.length || !correctForm.trim()) return;
    const marker: AsrMarker = {
      asr_errors: errors,
      correct: correctForm.trim(),
      category: category.trim() || "other",
      note: null,
      source: "ui",
      confidence: "verified",
      match_mode: matchMode,
    };
    const res = await commands.addAsrMarker(marker);
    if (res.status === "ok") {
      setStore(res.data);
      setErrorForm("");
      setCorrectForm("");
      setMatchMode("exact");
    } else {
      setError(String(res.error));
    }
  };

  const handleRemove = async (index: number) => {
    const res = await commands.removeAsrMarker(index);
    if (res.status === "ok") setStore(res.data);
    else setError(String(res.error));
  };

  const handleAcceptPending = async () => {
    const res = await commands.acceptPendingAsrMarkers();
    if (res.status === "ok") setStore(res.data);
    else setError(String(res.error));
  };

  const handleImportMd = async () => {
    if (!importMd.trim()) return;
    const res = await commands.importAsrMarkersMarkdown(
      importMd,
      "markdown_import",
    );
    if (res.status === "ok") {
      setStore(res.data);
      setImportMd("");
    } else {
      setError(String(res.error));
    }
  };

  const handleExportMd = async () => {
    const res = await commands.exportAsrMarkersMarkdown();
    if (res.status === "ok") {
      setImportMd(res.data);
      try {
        await navigator.clipboard.writeText(res.data);
      } catch {
        // textarea still shows content if clipboard is blocked
      }
    } else {
      setError(String(res.error));
    }
  };

  const handleExportMdFile = async () => {
    const res = await commands.exportAsrMarkersMarkdownFile();
    if (res.status === "ok") {
      setFilePath(res.data);
    } else {
      setError(String(res.error));
    }
  };

  const setConfidence = async (index: number, confidence: MarkerConfidence) => {
    const res = await commands.setAsrMarkerConfidence(index, confidence);
    if (res.status === "ok") setStore(res.data);
    else setError(String(res.error));
  };

  const pendingCount =
    store?.markers?.filter((m) => m.confidence === "pending").length ?? 0;

  return (
    <div className="space-y-3">
      <ToggleSwitch
        checked={!!ruleEnabled}
        onChange={(v) => updateSetting("asr_markers_rule_enabled", v)}
        isUpdating={isUpdating("asr_markers_rule_enabled")}
        label={t("settings.advanced.asrMarkers.ruleEnabled.title")}
        description={t("settings.advanced.asrMarkers.ruleEnabled.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
      <ToggleSwitch
        checked={!!injectGlossary}
        onChange={(v) => updateSetting("asr_markers_inject_glossary", v)}
        isUpdating={isUpdating("asr_markers_inject_glossary")}
        label={t("settings.advanced.asrMarkers.injectGlossary.title")}
        description={t(
          "settings.advanced.asrMarkers.injectGlossary.description",
        )}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
      <ToggleSwitch
        checked={!!autoAccept}
        onChange={(v) => updateSetting("asr_markers_auto_accept_new", v)}
        isUpdating={isUpdating("asr_markers_auto_accept_new")}
        label={t("settings.advanced.asrMarkers.autoAccept.title")}
        description={t("settings.advanced.asrMarkers.autoAccept.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />

      <SettingContainer
        title={t("settings.advanced.asrMarkers.manage.title")}
        description={t("settings.advanced.asrMarkers.manage.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        layout="stacked"
      >
        {filePath ? (
          <p className="text-xs text-mid-gray mb-2 break-all">
            {t("settings.advanced.asrMarkers.filePath")}: {filePath}
          </p>
        ) : null}
        {error ? (
          <p className="text-xs text-red-600 mb-2">{error}</p>
        ) : null}

        <div className="flex flex-wrap gap-2 mb-3">
          <Input
            value={errorForm}
            onChange={(e) => setErrorForm(e.target.value)}
            placeholder={t("settings.advanced.asrMarkers.form.error")}
            className="flex-1 min-w-[8rem]"
          />
          <Input
            value={correctForm}
            onChange={(e) => setCorrectForm(e.target.value)}
            placeholder={t("settings.advanced.asrMarkers.form.correct")}
            className="flex-1 min-w-[8rem]"
          />
          <Input
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            placeholder={t("settings.advanced.asrMarkers.form.category")}
            className="w-24"
          />
          <select
            className="px-2 py-1 rounded-md border border-mid-gray/40 bg-mid-gray/5 text-sm"
            value={matchMode}
            onChange={(e) => setMatchMode(e.target.value as MarkerMatchMode)}
            title={t("settings.advanced.asrMarkers.form.matchMode")}
          >
            <option value="exact">
              {t("settings.advanced.asrMarkers.matchMode.exact")}
            </option>
            <option value="case_insensitive">
              {t("settings.advanced.asrMarkers.matchMode.caseInsensitive")}
            </option>
            <option value="regex">
              {t("settings.advanced.asrMarkers.matchMode.regex")}
            </option>
          </select>
          <Button type="button" onClick={() => void handleAdd()}>
            {t("settings.advanced.asrMarkers.form.add")}
          </Button>
          {pendingCount > 0 ? (
            <Button type="button" onClick={() => void handleAcceptPending()}>
              {t("settings.advanced.asrMarkers.acceptPending", {
                count: pendingCount,
              })}
            </Button>
          ) : null}
          <Button type="button" variant="ghost" onClick={() => void refresh()}>
            {t("settings.advanced.asrMarkers.refresh")}
          </Button>
        </div>

        <div className="mb-3">
          <button
            type="button"
            className="text-xs text-logo-primary mb-1 underline-offset-2 hover:underline"
            onClick={() => setShowFormatHelp((v) => !v)}
          >
            {showFormatHelp
              ? t("settings.advanced.asrMarkers.helpHide")
              : t("settings.advanced.asrMarkers.helpShow")}
          </button>
          {showFormatHelp ? (
            <div className="mb-2 p-2 rounded-md border border-mid-gray/30 bg-mid-gray/5 text-xs text-text/80 space-y-1 whitespace-pre-wrap">
              {t("settings.advanced.asrMarkers.helpBody")}
              <Button
                type="button"
                size="sm"
                variant="ghost"
                className="mt-2"
                onClick={() =>
                  setImportMd(t("settings.advanced.asrMarkers.importExample"))
                }
              >
                {t("settings.advanced.asrMarkers.insertExample")}
              </Button>
            </div>
          ) : null}
          <textarea
            className="w-full min-h-[5rem] text-xs font-mono p-2 rounded-md border border-mid-gray/40 bg-mid-gray/5"
            value={importMd}
            onChange={(e) => setImportMd(e.target.value)}
            placeholder={t("settings.advanced.asrMarkers.importPlaceholder")}
          />
          <div className="mt-2 flex flex-wrap gap-2">
            <Button type="button" onClick={() => void handleImportMd()}>
              {t("settings.advanced.asrMarkers.import")}
            </Button>
            <Button
              type="button"
              variant="ghost"
              onClick={() => void handleExportMd()}
            >
              {t("settings.advanced.asrMarkers.export")}
            </Button>
            <Button
              type="button"
              variant="ghost"
              onClick={() => void handleExportMdFile()}
            >
              {t("settings.advanced.asrMarkers.exportFile")}
            </Button>
          </div>
          <p className="mt-1 text-xs text-mid-gray">
            {t("settings.advanced.asrMarkers.obsidianHint")}
          </p>
        </div>

        <div className="max-h-56 overflow-y-auto border border-mid-gray/20 rounded-md text-sm">
          {(store?.markers ?? []).length === 0 ? (
            <p className="p-3 text-mid-gray">
              {t("settings.advanced.asrMarkers.empty")}
            </p>
          ) : (
            <ul className="divide-y divide-mid-gray/15">
              {(store?.markers ?? []).map((m, i) => (
                <li
                  key={`${m.correct}-${i}`}
                  className="px-3 py-2 flex flex-wrap items-center gap-2 justify-between"
                >
                  <div className="min-w-0 flex-1">
                    <span className="text-mid-gray line-through mr-1">
                      {m.asr_errors.join(" / ")}
                    </span>
                    <span className="mx-1">→</span>
                    <span className="font-medium">{m.correct}</span>
                    <span className="ml-2 text-xs text-mid-gray">
                      [{m.category}] {m.confidence} ·{" "}
                      {m.match_mode === "case_insensitive"
                        ? t(
                            "settings.advanced.asrMarkers.matchMode.caseInsensitive",
                          )
                        : m.match_mode === "regex"
                          ? t("settings.advanced.asrMarkers.matchMode.regex")
                          : t("settings.advanced.asrMarkers.matchMode.exact")}
                    </span>
                  </div>
                  <div className="flex gap-1 shrink-0">
                    {m.confidence === "pending" ? (
                      <Button
                        type="button"
                        size="sm"
                        onClick={() => void setConfidence(i, "verified")}
                      >
                        {t("settings.advanced.asrMarkers.verify")}
                      </Button>
                    ) : null}
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      onClick={() => void handleRemove(i)}
                    >
                      {t("settings.advanced.asrMarkers.remove")}
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </SettingContainer>
    </div>
  );
};
