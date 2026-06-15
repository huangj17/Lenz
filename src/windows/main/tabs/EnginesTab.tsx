import { useTranslation } from "react-i18next";
import { Section, Placeholder } from "./Section";
import { useConfigStore } from "@/stores/config";

/**
 * 翻译引擎 Tab(占位)。
 * M7 实现:左侧引擎列表(拖拽排序 / 启用 toggle / 删除)+ 右侧详情表单 +
 * 测试连接 + 添加引擎 + API Key 明文警示条。(spec 09 §2.3)
 */
export function EnginesTab() {
  const { t } = useTranslation();
  const engines = useConfigStore((s) => s.config.translator.engines);

  return (
    <Section title={t("engines.title")}>
      <div className="mb-3 rounded-[var(--radius)] border border-border bg-field px-3 py-2 text-[11px] text-fg-muted">
        {t("engines.credentialWarning")}
      </div>
      {engines.length === 0 ? (
        <Placeholder label={t("engines.empty")} />
      ) : (
        <Placeholder label={t("common.comingSoon")} />
      )}
    </Section>
  );
}
