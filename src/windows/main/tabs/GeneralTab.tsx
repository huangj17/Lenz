import { useTranslation } from "react-i18next";
import { Section, Row } from "./Section";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useConfigStore } from "@/stores/config";
import type { LocaleSetting, ThemeSetting } from "@/lib/types";

/**
 * 通用 Tab:语言 / 主题切换(占位但功能性 —— 直接 patch config),
 * 以及自启 / 启动打开主窗口 / 检查更新开关、代理占位。
 */
export function GeneralTab() {
  const { t } = useTranslation();
  const config = useConfigStore((s) => s.config);
  const patch = useConfigStore((s) => s.patch);

  return (
    <div>
      <Section title={t("general.appearance")}>
        <Row label={t("locale.label")}>
          <Select
            value={config.app.locale}
            onChange={(e) =>
              void patch({
                app: { locale: e.target.value as LocaleSetting },
              })
            }
          >
            <option value="system">{t("locale.system")}</option>
            <option value="zh-CN">{t("locale.zh-CN")}</option>
            <option value="en-US">{t("locale.en-US")}</option>
          </Select>
        </Row>
        <Row label={t("theme.label")}>
          <Select
            value={config.app.theme}
            onChange={(e) =>
              void patch({ app: { theme: e.target.value as ThemeSetting } })
            }
          >
            <option value="system">{t("theme.system")}</option>
            <option value="light">{t("theme.light")}</option>
            <option value="dark">{t("theme.dark")}</option>
          </Select>
        </Row>
      </Section>

      <Section title={t("general.startup")}>
        <Row label={t("general.autoStart")} hint={t("general.autoStartHint")}>
          <Switch
            checked={config.app.autoStart}
            onCheckedChange={(v) => void patch({ app: { autoStart: v } })}
          />
        </Row>
        <Row label={t("general.openOnLaunch")}>
          <Switch
            checked={config.app.openOnLaunch}
            onCheckedChange={(v) => void patch({ app: { openOnLaunch: v } })}
          />
        </Row>
        <Row label={t("general.checkUpdate")}>
          <Switch
            checked={config.app.checkUpdate}
            onCheckedChange={(v) => void patch({ app: { checkUpdate: v } })}
          />
        </Row>
      </Section>

      <Section title={t("general.proxy")} description={t("general.proxyHint")}>
        <Row label={t("general.proxy")}>
          <Switch
            checked={config.proxy.enabled}
            onCheckedChange={(v) => void patch({ proxy: { enabled: v } })}
          />
        </Row>
        {/* TODO(M7): 协议 / 地址 / 端口 / 账号密码 / bypassLocal / 测试连接 */}
      </Section>
    </div>
  );
}
