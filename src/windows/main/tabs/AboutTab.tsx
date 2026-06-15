import { useTranslation } from "react-i18next";
import { Section, Row } from "./Section";

/**
 * 关于 Tab(占位)。
 * M7 实现:版本号 / 构建日期、MIT License 链接、GitHub 链接、
 * 第三方许可证页面。(spec 09 §2.2 / spec 10 §7)
 */
const APP_VERSION = "0.1.0";
const BUNDLE_ID = "app.lenz.desktop";
const GITHUB_URL = "https://github.com/";

export function AboutTab() {
  const { t } = useTranslation();

  return (
    <Section title={t("about.title")}>
      <Row label={t("about.version")}>
        <span className="font-mono text-xs text-fg-muted">{APP_VERSION}</span>
      </Row>
      <Row label={t("about.bundleId")}>
        <span className="font-mono text-xs text-fg-muted">{BUNDLE_ID}</span>
      </Row>
      <Row label={t("about.license")}>
        <span className="text-[13px] text-fg-muted">MIT</span>
      </Row>
      <Row label={t("about.github")}>
        {/* TODO(M7): 用 tauri-plugin-opener 在外部浏览器打开 */}
        <span className="text-[13px] text-fg-muted">{GITHUB_URL}</span>
      </Row>
    </Section>
  );
}
