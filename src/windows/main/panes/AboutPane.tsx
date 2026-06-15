import { useTranslation } from "react-i18next";
import { LensMark } from "@/components/LensMark";
import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";

const VERSION = "0.1.0";

/** 关于 —— logo + 信息 + 链接 + 隐私说明(对齐设计 AboutPane) */
export function AboutPane() {
  const { t } = useTranslation();
  return (
    <div className="flex items-start gap-7">
      <LensMark size={88} />
      <div>
        <div className="text-[28px] font-semibold tracking-[-0.02em]">
          Lenz
          <span className="ml-2 font-mono text-sm font-normal text-sub">
            v{VERSION}（{t("about.beta")}）
          </span>
        </div>
        <div className="mt-1 text-[13px] text-sub">{t("about.tagline")}</div>
        <div className="mt-0.5 text-[13px] leading-normal text-sub">
          {t("about.stack")}
        </div>

        <div className="mt-4 flex gap-2">
          <Button variant="warm">
            <Icon name="external" size={12} />
            GitHub
          </Button>
          <Button variant="secondary">{t("about.changelog")}</Button>
          <Button variant="secondary">{t("about.checkUpdate")}</Button>
        </div>

        <div className="mt-[26px] max-w-[440px] rounded-[9px] border-[0.5px] border-line bg-card p-3.5 text-xs leading-relaxed text-sub">
          {t("about.privacy")}
        </div>
      </div>
    </div>
  );
}
