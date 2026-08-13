import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { AppDataDirectory } from "../AppDataDirectory";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { ShowWhatsNewOnUpdate } from "../ShowWhatsNewOnUpdate";
import { ThemeSelector } from "../ThemeSelector";
import { LogDirectory } from "../debug";
import TingmoMark from "../../icons/TingmoMark";
import LuxiLabLogo from "../../icons/LuxiLabLogo";

export const AboutSettings: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.1.2");
      }
    };

    fetchVersion();
  }, []);

  const handleDonateClick = async () => {
    try {
      await openUrl("https://handy.computer/donate");
    } catch (error) {
      console.error("Failed to open donate link:", error);
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      {/* Product identity */}
      <div className="rounded-xl border border-border bg-surface px-5 py-6 flex flex-col items-center text-center gap-2">
        <TingmoMark size={56} />
        <h2 className="text-xl font-semibold text-text mt-1 tracking-wide">
          {t("app.name")}
        </h2>
        <p className="text-sm text-logo-primary font-medium tracking-wide">
          {t("app.slogan")}
        </p>
        <p className="text-xs text-text-muted max-w-md leading-relaxed">
          {t("app.tagline")}
        </p>
        {/* eslint-disable-next-line i18next/no-literal-string */}
        <span className="text-xs font-mono text-text-muted mt-1">v{version}</span>
      </div>

      <SettingsGroup title={t("settings.about.title")}>
        <AppLanguageSelector descriptionMode="tooltip" grouped={true} />
        <ThemeSelector descriptionMode="tooltip" grouped={true} />
        <SettingContainer
          title={t("settings.about.version.title")}
          description={t("settings.about.version.description")}
          grouped={true}
        >
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="text-sm font-mono">v{version}</span>
        </SettingContainer>

        <ShowWhatsNewOnUpdate descriptionMode="tooltip" grouped={true} />

        <SettingContainer
          title={t("settings.about.supportDevelopment.title")}
          description={t("settings.about.supportDevelopment.description")}
          grouped={true}
        >
          <Button variant="primary" size="md" onClick={handleDonateClick}>
            {t("settings.about.supportDevelopment.button")}
          </Button>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.sourceCode.title")}
          description={t("settings.about.sourceCode.description")}
          grouped={true}
        >
          <Button
            variant="secondary"
            size="md"
            onClick={() => openUrl("https://github.com/cjpais/Handy")}
          >
            {t("settings.about.sourceCode.button")}
          </Button>
        </SettingContainer>

        <AppDataDirectory descriptionMode="tooltip" grouped={true} />
        <LogDirectory grouped={true} />
      </SettingsGroup>

      {/* Developer / LUXI Lab */}
      <div className="rounded-xl border border-border bg-surface px-5 py-5 flex flex-col sm:flex-row items-center gap-4">
        <LuxiLabLogo width={88} height={88} className="shrink-0" />
        <div className="flex flex-col gap-1 text-center sm:text-start min-w-0">
          <p className="text-sm font-semibold text-text">
            {t("developer.name")}
          </p>
          <p className="text-sm text-logo-primary font-medium">
            {t("developer.slogan")}
          </p>
          <p className="text-xs text-text-muted leading-relaxed">
            {t("developer.blurb")}
          </p>
        </div>
      </div>

      <SettingsGroup title={t("settings.about.acknowledgments.title")}>
        <SettingContainer
          title={t("settings.about.acknowledgments.ggml.title")}
          description={t("settings.about.acknowledgments.ggml.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            {t("settings.about.acknowledgments.ggml.details")}
          </div>
        </SettingContainer>
      </SettingsGroup>
    </div>
  );
};
