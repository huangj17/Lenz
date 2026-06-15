import { useTranslation } from "react-i18next";
import { Section, Row, Placeholder } from "./Section";
import { useConfigStore } from "@/stores/config";

/**
 * OCR Tab(占位)。
 * M7 实现:引擎选择(MVP 仅 system)、preferredLanguages 多选、
 * minConfidence 滑块、语言包缺失引导。(spec 09 §2.2 / spec 03 §4.1)
 */
export function OcrTab() {
  const { t } = useTranslation();
  const ocr = useConfigStore((s) => s.config.ocr);

  return (
    <Section title={t("ocr.title")}>
      <Row label={t("ocr.engine")}>
        <span className="text-[13px] text-fg-muted">
          {t("ocr.systemEngine")}
        </span>
      </Row>
      <Row label={t("ocr.minConfidence")}>
        <span className="font-mono text-xs text-fg-muted">
          {ocr.minConfidence.toFixed(2)}
        </span>
      </Row>
      <div className="mt-4">
        <Placeholder label={t("common.comingSoon")} />
      </div>
    </Section>
  );
}
