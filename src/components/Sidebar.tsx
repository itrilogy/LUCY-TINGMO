import React from "react";
import { useTranslation } from "react-i18next";
import { Cog, FlaskConical, History, Info, Sparkles, Cpu } from "lucide-react";
import TingmoMark from "./icons/TingmoMark";
import { useSettings } from "../hooks/useSettings";
import {
  GeneralSettings,
  AdvancedSettings,
  HistorySettings,
  DebugSettings,
  AboutSettings,
  PostProcessingSettings,
  ModelsSettings,
} from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  [key: string]: any;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: (settings: any) => boolean;
}

export const SECTIONS_CONFIG = {
  general: {
    labelKey: "sidebar.general",
    icon: TingmoMark,
    component: GeneralSettings,
    enabled: () => true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: Cog,
    component: AdvancedSettings,
    enabled: () => true,
  },
  postprocessing: {
    labelKey: "sidebar.postProcessing",
    icon: Sparkles,
    component: PostProcessingSettings,
    enabled: (settings) => settings?.post_process_enabled ?? false,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode ?? false,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const availableSections = Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  return (
    <div className="flex flex-col w-48 h-full border-e border-border bg-surface/40 items-center px-2">
      <div className="flex flex-col items-center gap-1.5 m-4 mb-3">
        <TingmoMark size={36} />
        <div className="flex flex-col items-center leading-tight text-center px-1">
          <span className="text-base font-semibold tracking-wide text-text">
            {t("app.name")}
          </span>
          <span className="text-[10px] text-text-muted tracking-wide mt-0.5 leading-snug">
            {t("app.slogan")}
          </span>
        </div>
      </div>
      <div className="flex flex-col w-full items-center gap-1 pt-2 border-t border-border">
        {availableSections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;

          return (
            <div
              key={section.id}
              className={`relative flex gap-2 items-center p-2 w-full rounded-lg cursor-pointer transition-colors ${
                isActive
                  ? "bg-logo-primary/12 text-text"
                  : "hover:bg-surface-2 hover:opacity-100 opacity-85"
              }`}
              onClick={() => onSectionChange(section.id)}
            >
              {isActive && (
                <span className="absolute start-0 top-1.5 bottom-1.5 w-[3px] rounded-full bg-logo-primary" />
              )}
              <Icon
                width={22}
                height={22}
                size={22}
                className="shrink-0 ms-0.5"
              />
              <p
                className="text-sm font-medium truncate"
                title={t(section.labelKey)}
              >
                {t(section.labelKey)}
              </p>
            </div>
          );
        })}
      </div>
    </div>
  );
};
