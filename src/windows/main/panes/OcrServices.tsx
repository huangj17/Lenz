import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/Icon";
import { ocrApi } from "@/lib/ipc";
import { errorMessage } from "@/lib/errors";
import type { OcrService, TestOcrResult } from "@/lib/types";
import { useConfigStore } from "@/stores/config";
import { Card, Section } from "../components/primitives";

const BAIDU_LANGUAGES = ["CHN_ENG", "ENG", "JAP", "KOR", "FRE", "SPA", "POR", "GER", "ITA", "RUS"];
const fieldClass = "mt-1 w-full rounded-md border border-line bg-field px-2.5 py-1.5 text-xs text-fg outline-none focus:border-accent";

export function OcrServices({ selectedId, onSelect }: { selectedId: string; onSelect: (id: string) => void }) {
  const { t } = useTranslation();
  const config = useConfigStore((s) => s.config.ocr);
  const patch = useConfigStore((s) => s.patch);
  const [adding, setAdding] = useState(false);
  const [addKind, setAddKind] = useState<"baidu" | "vision">("baidu");
  const selected = config.services.find((s) => s.id === selectedId) ?? config.services.find((s) => s.id === config.defaultServiceId) ?? config.services[0];

  const add = async () => {
    setAdding(true);
    const service: OcrService = {
      id: crypto.randomUUID(), name: t(`ocr.kinds.${addKind}`), kind: addKind,
      apiUrl: "", apiKey: "", secretKey: "", model: "", timeoutMs: addKind === "vision" ? 60000 : 30000,
      baiduAccurate: true, baiduLanguage: "CHN_ENG",
    };
    try {
      if (await patch((current) => ({ ocr: { services: [...current.ocr.services, service] } }))) onSelect(service.id);
    } finally { setAdding(false); }
  };

  return <Section title={t("ocr.services")} description={t("ocr.servicesDesc")}>
    <div id="ocr-services" className="grid grid-cols-[145px_minmax(0,1fr)] gap-3">
      <div className="min-w-0">
        <div className="space-y-1" role="group" aria-label={t("ocr.services")}>
          {config.services.map((service) => <button key={service.id} type="button" aria-pressed={service.id === selected?.id}
            onClick={() => onSelect(service.id)} className={"w-full rounded-md border px-2.5 py-2 text-left text-xs " + (service.id === selected?.id ? "border-accent bg-tint" : "border-line bg-card")}>
            <span className="block truncate font-medium">{service.kind === "system" ? t("ocr.kinds.system") : service.name}</span>
            <span className="mt-1 block text-[10px] text-sub">{service.id === config.defaultServiceId ? t("engines.default") : t(`ocr.kinds.${service.kind}`)}</span>
          </button>)}
        </div>
        <label className="mt-3 block text-[11px] text-sub">{t("ocr.addType")}
          <select className={fieldClass} value={addKind} onChange={(event) => setAddKind(event.target.value as "baidu" | "vision")}>
            <option value="baidu">{t("ocr.kinds.baidu")}</option>
            <option value="vision">{t("ocr.kinds.vision")}</option>
          </select>
        </label>
        <Button variant="primary" className="mt-2 w-full justify-center disabled:opacity-50" disabled={adding} onClick={() => void add()}>
          <Icon name="plus" size={12} />{t("engines.add")}
        </Button>
      </div>
      {selected && <OcrServiceEditor key={selected.id} service={selected} defaultId={config.defaultServiceId} onSelect={onSelect} />}
    </div>
  </Section>;
}

function OcrServiceEditor({ service, defaultId, onSelect }: { service: OcrService; defaultId: string; onSelect: (id: string) => void }) {
  const { t } = useTranslation();
  const patch = useConfigStore((s) => s.patch);
  const recognitionProfile = useConfigStore((s) => JSON.stringify([s.config.ocr.preferredLanguages, s.config.ocr.fastMode, s.config.ocr.autoDetectLanguage, s.config.ocr.minConfidence]));
  const [draft, setDraft] = useState(service);
  const [timeout, setTimeout] = useState(String(service.timeoutMs));
  const [showKey, setShowKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<TestOcrResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const generation = useRef(0);
  const pending = useRef<Partial<OcrService>>({});
  const system = service.kind === "system";
  const ready = system || (draft.name.trim() && (service.kind === "baidu"
    ? draft.apiKey.trim() && draft.secretKey.trim() : draft.apiUrl.trim() && draft.model.trim()));

  useEffect(() => () => { generation.current += 1; }, []);
  useEffect(() => {
    generation.current += 1;
    setResult(null);
    setTesting(false);
  }, [recognitionProfile]);
  useEffect(() => {
    if (!Object.keys(pending.current).length && JSON.stringify(service) !== JSON.stringify(draft)) {
      setDraft(service);
      setTimeout(String(service.timeoutMs));
      generation.current += 1;
      setResult(null);
      setTesting(false);
    }
    // 只接收外部保存的变更;自己的保存不使正在进行的测试失效。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [service]);

  const edit = (partial: Partial<OcrService>) => {
    generation.current += 1;
    pending.current = { ...pending.current, ...partial };
    setDraft((current) => ({ ...current, ...partial }));
    setResult(null); setTesting(false); setError(null); setConfirmDelete(false);
  };
  const save = async () => {
    if (!draft.name.trim()) { setError(t("engines.warnName")); return false; }
    const ms = Number(timeout);
    if (!Number.isInteger(ms) || ms < 1000 || ms > 300000) { setError(t("ocr.timeoutHint")); return false; }
    if (!Object.keys(pending.current).length) return true;
    const changes: Partial<OcrService> = { ...pending.current, timeoutMs: ms };
    const saved = await patch((current) => ({ ocr: { services: current.ocr.services.map((s) => s.id === service.id ? { ...s, ...changes } : s) } }));
    if (saved) {
      for (const key of Object.keys(changes) as (keyof OcrService)[]) {
        if (pending.current[key] === changes[key]) delete pending.current[key];
      }
    }
    return saved;
  };
  const localizedError = (message: string) => {
    const key = `ocr.errors.${message}`;
    return t(key, { defaultValue: message.startsWith("ocr_http_") ? t("ocr.httpError", { status: message.slice(9) })
      : message.startsWith("ocr_baidu_") && /^\d+$/.test(message.slice(10)) ? t("ocr.baiduError", { code: message.slice(10) }) : t("ocr.testFailed") });
  };
  const test = async () => {
    const current = ++generation.current;
    const snapshot = { ...draft, timeoutMs: Number(timeout) };
    setTesting(true); setResult(null); setError(null);
    try {
      if (!(await save()) || generation.current !== current) return;
      const next = await ocrApi.test(snapshot);
      if (generation.current === current) setResult(next);
    } catch (value) { if (generation.current === current) setError(localizedError(errorMessage(value))); }
    finally { if (generation.current === current) setTesting(false); }
  };
  const makeDefault = async () => {
    if (!(await save())) return;
    await patch({ ocr: { defaultServiceId: service.id } });
  };
  const remove = async () => {
    if (!confirmDelete) { setConfirmDelete(true); return; }
    const saved = await patch((current) => ({ ocr: {
      services: current.ocr.services.filter((s) => s.id !== service.id),
      ...(current.ocr.defaultServiceId === service.id ? { defaultServiceId: "system" } : {}),
    } }));
    if (saved) onSelect("system");
  };
  const input = (key: "name" | "apiUrl" | "apiKey" | "secretKey" | "model", label: string, placeholder?: string) =>
    <label className="mb-3 block text-[11px] text-sub">{label}
      <input className={fieldClass} value={draft[key]} type={(key === "apiKey" || key === "secretKey") && !showKey ? "password" : "text"}
        autoComplete="off" spellCheck={false} placeholder={placeholder} onChange={(event) => edit({ [key]: event.target.value })} onBlur={() => void save()} />
    </label>;

  return <Card className="min-w-0 self-start p-3.5">
    <div className="mb-3 flex items-center justify-between gap-2">
      <span className="text-sm font-medium">{t(`ocr.kinds.${service.kind}`)}</span>
      {service.id === defaultId && <span className="rounded bg-tint px-2 py-0.5 text-[10px] text-accent-ink">{t("engines.default")}</span>}
    </div>
    {!system && <>
      {input("name", t("engines.name"))}
      {service.kind === "vision" && input("apiUrl", t("ocr.apiUrl"), "https://…/v1")}
      {input("apiKey", "API Key", service.kind === "vision" ? t("ocr.optionalKey") : undefined)}
      {service.kind === "baidu" && <>
        {input("secretKey", "Secret Key")}
        <p className="mb-3 text-[11px] text-sub">{t("ocr.baiduCredentialsHint")}</p>
        <label className="mb-3 block text-[11px] text-sub">{t("ocr.baiduVariant")}
          <select className={fieldClass} value={draft.baiduAccurate ? "accurate" : "general"} onChange={(event) => edit({ baiduAccurate: event.target.value === "accurate" })} onBlur={() => void save()}>
            <option value="accurate">{t("ocr.baiduAccurate")}</option><option value="general">{t("ocr.baiduGeneral")}</option>
          </select>
        </label>
        <label className="mb-3 block text-[11px] text-sub">{t("ocr.languages")}
          <select className={fieldClass} value={draft.baiduLanguage} onChange={(event) => edit({ baiduLanguage: event.target.value })} onBlur={() => void save()}>
            {BAIDU_LANGUAGES.map((code) => <option key={code} value={code}>{t(`ocr.baiduLanguages.${code}`)}</option>)}
          </select>
        </label>
      </>}
      {service.kind === "vision" && input("model", t("engines.model"), t("ocr.visionModelPlaceholder"))}
      <button type="button" onClick={() => setShowKey(!showKey)} className="mb-3 text-xs text-accent-ink">{t(showKey ? "engines.hideKey" : "engines.showKey")}</button>
      <label className="mb-3 block text-[11px] text-sub">{t("engines.timeout")}
        <input className={fieldClass} type="number" min={1000} max={300000} step={1000} value={timeout}
          onChange={(event) => { setTimeout(event.target.value); edit({ timeoutMs: Number(event.target.value) }); }} onBlur={() => void save()} />
      </label>
      <p className="mb-2 text-[11px] text-sub">{t("ocr.timeoutHint")}</p>
      <p className="mb-2 text-[11px] text-sub">{t("engines.keyHint")}</p>
      <p className="mb-3 text-xs text-sub">{t("ocr.uploadNotice")}</p>
    </>}
    <p className="mb-3 text-xs text-sub">{t(system ? "ocr.systemServiceHint" : service.kind === "vision" ? "ocr.visionHint" : "ocr.baiduHint")}</p>
    <div className="flex flex-wrap gap-2">
      <Button disabled={testing} className="disabled:opacity-50" onClick={() => void test()}>{t(testing ? "engines.testing" : "ocr.test")}</Button>
      {service.id !== defaultId && <Button disabled={!ready} className="disabled:opacity-50" onClick={() => void makeDefault()}>{t("engines.setDefault")}</Button>}
      {!system && <Button className="text-danger" onClick={() => void remove()}>{t(confirmDelete ? "engines.confirmDelete" : "engines.delete")}</Button>}
    </div>
    <p className="mt-3 text-[11px] text-sub">{t("ocr.testHint")}</p>
    {error && <p role="alert" className="mt-2 text-xs text-danger">{error}</p>}
    {result && <div role="status" className="mt-3 rounded-md border border-line p-2 text-xs">
      <p className={result.success ? "text-accent-ink" : "text-danger"}>{result.success ? t("ocr.testSuccess", { ms: result.costMs }) : localizedError(result.error ?? "ocr_test_failed")}</p>
      {result.text && <p className="mt-1 max-h-32 overflow-auto whitespace-pre-wrap break-words text-fg">{result.text}</p>}
    </div>}
  </Card>;
}
