import { useEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from "react";
import { useTranslation } from "react-i18next";
import { clsx } from "clsx";
import { InputField, PromptField, SelectField } from "../components/primitives";
import { Button } from "@/components/ui/button";
import { Toggle } from "@/components/ui/toggle";
import { Icon } from "@/components/Icon";
import { useConfigStore } from "@/stores/config";
import { engineApi } from "@/lib/ipc";
import type {
  TestEngineResult,
  TranslatorEngine,
  TranslatorEngineKind,
} from "@/lib/types";

const KINDS: TranslatorEngineKind[] = [
  "openai",
  "google",
  "deepl",
  "youdao",
  "baidu",
  "microsoft",
  "apple",
];

/** 引擎徽标按类型配色(贴近各家品牌色调) */
const KIND_BADGE: Record<TranslatorEngineKind, string> = {
  openai: "linear-gradient(135deg,#19c37d,#0e8f5e)",
  google: "linear-gradient(135deg,#4285f4,#2b6cd4)",
  deepl: "linear-gradient(135deg,#1c3a52,#0f2740)",
  youdao: "linear-gradient(135deg,#e0564f,#c23a34)",
  baidu: "linear-gradient(135deg,#2932e1,#1b21a8)",
  microsoft: "linear-gradient(135deg,#2aa0d6,#1f7fb0)",
  apple: "linear-gradient(135deg,#8a8f98,#5f636b)",
};

const DEFAULT_PROMPT =
  "你是一位专业译者。请将文本从 {sourceLang} 翻译为 {targetLang}。保留原有格式与术语。";

/** 引擎名首字母作徽标(LM Studio（本地）→ LM) */
function initials(name: string): string {
  const s = name.replace(/[（(].*$/, "").trim();
  return s.slice(0, 2).toUpperCase() || "AI";
}

/** 必填校验:返回缺失项的 i18n key 列表(非阻断,仅提示) */
function validate(e: TranslatorEngine): string[] {
  const issues: string[] = [];
  if (!e.name.trim()) issues.push("engines.warnName");
  if (e.kind === "openai" && !(e.model ?? "").trim()) issues.push("engines.warnModel");
  if (e.kind === "youdao" && !e.apiKey.includes(":")) issues.push("engines.warnYoudaoKey");
  if (e.kind === "baidu" && !e.apiKey.includes(":")) issues.push("engines.warnBaiduKey");
  // 谷歌留空 Key 即用内置默认 Key,故不作必填校验。
  if ((e.kind === "deepl" || e.kind === "microsoft") && !e.apiKey.trim())
    issues.push("engines.warnKey");
  if (e.kind === "microsoft" && !(e.region ?? "").trim()) issues.push("engines.warnRegion");
  return issues;
}

type EnginePatch = { engines: TranslatorEngine[]; defaultEngineId?: string };

/**
 * 文本翻译引擎 —— 左列表(可拖动排序)+ 右编辑器(可编辑表单 + 测试 / 设默认 / 删除)。
 *
 * 写入约定:merge patch 对数组整体替换,故任何引擎改动都整体回写 `translator.engines`;
 * 提交时用 `useConfigStore.getState()` 读最新值,避免事件闭包里的旧 engines。
 * 不变量:默认引擎恒为「已启用」——设默认会顺带启用,停用默认会改派给下一个启用引擎。
 */
export function EnginesPane() {
  const { t } = useTranslation();
  const engines = useConfigStore((s) => s.config.translator.engines);
  const defaultId = useConfigStore((s) => s.config.translator.defaultEngineId);
  const patch = useConfigStore((s) => s.patch);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = engines.find((e) => e.id === selectedId) ?? engines[0] ?? null;

  const [draft, setDraft] = useState<TranslatorEngine | null>(null);
  const [maxTokensText, setMaxTokensText] = useState("");
  const [tempText, setTempText] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<TestEngineResult | null>(null);
  const [confirmDel, setConfirmDel] = useState(false);
  const [dragId, setDragId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);
  const [overAfter, setOverAfter] = useState(false);
  const delTimer = useRef<number | undefined>(undefined);
  const listRef = useRef<HTMLDivElement>(null);

  // 初次有引擎时固定选中第一个,避免重排后 selected 随 engines[0] 漂移。
  useEffect(() => {
    if (!selectedId && engines.length) setSelectedId(engines[0].id);
  }, [selectedId, engines]);

  // 切换选中引擎:重置草稿与瞬态(超时输入 / Key 显隐 / 测试结果 / 删除确认)。
  useEffect(() => {
    setDraft(selected ? { ...selected } : null);
    setMaxTokensText(selected?.maxTokens != null ? String(selected.maxTokens) : "");
    setTempText(selected?.temperature != null ? String(selected.temperature) : "");
    setShowKey(false);
    setTesting(false);
    setTestResult(null);
    setConfirmDel(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected?.id]);

  // 卸载时清理删除确认计时器。
  useEffect(() => () => window.clearTimeout(delTimer.current), []);

  const liveTranslator = () => useConfigStore.getState().config.translator;
  const writeEngines = (next: TranslatorEngine[]) =>
    patch({ translator: { engines: next } });

  // 任意编辑都让上一次测试结果失效(避免「改了配置却还显示 ✓」)。
  const invalidateTest = () => setTestResult(null);

  // 即时提交(下拉等离散操作):更新草稿并整体回写。
  const commitNow = (partial: Partial<TranslatorEngine>) => {
    if (!draft) return;
    invalidateTest();
    const merged = { ...draft, ...partial };
    setDraft(merged);
    void writeEngines(liveTranslator().engines.map((e) => (e.id === merged.id ? merged : e)));
  };

  // 文本类:本地改草稿,失焦再整体回写(避免逐键写盘 + 逐键热重载)。
  const editDraft = (partial: Partial<TranslatorEngine>) => {
    invalidateTest();
    setDraft((d) => (d ? { ...d, ...partial } : d));
  };
  const commitDraft = () => {
    if (!draft) return;
    const cur = liveTranslator().engines;
    const orig = cur.find((e) => e.id === draft.id);
    if (orig && JSON.stringify(orig) === JSON.stringify(draft)) return;
    void writeEngines(cur.map((e) => (e.id === draft.id ? draft : e)));
  };

  // 启用 / 停用:停用当前默认引擎时,默认改派给下一个仍启用的引擎(默认恒启用)。
  const setEnabled = (v: boolean) => {
    if (!draft) return;
    invalidateTest();
    const merged = { ...draft, enabled: v };
    setDraft(merged);
    const tr = liveTranslator();
    const nextEngines = tr.engines.map((e) => (e.id === merged.id ? merged : e));
    const next: EnginePatch = { engines: nextEngines };
    if (!v && tr.defaultEngineId === merged.id) {
      next.defaultEngineId = nextEngines.find((e) => e.enabled && e.id !== merged.id)?.id ?? "";
    }
    void patch({ translator: next });
  };

  // 设为默认:顺带启用,保证默认引擎可用。
  const setAsDefault = () => {
    if (!draft) return;
    invalidateTest();
    const merged = { ...draft, enabled: true };
    setDraft(merged);
    const cur = liveTranslator().engines;
    void patch({
      translator: {
        engines: cur.map((e) => (e.id === merged.id ? merged : e)),
        defaultEngineId: merged.id,
      },
    });
  };

  // 最大输出 token:留空 = 不限(undefined),否则夹到 [1, 1000000]。
  const commitMaxTokens = () => {
    if (!draft) return;
    const txt = maxTokensText.trim();
    let val: number | undefined;
    if (txt === "") val = undefined;
    else {
      const n = parseInt(txt, 10);
      val = Number.isFinite(n) ? Math.min(1000000, Math.max(1, n)) : draft.maxTokens;
    }
    setMaxTokensText(val == null ? "" : String(val));
    if (val !== draft.maxTokens) commitNow({ maxTokens: val });
  };

  // 温度:留空 = 默认(undefined → 后端 0.1),否则夹到 [0, 2]。
  const commitTemp = () => {
    if (!draft) return;
    const txt = tempText.trim();
    let val: number | undefined;
    if (txt === "") val = undefined;
    else {
      const n = parseFloat(txt);
      val = Number.isFinite(n) ? Math.min(2, Math.max(0, n)) : draft.temperature;
    }
    setTempText(val == null ? "" : String(val));
    if (val !== draft.temperature) commitNow({ temperature: val });
  };

  // 插到目标行「之前 / 之后」(由光标落在上下半区决定),支持拖到末尾。
  const reorder = (fromId: string, toId: string, after: boolean) => {
    if (fromId === toId) return;
    const next = [...liveTranslator().engines];
    const from = next.findIndex((e) => e.id === fromId);
    if (from < 0) return;
    const [moved] = next.splice(from, 1);
    let insertAt = next.findIndex((e) => e.id === toId);
    if (insertAt < 0) return;
    if (after) insertAt += 1;
    next.splice(insertAt, 0, moved);
    void writeEngines(next);
  };

  // 命中测试:光标 Y 落在哪一行、上半区(之前)还是下半区(之后)。
  const hitTest = (clientY: number): { id: string; after: boolean } | null => {
    const container = listRef.current;
    if (!container) return null;
    const rows = Array.from(
      container.querySelectorAll<HTMLElement>("[data-engine-id]"),
    );
    if (!rows.length) return null;
    for (const row of rows) {
      const r = row.getBoundingClientRect();
      if (clientY >= r.top && clientY <= r.bottom) {
        return { id: row.dataset.engineId!, after: clientY > r.top + r.height / 2 };
      }
    }
    // 落在列表上/下空白:归到首/末行。
    const first = rows[0].getBoundingClientRect();
    if (clientY < first.top) return { id: rows[0].dataset.engineId!, after: false };
    return { id: rows[rows.length - 1].dataset.engineId!, after: true };
  };

  // 指针拖拽排序:不用 HTML5 DnD(Tauri/WKWebView 会被文件拖放拦截、且光标不变)。
  const startDrag = (ev: ReactMouseEvent, id: string) => {
    ev.preventDefault();
    ev.stopPropagation();
    setDragId(id);
    document.body.style.cursor = "grabbing";
    const onMove = (m: MouseEvent) => {
      const hit = hitTest(m.clientY);
      if (hit && hit.id !== id) {
        setOverId(hit.id);
        setOverAfter(hit.after);
      } else {
        setOverId(null);
      }
    };
    const onUp = (u: MouseEvent) => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      const hit = hitTest(u.clientY);
      if (hit && hit.id !== id) reorder(id, hit.id, hit.after);
      setDragId(null);
      setOverId(null);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  };

  const addEngine = () => {
    const ne: TranslatorEngine = {
      id: crypto.randomUUID(),
      name: t("engines.newName"),
      kind: "openai",
      enabled: true,
      apiUrl: "",
      apiKey: "",
      model: "",
      timeoutMs: 30000,
    };
    void writeEngines([...liveTranslator().engines, ne]);
    setSelectedId(ne.id);
  };

  const removeEngine = () => {
    if (!draft) return;
    const tr = liveTranslator();
    const next = tr.engines.filter((e) => e.id !== draft.id);
    const p: EnginePatch = { engines: next };
    if (tr.defaultEngineId === draft.id) {
      p.defaultEngineId = next.find((e) => e.enabled)?.id ?? next[0]?.id ?? "";
    }
    void patch({ translator: p });
    setSelectedId(next[0]?.id ?? null);
  };

  // 两步删除:首次点击进入确认态(3.5s 自动取消),再次点击才删除。
  const onDeleteClick = () => {
    if (confirmDel) {
      window.clearTimeout(delTimer.current);
      setConfirmDel(false);
      removeEngine();
      return;
    }
    setConfirmDel(true);
    window.clearTimeout(delTimer.current);
    delTimer.current = window.setTimeout(() => setConfirmDel(false), 3500);
  };

  const runTest = async () => {
    if (!draft) return;
    setTesting(true);
    setTestResult(null);
    try {
      setTestResult(await engineApi.test(draft));
    } catch (err) {
      setTestResult({
        ok: false,
        latencyMs: 0,
        sample: "",
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setTesting(false);
    }
  };

  const issues = draft ? validate(draft) : [];

  return (
    <div className="flex h-full flex-col">
      <div className="mb-[14px] flex items-end justify-between">
        <div className="max-w-[440px] text-xs text-sub">{t("engines.desc")}</div>
        <Button variant="primary" onClick={addEngine}>
          <Icon name="plus" size={13} />
          {t("engines.add")}
        </Button>
      </div>

      {engines.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 text-center">
          <div className="flex size-12 items-center justify-center rounded-xl bg-tint text-accent-ink">
            <Icon name="cpu" size={22} />
          </div>
          <div className="text-[13px] font-medium">{t("engines.emptyTitle")}</div>
          <div className="max-w-[280px] text-xs text-sub">{t("engines.emptyDesc")}</div>
          <Button variant="primary" onClick={addEngine}>
            <Icon name="plus" size={13} />
            {t("engines.add")}
          </Button>
        </div>
      ) : (
        <div className="grid min-h-0 flex-1 grid-rows-1 grid-cols-[260px_1fr] gap-[14px]">
          {/* 引擎列表(拖动排序) */}
          <div
            ref={listRef}
            className="min-h-0 overflow-y-auto rounded-[9px] border-[0.5px] border-line bg-card"
          >
            {engines.map((e, i) => {
              const active = selected?.id === e.id;
              const isOver = overId === e.id && !!dragId && dragId !== e.id;
              return (
                <div
                  key={e.id}
                  data-engine-id={e.id}
                  onClick={() => setSelectedId(e.id)}
                  className={clsx(
                    "group flex cursor-pointer items-center gap-2 py-2.5 pr-2 pl-[11px] text-left",
                    i < engines.length - 1 && "border-b-[0.5px] border-line",
                    dragId === e.id && "opacity-50",
                  )}
                  style={{
                    background: active ? "var(--tint-soft)" : "transparent",
                    borderLeft: active
                      ? "3px solid var(--lens)"
                      : "3px solid transparent",
                    // 拖拽悬停:落点在上半区 → 顶部线,下半区 → 底部线(inset 阴影,不重排)。
                    boxShadow: isOver
                      ? overAfter
                        ? "inset 0 -2px 0 var(--accent)"
                        : "inset 0 2px 0 var(--accent)"
                      : undefined,
                  }}
                >
                  <span
                    onMouseDown={(ev) => startDrag(ev, e.id)}
                    onClick={(ev) => ev.stopPropagation()}
                    title={t("engines.reorder")}
                    className="flex shrink-0 cursor-grab items-center text-sub opacity-40 transition-opacity hover:opacity-80 active:cursor-grabbing"
                  >
                    <Icon name="drag" size={14} />
                  </span>
                  <span
                    className={clsx("size-2 shrink-0 rounded-full", e.enabled && "ln-glow")}
                    style={{
                      background: e.enabled
                        ? "var(--sage)"
                        : "color-mix(in srgb, var(--fg) 20%, transparent)",
                    }}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <div className="flex-1 truncate text-[13px] font-medium">
                        {e.name}
                      </div>
                      {e.id === defaultId && (
                        <span className="shrink-0 rounded bg-accent px-1.5 py-px font-mono text-[9.5px] font-semibold uppercase tracking-[0.08em] text-accent-fg">
                          {t("engines.default")}
                        </span>
                      )}
                    </div>
                    {e.model && (
                      <div className="mt-[3px] truncate font-mono text-[11px] text-sub">
                        {e.model}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>

          {/* 引擎编辑器 */}
          {draft && (
            <div className="min-h-0 overflow-y-auto rounded-[9px] border-[0.5px] border-line bg-card p-4">
              <div className="mb-3 flex items-center gap-2.5">
                <div
                  className="flex size-[26px] shrink-0 items-center justify-center rounded-md font-mono text-[11px] font-bold text-white"
                  style={{ background: KIND_BADGE[draft.kind] }}
                >
                  {initials(draft.name)}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold">
                    {draft.name || t("engines.newName")}
                  </div>
                  <div className="font-mono text-[11px] text-sub">
                    {t(`engines.kinds.${draft.kind}`)}
                  </div>
                </div>
                {draft.id === defaultId ? (
                  <span className="shrink-0 rounded bg-accent px-2 py-1 font-mono text-[9.5px] font-semibold uppercase tracking-[0.08em] text-accent-fg">
                    {t("engines.default")}
                  </span>
                ) : (
                  <Button variant="secondary" onClick={setAsDefault} className="shrink-0">
                    <Icon name="star" size={12} />
                    {t("engines.setDefault")}
                  </Button>
                )}
                <Toggle
                  checked={draft.enabled}
                  onCheckedChange={setEnabled}
                  aria-label={draft.name || t("engines.newName")}
                />
              </div>

              <InputField
                label={t("engines.name")}
                value={draft.name}
                onChange={(v) => editDraft({ name: v })}
                onBlur={commitDraft}
                placeholder={t("engines.newName")}
              />

              {/* 类型对内置引擎是固定的(且已在头部副标题展示),仅自定义引擎可改。 */}
              {!draft.id.startsWith("builtin-") && (
                <SelectField
                  label={t("engines.type")}
                  value={draft.kind}
                  onChange={(v) => commitNow({ kind: v as TranslatorEngineKind })}
                  options={KINDS.map((k) => ({
                    value: k,
                    label: t(`engines.kinds.${k}`),
                  }))}
                />
              )}

              {draft.kind === "apple" ? (
                <div className="rounded-md border-[0.5px] border-line bg-field px-3 py-2.5 text-xs text-sub">
                  {t("engines.appleNote")}
                </div>
              ) : (
                <>
                  <InputField
                    label="API URL"
                    mono
                    value={draft.apiUrl}
                    onChange={(v) => editDraft({ apiUrl: v })}
                    onBlur={commitDraft}
                    placeholder={t("engines.urlPlaceholder")}
                  />
                  <InputField
                    label="API KEY"
                    mono
                    type={showKey ? "text" : "password"}
                    autoComplete="off"
                    value={draft.apiKey}
                    onChange={(v) => editDraft({ apiKey: v })}
                    onBlur={commitDraft}
                    placeholder={
                      draft.kind === "youdao"
                        ? "appKey:appSecret"
                        : draft.kind === "baidu"
                          ? "appid:secret"
                          : t("engines.keyPlaceholder")
                    }
                    hint={
                      draft.kind === "youdao"
                        ? t("engines.youdaoKeyHint")
                        : draft.kind === "baidu"
                          ? t("engines.baiduKeyHint")
                          : draft.kind === "google"
                            ? t("engines.googleKeyHint")
                            : t("engines.keyHint")
                    }
                    trailing={
                      <button
                        type="button"
                        onClick={() => setShowKey((v) => !v)}
                        className="shrink-0 cursor-pointer border-0 bg-transparent text-[11px] text-sub hover:text-fg"
                      >
                        {showKey ? t("engines.hideKey") : t("engines.showKey")}
                      </button>
                    }
                  />

                  {draft.kind === "microsoft" && (
                    <InputField
                      label={t("engines.region")}
                      mono
                      value={draft.region ?? ""}
                      onChange={(v) => editDraft({ region: v })}
                      onBlur={commitDraft}
                      placeholder="eastasia"
                    />
                  )}

                  {draft.kind === "openai" && (
                    <>
                      <InputField
                        label={t("engines.model")}
                        mono
                        value={draft.model ?? ""}
                        onChange={(v) => editDraft({ model: v })}
                        onBlur={commitDraft}
                        placeholder={t("engines.modelPlaceholder")}
                      />
                      <PromptField
                        label={t("engines.systemPrompt")}
                        value={draft.systemPrompt ?? ""}
                        onChange={(v) => editDraft({ systemPrompt: v })}
                        onBlur={commitDraft}
                        placeholder={DEFAULT_PROMPT}
                        rows={3}
                      />
                      <div className="grid grid-cols-2 gap-3">
                        <InputField
                          label={t("engines.maxTokens")}
                          mono
                          type="number"
                          value={maxTokensText}
                          onChange={(v) => {
                            invalidateTest();
                            setMaxTokensText(v);
                          }}
                          onBlur={commitMaxTokens}
                          placeholder={t("engines.unlimited")}
                          hint={t("engines.maxTokensHint")}
                        />
                        <InputField
                          label={t("engines.temperature")}
                          mono
                          type="number"
                          value={tempText}
                          onChange={(v) => {
                            invalidateTest();
                            setTempText(v);
                          }}
                          onBlur={commitTemp}
                          placeholder="0.1"
                          hint={t("engines.temperatureHint")}
                        />
                      </div>
                    </>
                  )}
                </>
              )}

              {issues.length > 0 && (
                <div className="mt-1 flex flex-col gap-1 rounded-md border-[0.5px] border-line bg-field px-3 py-2">
                  {issues.map((k) => (
                    <div
                      key={k}
                      className="flex items-center gap-1.5 text-[11px]"
                      style={{ color: "var(--danger)" }}
                    >
                      <Icon name="info" size={12} className="shrink-0" />
                      {t(k)}
                    </div>
                  ))}
                </div>
              )}

              <div className="mt-4 flex items-center gap-2">
                {draft.kind !== "apple" && (
                  <Button
                    variant="secondary"
                    onClick={runTest}
                    disabled={testing}
                    className={testing ? "opacity-60" : undefined}
                  >
                    {testing ? t("engines.testing") : t("engines.test")}
                  </Button>
                )}
                <Button
                  variant="secondary"
                  onClick={onDeleteClick}
                  style={{ color: "var(--danger)" }}
                >
                  <Icon name="trash" size={12} />
                  {confirmDel ? t("engines.confirmDelete") : t("engines.delete")}
                </Button>
                {testResult && (
                  <span
                    className={clsx(
                      "min-w-0 flex-1 truncate text-xs",
                      testResult.ok ? "text-sage" : "text-danger",
                    )}
                    title={testResult.ok ? testResult.sample : (testResult.error ?? "")}
                  >
                    {testResult.ok
                      ? `✓ ${t("engines.testOk", { ms: testResult.latencyMs, sample: testResult.sample })}`
                      : `✗ ${t("engines.testFail", { error: testResult.error ?? "" })}`}
                  </span>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
