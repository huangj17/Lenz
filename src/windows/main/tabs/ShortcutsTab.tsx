import { useTranslation } from "react-i18next";
import { Section, Row, Placeholder } from "./Section";
import { useConfigStore } from "@/stores/config";

/**
 * 快捷键 Tab(占位)。
 * M7 实现:每行 功能名 + 当前快捷键 + 录制 + 清空,冲突检测。(spec 09 §2.4)
 */
export function ShortcutsTab() {
  const { t } = useTranslation();
  const shortcuts = useConfigStore((s) => s.config.shortcuts);

  const rows: { key: keyof typeof shortcuts; label: string }[] = [
    { key: "selection", label: t("shortcuts.selection") },
    { key: "screenshot", label: t("shortcuts.screenshot") },
    { key: "overlay", label: t("shortcuts.overlay") },
    { key: "openMain", label: t("shortcuts.openMain") },
  ];

  return (
    <Section title={t("shortcuts.title")}>
      {rows.map((r) => (
        <Row key={r.key} label={r.label}>
          <span className="font-mono text-xs text-fg-muted">
            {shortcuts[r.key] ?? t("shortcuts.unset")}
          </span>
        </Row>
      ))}
      <div className="mt-4">
        <Placeholder label={t("common.comingSoon")} />
      </div>
    </Section>
  );
}
