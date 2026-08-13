import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { SettingContainer } from "../ui/SettingContainer";
import { Dropdown } from "../ui/Dropdown";
import { useSettings } from "../../hooks/useSettings";
import { useModelStore } from "../../stores/modelStore";
import { commands } from "@/bindings";

interface TranscriptionModeSelectorProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

type ModeValue = "batch_auto_split" | "vad_segmented" | "native_stream";

export const TranscriptionModeSelector: React.FC<
  TranscriptionModeSelectorProps
> = ({ descriptionMode = "tooltip", grouped = false }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating, refreshSettings } =
    useSettings();
  const models = useModelStore((s) => s.models);
  const selectedModelId = getSetting("selected_model") ?? "";

  const mode = (getSetting("transcription_mode") ??
    "batch_auto_split") as ModeValue;

  const selectedModel = useMemo(
    () => models.find((m) => m.id === selectedModelId),
    [models, selectedModelId],
  );
  const supportsStreaming = !!selectedModel?.supports_streaming;

  // Backend demoted mode after model switch
  useEffect(() => {
    let un: (() => void) | undefined;
    void listen<string>("transcription-mode-fallback", async () => {
      await refreshSettings();
      toast.message(t("settings.advanced.transcriptionMode.fallbackToast"));
    }).then((fn) => {
      un = fn;
    });
    return () => un?.();
  }, [refreshSettings, t]);

  // Local consistency if settings still say native_stream without capability
  useEffect(() => {
    if (mode === "native_stream" && selectedModel && !supportsStreaming) {
      void (async () => {
        await updateSetting("transcription_mode", "vad_segmented" as any);
        toast.message(t("settings.advanced.transcriptionMode.fallbackToast"));
      })();
    }
  }, [mode, supportsStreaming, selectedModel?.id]);

  const options = [
    {
      value: "batch_auto_split",
      label: t("settings.advanced.transcriptionMode.options.batchAutoSplit"),
      disabled: false,
    },
    {
      value: "vad_segmented",
      label: t("settings.advanced.transcriptionMode.options.vadSegmented"),
      disabled: false,
    },
    {
      value: "native_stream",
      label: supportsStreaming
        ? t("settings.advanced.transcriptionMode.options.nativeStream")
        : `${t("settings.advanced.transcriptionMode.options.nativeStream")} (${t("settings.advanced.transcriptionMode.requiresStreaming")})`,
      disabled: !supportsStreaming,
    },
  ];

  const handleSelect = async (value: string) => {
    if (value === "native_stream" && !supportsStreaming) {
      toast.error(t("settings.advanced.transcriptionMode.noStreamingError"));
      await updateSetting("transcription_mode", "vad_segmented" as any);
      return;
    }
    const res = await commands.changeTranscriptionModeSetting(value as any);
    if (res.status === "error") {
      toast.error(t("settings.advanced.transcriptionMode.noStreamingError"));
      await updateSetting("transcription_mode", "vad_segmented" as any);
      await refreshSettings();
      return;
    }
    await refreshSettings();
  };

  const displayMode =
    mode === "native_stream" && !supportsStreaming ? "vad_segmented" : mode;

  return (
    <SettingContainer
      title={t("settings.advanced.transcriptionMode.title")}
      description={t("settings.advanced.transcriptionMode.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    >
      <div className="space-y-2 w-full">
        <Dropdown
          options={options}
          selectedValue={displayMode}
          onSelect={(value: string) => void handleSelect(value)}
          disabled={isUpdating("transcription_mode")}
        />
        {!supportsStreaming && (
          <p className="text-xs text-mid-gray leading-relaxed">
            {t("settings.advanced.transcriptionMode.noStreamingHint")}
          </p>
        )}
        {supportsStreaming && mode === "native_stream" && (
          <p className="text-xs text-mid-gray leading-relaxed">
            {t("settings.advanced.transcriptionMode.nativeStreamHint")}
          </p>
        )}
        {displayMode === "vad_segmented" && (
          <p className="text-xs text-mid-gray leading-relaxed">
            {t("settings.advanced.transcriptionMode.vadHint")}
          </p>
        )}
        {displayMode === "batch_auto_split" && (
          <p className="text-xs text-mid-gray leading-relaxed">
            {t("settings.advanced.transcriptionMode.batchHint")}
          </p>
        )}
      </div>
    </SettingContainer>
  );
};
