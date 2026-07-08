import { useTranslation } from "react-i18next";
import { Card, Section, Row } from "../components/primitives";
import { Toggle } from "@/components/ui/toggle";
import { Segmented } from "@/components/ui/segmented";
import { Icon } from "@/components/Icon";
import { useConfigStore } from "@/stores/config";
import type { LocaleSetting, ThemeSetting } from "@/lib/types";

/** 通用 —— 启动 / 外观(对齐设计 GeneralPane) */
export function GeneralPane() {
  const { t } = useTranslation();
  const config = useConfigStore((s) => s.config);
  const patch = useConfigStore((s) => s.patch);
  const { app, selection } = config;

  return (
    <div>
      <Section title={t("general.startup")}>
        <Card>
          <Row label={t("general.autoStart")} hint={t("general.autoStartHint")}>
            <Toggle
              checked={app.autoStart}
              onCheckedChange={(v) => void patch({ app: { autoStart: v } })}
            />
          </Row>
          <Row
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
            label={t("general.selectionIndicator")}
            hint={t("general.selectionIndicatorHint")}
          >
            <Toggle
              checked={selection.enabled}
              onCheckedChange={(v) => void patch({ selection: { enabled: v } })}
            />
          </Row>
        </Card>
      </Section>

      <Section title={t("general.appearance")}>
        <Card>
          <Row label={t("theme.label")}>
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
          <Row last label={t("locale.label")}>
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
