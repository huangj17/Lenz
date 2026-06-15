import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Card, Section, Row } from "../components/primitives";
import { Toggle } from "@/components/ui/toggle";
import { Segmented } from "@/components/ui/segmented";
import { Icon } from "@/components/Icon";
import { useConfigStore } from "@/stores/config";

const LANG_NAMES: Record<string, string> = {
  "en-US": "English",
  en: "English",
  "zh-Hans": "简体中文",
  "zh-CN": "简体中文",
  "zh-Hant": "繁體中文",
  "ja-JP": "日本語",
  ja: "日本語",
  "ko-KR": "한국어",
  ko: "한국어",
  "fr-FR": "Français",
  fr: "Français",
  "de-DE": "Deutsch",
  de: "Deutsch",
  "es-ES": "Español",
  es: "Español",
};
const SUGGESTED = ["ja-JP", "ko-KR", "fr-FR", "de-DE", "es-ES"];

type Accuracy = "fast" | "accurate";

/** OCR —— 引擎设置 + 识别语言(对齐设计 OcrPane)。精度/自动检测为 UI 占位,M2 接 OCR 后落配置。 */
export function OcrPane() {
  const { t } = useTranslation();
  const preferred = useConfigStore((s) => s.config.ocr.preferredLanguages);
  const [accuracy, setAccuracy] = useState<Accuracy>("accurate");
  const [autoDetect, setAutoDetect] = useState(true);

  const selected = new Set(preferred);
  const others = SUGGESTED.filter((l) => !selected.has(l));

  return (
    <div>
      <Section title={t("ocr.engine")}>
        <Card>
          <Row label={t("ocr.backend")} hint={t("ocr.backendHint")}>
            <span className="flex items-center gap-1 text-[12.5px]">
              Apple Vision <Icon name="chevronDown" size={12} />
            </span>
          </Row>
          <Row label={t("ocr.accuracy")} hint={t("ocr.accuracyHint")}>
            <Segmented<Accuracy>
              size="sm"
              value={accuracy}
              onChange={setAccuracy}
              options={[
                { value: "fast", label: t("ocr.fast") },
                { value: "accurate", label: t("ocr.accurate") },
              ]}
            />
          </Row>
          <Row last label={t("ocr.autoDetect")}>
            <Toggle checked={autoDetect} onCheckedChange={setAutoDetect} />
          </Row>
        </Card>
      </Section>

      <Section title={t("ocr.languages")} description={t("ocr.languagesDesc")}>
        <div className="flex flex-wrap gap-1.5">
          {preferred.map((l) => (
            <span
              key={l}
              className="inline-flex items-center gap-1.5 rounded-md border-[0.5px] px-2.5 py-1 text-xs font-medium text-accent-ink"
              style={{ background: "var(--tint)", borderColor: "rgba(78,163,217,.3)" }}
            >
              <Icon name="drag" size={10} className="opacity-40" />
              {LANG_NAMES[l] ?? l}
            </span>
          ))}
          {others.map((l) => (
            <span
              key={l}
              className="inline-flex items-center gap-1.5 rounded-md border-[0.5px] border-line bg-field px-2.5 py-1 text-xs"
            >
              <Icon name="drag" size={10} className="opacity-40" />
              {LANG_NAMES[l] ?? l}
            </span>
          ))}
        </div>
      </Section>
    </div>
  );
}
