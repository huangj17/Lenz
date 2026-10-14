import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LensMark } from "@/components/LensMark";
import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { systemApi } from "@/lib/ipc";
import { useSystemStore } from "@/stores/system";
import { useConfigStore } from "@/stores/config";

export function AboutPane() {
  const { t, i18n } = useTranslation();
  const info = useSystemStore((s) => s.info);
  const update = useSystemStore((s) => s.update);
  const setUpdate = useSystemStore((s) => s.setUpdate);
  const setError = useConfigStore((s) => s.setError);
  const [checking, setChecking] = useState(false);
  const [notices, setNotices] = useState<string | null>(null);
  const [noticeIndex, setNoticeIndex] = useState(0);
  const [loadingLicense, setLoadingLicense] = useState(false);
  const checkUpdate = async () => {
    setChecking(true);
    try { setUpdate(await systemApi.checkUpdate()); } catch (error) { setError(error); }
    finally { setChecking(false); }
  };
  const openLicenses = async () => {
    setLoadingLicense(true);
    try { setNotices(await systemApi.licenses()); setNoticeIndex(0); } catch (error) { setError(error); }
    finally { setLoadingLicense(false); }
  };
  const sections = notices?.split("========================================\n") ?? [];
  const date = info?.buildDate ? new Date(Number(info.buildDate) * 1000).toLocaleDateString(i18n.language) : "…";
  return (
    <div id="about" className="flex items-start gap-7">
      <LensMark size={88} />
      <div className="min-w-0 flex-1">
        <div className="text-[28px] font-semibold tracking-[-0.02em]">
          Lenz <span className="ml-2 font-mono text-sm font-normal text-sub">
            {info ? "v" + info.version : "…"} ({t("about.beta")})
          </span>
        </div>
        <div className="mt-1 text-[13px] text-sub">{t("about.tagline")}</div>
        <div className="mt-0.5 text-[13px] leading-normal text-sub">{t("about.stack")}</div>
        <dl className="mt-3 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs text-sub">
          <dt>{t("about.buildDate")}</dt><dd>{date}</dd>
          <dt>Bundle ID</dt><dd className="break-all font-mono">{info?.identifier ?? "…"}</dd>
          <dt>{t("ocr.backend")}</dt><dd>{info?.platform === "windows" ? "Windows OCR" : "Apple Vision"}</dd>
        </dl>
        <div className="mt-4 flex flex-wrap gap-2">
          <Button variant="warm" onClick={() => void systemApi.open("repository").catch(setError)}>
            <Icon name="external" size={12} />GitHub
          </Button>
          <Button variant="secondary" onClick={() => void systemApi.open("releases").catch(setError)}>{t("about.changelog")}</Button>
          <Button variant="secondary" disabled={checking} onClick={() => void checkUpdate()}>
            {t(checking ? "about.checking" : "about.checkUpdate")}
          </Button>
          <Button variant="secondary" disabled={loadingLicense} onClick={() => void openLicenses()}>{t("about.licenses")}</Button>
        </div>
        {update && <div role="status" className="mt-4 rounded-md border border-line p-3 text-xs">
          <p>{update.available ? t("about.updateAvailable", { version: update.latestVersion })
            : t(update.published ? "about.upToDate" : "about.noRelease")}</p>
          {update.available && <Button variant="primary" className="mt-2" onClick={() => void systemApi.open("latestRelease").catch(setError)}>{t("about.openRelease")}</Button>}
          {update.notes && <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap font-sans">{update.notes}</pre>}
        </div>}
        <div className="mt-[26px] max-w-[440px] rounded-[9px] border-[0.5px] border-line bg-card p-3.5 text-xs leading-relaxed text-sub">
          {t("about.privacy")}
        </div>
      </div>
      {notices !== null && <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-8"
        onKeyDown={(event) => { if (event.key === "Escape") { event.stopPropagation(); setNotices(null); } }}>
        <div role="dialog" aria-modal="true" aria-label={t("about.licenses")} className="flex max-h-full w-full max-w-3xl flex-col rounded-xl border border-line bg-bg p-5 shadow-xl">
          <div className="mb-3 flex items-center justify-between"><h2 className="font-medium">{t("about.licenses")}</h2>
            <button type="button" autoFocus aria-label={t("common.close")} onClick={() => setNotices(null)}><Icon name="close" size={16} /></button></div>
          <select aria-label={t("about.licensePackage")} value={noticeIndex} onChange={(event) => setNoticeIndex(Number(event.target.value))}
            className="mb-3 w-full rounded border border-line bg-field p-2 text-xs">
            {sections.map((section, index) => <option key={index} value={index}>{index === 0 ? "Lenz · MIT License" : section.split("\n")[0]}</option>)}
          </select>
          <pre className="min-h-0 overflow-auto whitespace-pre-wrap break-words text-xs leading-relaxed">{sections[noticeIndex]}</pre>
        </div>
      </div>}
    </div>
  );
}
