import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Card, Section, Row } from "../components/primitives";
import { Toggle } from "@/components/ui/toggle";
import { Segmented } from "@/components/ui/segmented";
import { Icon } from "@/components/Icon";
import { systemApi } from "@/lib/ipc";
import { useSystemStore } from "@/stores/system";
import { moveLanguage } from "@/lib/settingsSearch";
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
export function OcrPane() {
  const { t } = useTranslation();
  const config = useConfigStore((s) => s.config.ocr);
  const patch = useConfigStore((s) => s.patch);
  const setError = useConfigStore((s) => s.setError);
  const info = useSystemStore((s) => s.info);
  const reload = useSystemStore((s) => s.load);
  const [language, setLanguage] = useState("");
  const [error, setLocalError] = useState<string | null>(null);
  const [dragged, setDragged] = useState<string | null>(null);
  const preferred = config.preferredLanguages;
  const candidates = Array.from(new Set([...(info?.ocrLanguages ?? []), ...SUGGESTED])).filter((tag) => !preferred.includes(tag));
  const windows = info?.platform === "windows";
  const toggle = (tag: string) => {
    void patch((current) => ({
      ocr: { preferredLanguages: current.ocr.preferredLanguages.includes(tag)
        ? current.ocr.preferredLanguages.filter((l) => l !== tag) : [...current.ocr.preferredLanguages, tag] },
    }));
  };
  const reorder = (from: string, to: string) => {
    void patch((current) => ({ ocr: { preferredLanguages: moveLanguage(current.ocr.preferredLanguages, from, to) } }));
  };
  const add = () => {
    try {
      const tag = Intl.getCanonicalLocales(language.trim())[0];
      if (!tag || !/^[a-z]{2,3}(?:-[a-z0-9]{2,8})*$/i.test(tag)) throw new Error();
      if (!preferred.includes(tag)) toggle(tag);
      setLanguage(""); setLocalError(null);
    } catch { setLocalError(t("ocr.invalidLanguage")); }
  };
  const openLanguages = () => { void systemApi.open("ocrLanguages").catch(setError); };
  return (
    <div>
      {windows && info && !info.windowsPackageIdentity && <div role="alert" className="mb-4 rounded-md border border-line bg-card p-3 text-xs">
        <p>{t("ocr.packageRequired")}</p>
        <button type="button" className="mt-2 text-accent-ink" onClick={() => void systemApi.open("releases").catch(setError)}>{t("ocr.downloadInstalledVersion")}</button>
      </div>}
      <Section title={t("ocr.engine")}>
        <Card>
          <Row label={t("ocr.backend")} hint={t(windows ? "ocr.backendWindowsHint" : "ocr.backendHint")}>
            <span className="text-[12.5px]">{windows ? "Windows OCR" : "Apple Vision"}</span>
          </Row>
          <Row id="ocr-accuracy" label={t("ocr.accuracy")} hint={t(info?.ocrFastModeSupported ? "ocr.accuracyHint" : "ocr.systemAccuracy")}>
            {info?.ocrFastModeSupported && <Segmented<Accuracy> size="sm" value={config.fastMode ? "fast" : "accurate"}
              onChange={(value) => void patch({ ocr: { fastMode: value === "fast" } })}
              options={[{ value: "fast", label: t("ocr.fast") }, { value: "accurate", label: t("ocr.accurate") }]} />}
          </Row>
          <Row label={t(windows ? "ocr.systemFallback" : "ocr.autoDetect")} hint={info?.ocrAutoDetectSupported === false ? t("ocr.autoDetectRequires13") : undefined}>
            {info?.ocrAutoDetectSupported !== false && <Toggle checked={config.autoDetectLanguage ?? true}
              onCheckedChange={(value) => void patch({ ocr: { autoDetectLanguage: value } })} />
            }
          </Row>
          <Row last label={t("ocr.minConfidence")} hint={info?.ocrConfidenceSupported ? undefined : t("ocr.systemConfidence")}>
            {info?.ocrConfidenceSupported && <div className="flex items-center gap-2">
              <input type="range" min={0} max={1} step={0.05} value={config.minConfidence ?? 0.3}
                aria-label={t("ocr.minConfidence")}
                onChange={(event) => void patch({ ocr: { minConfidence: Number(event.target.value) } })} />
              <span className="w-8 text-right font-mono text-xs">{(config.minConfidence ?? 0.3).toFixed(2)}</span>
            </div>}
          </Row>
        </Card>
      </Section>
      <div id="ocr-languages">
        <Section title={t("ocr.languages")} description={t("ocr.languagesDesc")}>
          <div className="flex flex-wrap gap-1.5">
            {preferred.map((tag, index) => <div key={tag} data-ocr-language={tag}
              className="inline-flex items-center gap-1 rounded-md border border-line bg-tint px-2 py-1 text-xs">
              <button type="button" aria-label={t("ocr.reorderLanguage", { language: tag })} className="cursor-grab"
                onPointerDown={(event) => { event.preventDefault(); setDragged(tag); event.currentTarget.setPointerCapture(event.pointerId); }}
                onPointerUp={(event) => {
                  const target = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>("[data-ocr-language]")?.dataset.ocrLanguage;
                  if (target && dragged) reorder(dragged, target);
                  setDragged(null);
                }} onPointerCancel={() => setDragged(null)}><Icon name="drag" size={12} /></button>
              <span>{LANG_NAMES[tag] ?? tag}</span>
              <button type="button" disabled={index === 0} aria-label={t("ocr.moveUp", { language: tag })}
                className="disabled:opacity-30" onClick={() => reorder(tag, preferred[index - 1])}>↑</button>
              <button type="button" disabled={index === preferred.length - 1} aria-label={t("ocr.moveDown", { language: tag })}
                className="disabled:opacity-30" onClick={() => reorder(tag, preferred[index + 1])}>↓</button>
              <button type="button" aria-label={t("ocr.removeLanguage", { language: tag })} onClick={() => toggle(tag)}><Icon name="close" size={11} /></button>
            </div>)}
            {candidates.map((tag) => <button type="button" key={tag} onClick={() => toggle(tag)}
              className="inline-flex items-center gap-1 rounded-md border border-line bg-field px-2 py-1 text-xs">
              <Icon name="plus" size={11} />{LANG_NAMES[tag] ?? tag}
            </button>)}
          </div>
          <div className="mt-3 flex items-center gap-2">
            <input value={language} placeholder={t("ocr.languageCode")} aria-label={t("ocr.languageCode")}
              className="w-36 rounded border border-line bg-field px-2 py-1 text-xs"
              onChange={(event) => setLanguage(event.target.value)}
              onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); add(); } }} />
            <button type="button" onClick={add} className="text-xs text-accent-ink">{t("ocr.addLanguage")}</button>
          </div>
          {windows && !config.autoDetectLanguage && !preferred.length && <p role="alert" className="mt-2 text-xs text-danger">{t("ocr.languageRequired")}</p>}
          {error && <p role="alert" className="mt-2 text-xs text-danger">{error}</p>}
          {windows && <div className="mt-3 rounded-md border border-line p-3 text-xs">
            <p>{t("ocr.windowsLanguages")}</p>
            {info?.windowsPackageIdentity && !info.ocrLanguages.length && <p role="alert" className="mt-1 text-danger">{t("ocr.languageMissing")}</p>}
            <div className="mt-2 flex gap-3">
              <button type="button" onClick={openLanguages} className="text-accent-ink">{t("ocr.installLanguages")}</button>
              <button type="button" onClick={() => void reload()}>{t("common.refresh")}</button>
            </div>
          </div>}
        </Section>
      </div>
    </div>
  );
}
