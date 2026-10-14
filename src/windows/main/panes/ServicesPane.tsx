import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Segmented } from "@/components/ui/segmented";
import { EnginesPane } from "./EnginesPane";
import { OcrPane } from "./OcrPane";

type Seg = "translate" | "ocr";

/** 服务 —— 分段控件在 文本翻译 / OCR 间切换(对齐设计 ServicesPane) */
export function ServicesPane({ section, onSectionChange }: { section?: Seg; onSectionChange?: (section: Seg) => void }) {
  const { t } = useTranslation();
  const [seg, setSeg] = useState<Seg>("translate");

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="mb-4">
        <Segmented<Seg>
          value={section ?? seg}
          onChange={(value) => { setSeg(value); onSectionChange?.(value); }}
          options={[
            { value: "translate", label: t("services.translate"), icon: "cpu" },
            { value: "ocr", label: t("services.ocr"), icon: "ocr" },
          ]}
        />
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {(section ?? seg) === "translate" ? <EnginesPane /> : <OcrPane />}
      </div>
    </div>
  );
}
