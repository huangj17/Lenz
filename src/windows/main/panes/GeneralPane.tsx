import { useTranslation } from "react-i18next";
import { Card, Section, Row } from "../components/primitives";
import { Toggle } from "@/components/ui/toggle";
import { Segmented } from "@/components/ui/segmented";
import { Icon } from "@/components/Icon";
import { useConfigStore } from "@/stores/config";
import { useSystemStore } from "@/stores/system";
import { systemApi, selectionApi } from "@/lib/ipc";
import type { LocaleSetting, ThemeSetting } from "@/lib/types";

/** 通用 —— 启动 / 外观(对齐设计 GeneralPane) */
export function GeneralPane() {
  const { t } = useTranslation();
  const config = useConfigStore((s) => s.config);
  const setError = useConfigStore((s) => s.setError);
  const patch = useConfigStore((s) => s.patch);
  const { app, selection } = config;
  const info = useSystemStore((s) => s.info);
  const reload = useSystemStore((s) => s.load);
  const retryListener = async () => {
    await selectionApi.toggleListener(selection.enabled);
    await reload();
  };

  return (
    <div>
      <Section title={t("general.startup")}>
        <Card>
          <Row id="general-auto-start" label={t("general.autoStart")} hint={t("general.autoStartHint")}>
            <Toggle
              checked={app.autoStart}
              onCheckedChange={(v) => void systemApi.setAutoStart(v).catch(setError)}
            />
          </Row>
          <Row
            id="general-update"
            label={t("general.checkUpdate")}
            hint={t("general.checkUpdateHint")}
          >
            <Toggle
              checked={app.checkUpdate}
              onCheckedChange={(v) => void patch({ app: { checkUpdate: v } })}
            />
          </Row>
          <Row
            last
            id="general-open-on-launch"
            label={t("general.openOnLaunch")}
            hint={t("general.openOnLaunchHint")}
          >
            <Toggle
              checked={app.openOnLaunch}
              onCheckedChange={(v) => void patch({ app: { openOnLaunch: v } })}
            />
          </Row>
        </Card>
      </Section>

      <Section title={t("general.selection")}>
        <Card>
          <Row
            last
            id="selection-indicator"
            label={t("general.selectionIndicator")}
            hint={t(info?.platform === "windows" ? "general.selectionIndicatorWindowsHint" : "general.selectionIndicatorHint")}
          >
            <Toggle
              checked={selection.enabled}
              onCheckedChange={(v) => void selectionApi.toggleListener(v).then(() => reload()).catch(setError)}
            />
          </Row>
        </Card>
      </Section>

      {info && <div className="mb-5 rounded-md border border-line bg-card p-3 text-xs">
        <div className="flex items-center justify-between">
          <span>{t("general.listenerStatus")}: {t(info.selectionRunning ? "general.running" : "general.stopped")}</span>
          <button type="button" className="text-accent-ink" onClick={() => void retryListener().catch(setError)}>{t("common.refresh")}</button>
        </div>
        {info.platform === "macos" && <div className="mt-2 flex flex-col gap-2">
          {!info.accessibilityGranted && <div><span>{t("general.accessibilityRequired")}</span>
            <button type="button" className="ml-2 text-accent-ink" onClick={() => void systemApi.open("accessibility").catch(setError)}>{t("general.openPermissions")}</button></div>}
          {!info.screenRecordingGranted && <div><span>{t("general.screenRecordingRequired")}</span>
            <button type="button" className="ml-2 text-accent-ink" onClick={() => void systemApi.open("screenRecording").catch(setError)}>{t("general.openPermissions")}</button></div>}
        </div>}
      </div>}

      <Section title={t("general.appearance")}>
        <Card>
          <Row id="theme" label={t("theme.label")}>
            <Segmented<ThemeSetting>
              size="sm"
              value={app.theme}
              onChange={(v) => void patch({ app: { theme: v } })}
              options={[
                { value: "system", label: t("theme.system") },
                { value: "light", label: t("theme.light") },
                { value: "dark", label: t("theme.dark") },
              ]}
            />
          </Row>
          <Row id="locale" last label={t("locale.label")}>
            <LangSelect
              value={app.locale}
              onChange={(v) => void patch({ app: { locale: v } })}
            />
          </Row>
        </Card>
      </Section>
    </div>
  );
}

/** 语言下拉:样式化原生 select(隐藏默认箭头 + 自定义 chevron) */
function LangSelect({
  value,
  onChange,
}: {
  value: LocaleSetting;
  onChange: (v: LocaleSetting) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="relative inline-flex items-center text-[12.5px]">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as LocaleSetting)}
        className="cursor-pointer appearance-none border-0 bg-transparent pr-5 text-right text-[12.5px] text-fg outline-none"
      >
        <option value="system">{t("locale.system")}</option>
        <option value="zh-CN">{t("locale.zh-CN")}</option>
        <option value="en-US">{t("locale.en-US")}</option>
      </select>
      <Icon
        name="chevronDown"
        size={12}
        className="pointer-events-none -ml-4 text-sub"
      />
    </div>
  );
}
