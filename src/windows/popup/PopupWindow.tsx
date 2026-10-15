import { errorMessage } from "@/lib/errors";
import { tencentOcrErrorKey } from "@/lib/ocr-errors";
import { PopupSessions } from "@/lib/popupSessions";
import { PopupTranslations, popupTranslationRows, type PopupTranslationPanel, type PopupTranslationError, type PopupTranslationRow } from "@/lib/popupTranslations";
import { PopupServiceRow } from "./PopupServiceRow";
/**
 * 翻译结果弹框窗口(spec 06 §3 / spec 09 §3)。视觉对齐设计交接包 popup-f1/f2:
 * 暖色毛玻璃卡片 + Lens 头部 + 原文块(识别语言 chip + 字数)+ 语言切换条 +
 * 引擎结果行(徽标 + 名称 + 默认 + 用时)+ 底部 esc 提示。
 *
 * 订阅 `screenshot://progress|translating|result|error`,截图/OCR 使用整体进度,
 * 翻译阶段按已启用服务分别展示加载、结果和错误。
 * 头部为窗口拖拽区(`data-tauri-drag-region`)。内容渲染后用 `setSize` 动态调高;
 * ESC / 失焦 / 关闭隐藏;复制译文;翻译失败展示原文 + 重试;权限缺失引导打开系统设置。
 */
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { Icon, Kbd } from "@/components/Icon";
import { LensMark } from "@/components/LensMark";
import { Select, type SelectOption } from "@/components/Select";
import { isLenzError, emit, Events, listenEvent, screenshotApi, selectionApi, translateApi } from "@/lib/ipc";
import { TRANSLATE_LANGUAGES } from "@/lib/languages";
import { useConfigStore } from "@/stores/config";
import type { PopupServiceUiConfig, ScreenshotErrorPayload } from "@/lib/types";

/** 弹框可拖拽宽度上/下限(逻辑像素),与后端 `POPUP_MIN/MAX_WIDTH` 一致。 */
const POPUP_MIN_WIDTH = 300;
const POPUP_MAX_WIDTH = 720;

type View =
  | { kind: "loading"; stage: "capture" | "ocr" | "translate" }
  | PopupTranslationPanel
  | ({ kind: "error" } & ScreenshotErrorPayload);

/** BCP-47 → 当前 UI 语言下的语言名;失败回退原 code。`null` 交由调用方补「自动检测」标签。 */
function langName(code: string | null | undefined, uiLocale: string): string {
  if (!code) return "";
  try {
    return new Intl.DisplayNames([uiLocale], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}

async function hidePopup() {
  try {
    await getCurrentWindow().hide();
  } catch {
    /* ignore */
  }
}

/** 头部:Lens 品牌 + 关闭。整行作为拖拽区。 */
function Header({ onClose, onDragStart }: { onClose: () => void; onDragStart: () => void }) {
  const { t } = useTranslation();
  return (
    <div
      data-tauri-drag-region
      onMouseDown={onDragStart}
      className="flex shrink-0 items-center gap-2 border-b border-line px-3 py-2.5"
    >
      <LensMark size={15} />
      <span className="text-[12px] font-semibold tracking-tight text-fg">Lenz</span>
      <button
        type="button"
        onClick={onClose}
        aria-label={t("popup.close")}
        className="ml-auto grid h-[22px] w-[22px] place-items-center rounded-md text-sub hover:bg-field hover:text-fg"
      >
        <Icon name="close" size={13} />
      </button>
    </div>
  );
}

export function PopupWindow() {
  const { t, i18n } = useTranslation();
  const engines = useConfigStore((s) => s.config.translator.engines);
  const targetLang = useConfigStore((s) => s.config.translator.targetLang);
  const defaultEngineId = useConfigStore((s) => s.config.translator.defaultEngineId);
  const loaded = useConfigStore((s) => s.loaded);
  const patch = useConfigStore((s) => s.patch);

  const [view, setView] = useState<View>({ kind: "loading", stage: "capture" });
  // 复制反馈按 original / translation:{engineId} 区分,避免复制到其他服务的译文。
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  // 原文可编辑草稿:同步自最近一次结果的原文;用户可改写后回车重译。
  const [draft, setDraft] = useState("");
  const originalRef = useRef<HTMLTextAreaElement>(null);
  // 供失焦回调读取最新状态(回调在挂载时绑定,闭包内 view 会过期)。
  const viewRef = useRef(view);
  viewRef.current = view;
  // 前端请求代号配合后端会话:过滤 IPC 途中迟到的响应和事件。
  const translateGenerationRef = useRef(0);
  const sessions = useRef(new PopupSessions());
  const translations = useRef(new PopupTranslations(translateApi.popup, setView, translateApi.cancelPopupRequest));
  const preferenceOverrides = useRef<Record<string, PopupServiceUiConfig>>({});
  const preferenceTickets = useRef<Record<string, number>>({});
  const nextPreferenceTicket = useRef(0);
  const [preferenceError, setPreferenceError] = useState<{ engineId: string; message: string } | null>(null);
  const serviceSettings = () => ({
    ...useConfigStore.getState().config.ui.popup.services,
    ...preferenceOverrides.current,
  });
  const cancelAndHide = useCallback(() => {
    translateGenerationRef.current += 1;
    sessions.current.cancel();
    translations.current.clear();
    void screenshotApi.cancel();
    void hidePopup();
  }, []);
  // 拖拽窗口时,macOS 原生拖拽会瞬间让窗口失焦;此时的失焦不应触发「失焦即隐藏」,
  // 否则一拖头部就被当成「点击窗口外」而消失。用「拖拽宽限时间戳」而非布尔标记:拖拽
  // 起始 + 拖拽中的每次 onMoved 都把宽限截止时间往后推;失焦回调只要仍在宽限期内就
  // 跳过隐藏。这样与「失焦事件经 IPC 异步送达」的时机解耦,避免 mouseup 先复位标记、
  // 迟到的失焦再误隐藏(旧布尔方案的竞态)。
  const dragGuardUntilRef = useRef(0);
  const DRAG_GUARD_MS = 600;
  const bumpDragGuard = useCallback(() => {
    dragGuardUntilRef.current = Date.now() + DRAG_GUARD_MS;
  }, []);

  // ── 右缘拖拽调节窗口宽度 ───────────────────────────────────────────────
  // 拖拽中实时调窗口宽(rAF 节流,走后端 resize_popup_width);松手时把最终宽度 patch 进
  // config(ui.popup.width),下次打开 popup 由后端按此宽度摆放,实现「记忆宽度」。
  const widthDragRef = useRef<{ startX: number; startW: number } | null>(null);
  const pendingWidthRef = useRef(0);
  const widthRafRef = useRef(0);
  const flushWidth = useCallback(() => {
    widthRafRef.current = 0;
    void screenshotApi.resizePopupWidth(pendingWidthRef.current);
  }, []);
  const onGripDown = useCallback(
    (e: React.PointerEvent) => {
      e.preventDefault();
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      // screenX 用屏幕绝对坐标,不受窗口本身随拖拽变宽影响;startW 取当前窗口逻辑宽。
      widthDragRef.current = { startX: e.screenX, startW: window.innerWidth };
      pendingWidthRef.current = window.innerWidth;
      bumpDragGuard();
    },
    [bumpDragGuard],
  );
  const onGripMove = useCallback((e: React.PointerEvent) => {
    const d = widthDragRef.current;
    if (!d) return;
    const w = Math.round(
      Math.min(POPUP_MAX_WIDTH, Math.max(POPUP_MIN_WIDTH, d.startW + (e.screenX - d.startX))),
    );
    pendingWidthRef.current = w;
    if (!widthRafRef.current) widthRafRef.current = requestAnimationFrame(flushWidth);
  }, [flushWidth]);
  const onGripUp = useCallback(
    (e: React.PointerEvent) => {
      const d = widthDragRef.current;
      widthDragRef.current = null;
      if (widthRafRef.current) {
        cancelAnimationFrame(widthRafRef.current);
        widthRafRef.current = 0;
      }
      try {
        (e.target as HTMLElement).releasePointerCapture(e.pointerId);
      } catch {
        /* ignore */
      }
      if (!d) return;
      const w = pendingWidthRef.current || window.innerWidth;
      void screenshotApi.resizePopupWidth(w); // 落定最终宽度
      void patch({ ui: { popup: { width: Math.round(w) } } }); // 持久化,供下次打开
    },
    [patch],
  );

  // 新结果/翻译中到达时,把可编辑原文重置为该原文(loading/error 不覆盖用户草稿)。
  const incomingOriginal =
    view.kind === "translations" ? view.original : null;
  useEffect(() => {
    if (incomingOriginal !== null) setDraft(incomingOriginal);
  }, [incomingOriginal]);

  // ── 订阅 F2 / F1 事件 ─────────────────────────────────────────────────
  useEffect(() => {
    // Read the actual configuration before acknowledging popup readiness, so a
    // cold-start native event can snapshot every enabled engine.
    if (!loaded) return;
    let active = true;
    const enabledIds = () => useConfigStore.getState().config.translator.engines
      .filter((engine) => engine.enabled).map((engine) => engine.id);
    const subs = [
      listenEvent(Events.ScreenshotProgress, (e) => sessions.current.event(e.payload.sessionId ?? 0, () => {
        translateGenerationRef.current += 1;
        translations.current.clear();
        setCopiedId(null);
        setView({ kind: "loading", stage: e.payload.stage });
      })),
      listenEvent(Events.ScreenshotTranslating, (e) => sessions.current.event(e.payload.sessionId ?? 0, () => {
        translateGenerationRef.current += 1;
        const p = e.payload;
        setCopiedId(null);
        translations.current.start({ ...p, sessionId: p.sessionId ?? 0 }, enabledIds(), p.engineId, p.engineId,
          serviceSettings(), p.requestId);
      })),
      listenEvent(Events.ScreenshotResult, (e) => sessions.current.event(e.payload.sessionId ?? 0, () => {
        const r = e.payload;
        const input = { ...r, sessionId: r.sessionId ?? 0 };
        // Cache replay may arrive without a preceding translating event. Reuse
        // that primary result and request the other currently enabled services.
        if (!translations.current.matches(input)) {
          translateGenerationRef.current += 1;
          setCopiedId(null);
          translations.current.start(input, enabledIds(), r.engineId, r.engineId, serviceSettings());
        }
        translations.current.nativeSuccess(input.sessionId, r.engineId, {
          translations: [r.translation], detectedSourceLang: r.sourceLang,
          engineId: r.engineId, costMs: r.cost.translate,
        });
      })),
      listenEvent(Events.ScreenshotError, (e) => sessions.current.event(e.payload.sessionId ?? 0, () => {
        const p = e.payload;
        if (p.stage === "translate" && translations.current.nativeFailure(p.sessionId ?? 0, p)) return;
        translateGenerationRef.current += 1;
        translations.current.clear();
        setView({ kind: "error", ...p });
      })),
    ];
    // 四个监听全部挂上后,通知后端 popup 可接收事件——关闭冷启动首帧竞态:
    // 首次划词/截图时后端会先等这个 `popup://ready` 再投递 progress/result(方案 A)。
    void Promise.all(subs).then(() => { if (active) void emit("popup://ready"); });
    return () => {
      active = false;
      translateGenerationRef.current += 1;
      sessions.current.cancel();
      translations.current.clear();
      for (const s of subs) void s.then((fn) => fn());
    };
  }, [loaded]);

  // ── ESC 取消 / 失焦隐藏 ───────────────────────────────────────────────
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        cancelAndHide();
      }
    }
    window.addEventListener("keydown", onKey);
    const win = getCurrentWindow();
    // 拖拽中窗口持续移动 → 每次 onMoved 续期宽限,把「拖拽引发的失焦」一路罩住。
    const unmoved = win.onMoved(() => bumpDragGuard());
    const unfocus = win.onFocusChanged(({ payload: focused }) => {
      if (focused) return;
      // 处理中(loading)不因焦点丢失而消失;
      // 仍处于拖拽宽限期内(刚拖动过)的失焦也跳过 —— 避免拖动被当成「点窗口外」而隐藏;
      // 已出结果 / 报错后点窗口外才隐藏。
      if (Date.now() < dragGuardUntilRef.current) return;
      // 任意服务仍在请求时保留窗口;所有服务都结束后点窗口外才隐藏。
      const current = viewRef.current;
      if (current.kind === "loading" || (current.kind === "translations"
        && current.rows.some((row) => row.status === "pending"))) return;
      cancelAndHide();
    });
    return () => {
      window.removeEventListener("keydown", onKey);
      void unmoved.then((fn) => fn());
      void unfocus.then((fn) => fn());
    };
  }, [bumpDragGuard, cancelAndHide]);

  // ── 内容尺寸变化 → 调整窗口高度(spec 09 §3.1)──────────────────────────
  // 测量卡片实际高度,请求后端把窗口调到该高度(夹紧 96~480)。resize_popup 命令
  // 调整后会把窗口夹回显示器内(避免长译文底部溢出),并在 macOS 上刷新窗口阴影。
  const resizeToContent = useCallback(() => {
    const el = cardRef.current;
    if (!el) return;
    const h = Math.min(480, Math.max(96, Math.ceil(el.getBoundingClientRect().height)));
    void screenshotApi.resizePopup(h);
  }, []);

  // 用 ResizeObserver 跟随卡片实际高度,覆盖原文 textarea 增/减行等连续高度变化。
  useLayoutEffect(() => {
    const el = cardRef.current;
    if (!el) return;
    let raf = 0;
    const apply = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(resizeToContent);
    };
    apply();
    const ro = new ResizeObserver(apply);
    ro.observe(el);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, [resizeToContent]);

  // 每次结果/视图变化都强制重测一次:再次划词「回显相同内容」时,后端会把窗口重置
  // 回初始高度,而 DOM 高度不变 → ResizeObserver 不触发,这里兜底补一次 resize。
  useLayoutEffect(() => {
    resizeToContent();
  }, [view, resizeToContent]);

  // 原文 textarea 自适应高度(上限 96px,超出内部滚动)。
  useLayoutEffect(() => {
    const ta = originalRef.current;
    if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = `${Math.min(96, ta.scrollHeight)}px`;
  }, [draft, view]);

  const onCopy = useCallback(async (id: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedId(id);
      // 仅清除自己的反馈,避免清掉期间发生的另一次复制。
      window.setTimeout(() => setCopiedId((cur) => (cur === id ? null : cur)), 1500);
    } catch {
      /* clipboard 不可用时静默 */
    }
  }, []);

  const saveServiceSettings = async (engineId: string, values: PopupServiceUiConfig) => {
    const ticket = ++nextPreferenceTicket.current;
    preferenceTickets.current[engineId] = ticket;
    preferenceOverrides.current[engineId] = { ...serviceSettings()[engineId], ...values };
    setPreferenceError(null);
    const saved = await patch({ ui: { popup: { services: { [engineId]: preferenceOverrides.current[engineId] } } } });
    if (preferenceTickets.current[engineId] !== ticket) return;
    delete preferenceTickets.current[engineId];
    delete preferenceOverrides.current[engineId];
    if (!saved) {
      const persisted = useConfigStore.getState().config.ui.popup.services?.[engineId];
      translations.current.configure(engineId, {
        rememberCollapse: persisted?.rememberCollapse ?? false,
        skipWhenCollapsed: persisted?.skipWhenCollapsed ?? false,
      });
      setPreferenceError({ engineId, message: t("popup.preferenceSaveFailed") });
    }
  };
  const toggleService = (row: PopupTranslationRow) => {
    translations.current.configure(row.engineId, { collapsed: !row.collapsed });
    if (row.rememberCollapse) void saveServiceSettings(row.engineId, { collapsed: !row.collapsed });
  };

  // 统一的(重)翻译:重译按钮、原文改写回车、语言下拉切换/互换都走这里。
  // `src` 传 null = 自动检测;`tgt` 缺省用当前配置的目标语言。用「翻译中」布局(原文 +
  // 引擎行转圈)而非整块骨架,切换语言时原文与语言条不闪。
  const runTranslate = useCallback(
    async (original: string, src: string | null = null, tgt: string = targetLang) => {
      const requestGeneration = ++translateGenerationRef.current;
      translations.current.clear();
      const ids = useConfigStore.getState().config.translator.engines
        .filter((engine) => engine.enabled).map((engine) => engine.id);
      setView({
        kind: "translations",
        sessionId: 0,
        original,
        sourceLang: src,
        targetLang: tgt,
        primaryEngineId: defaultEngineId,
        rows: popupTranslationRows(ids, serviceSettings()),
      });
      const ticket = sessions.current.begin();
      try {
        const sessionId = await translateApi.beginPopupSession();
        if (!sessions.current.commit(ticket, sessionId) || requestGeneration !== translateGenerationRef.current) return;
        translations.current.start({ sessionId, original, sourceLang: src, targetLang: tgt }, ids, defaultEngineId,
          undefined, serviceSettings());
      } catch (err) {
        sessions.current.fail(ticket);
        if (isLenzError(err) && err.code === "Cancelled") return;
        if (requestGeneration !== translateGenerationRef.current) return;
        setView({
          kind: "error",
          sessionId: 0,
          stage: "translate",
          code: isLenzError(err) && err.code === "Timeout" ? "timeout" : "failed",
          message: errorMessage(err),
          original,
        });
      }
    },
    [targetLang, defaultEngineId],
  );

  const engineName = (id: string) => engines.find((e) => e.id === id)?.name ?? id;

  const panel = view.kind === "translations" ? view : null;
  const ocrServiceErrorKey = view.kind === "error" && view.stage === "ocr" ? tencentOcrErrorKey(view.message) : null;
  const successes = panel?.rows.filter((row) => row.status === "success");
  const succ = successes?.find((row) => row.engineId === panel?.primaryEngineId) ?? successes?.[0];
  const rowError = (error: PopupTranslationError) => error.message.includes("apple_languages_required")
    ? t("engines.appleLanguagesRequired")
    : error.code.toLowerCase() === "timeout" ? t("popup.error.timeout")
      : error.message === "engine_not_found" ? t("popup.error.engineUnavailable")
        : t("popup.error.translateFailed", { msg: error.message });

  // 语言下拉选项:常用语言 ∪ 当前源/目标(保证即便检测到冷门语言,下拉里也能选中它)。
  const langCodes = useMemo(() => {
    const set = new Set<string>(TRANSLATE_LANGUAGES);
    if (panel?.sourceLang) set.add(panel.sourceLang);
    if (panel?.targetLang) set.add(panel.targetLang);
    return [...set];
  }, [panel?.sourceLang, panel?.targetLang]);
  const targetOptions: SelectOption[] = useMemo(
    () => langCodes.map((c) => ({ value: c, label: langName(c, i18n.language) || c })),
    [langCodes, i18n.language],
  );
  // 源语言额外多一个「自动检测」项置顶。
  const sourceOptions: SelectOption[] = useMemo(
    () => [{ value: "auto", label: t("popup.auto") }, ...targetOptions],
    [targetOptions, t],
  );

  // 语言条当前值 + 切换处理(要翻译的原文取当前草稿,回落到结果原文)。
  const curOriginal = () => (panel ? draft.trim() || panel.original : "");
  const onChangeSource = (v: string) => {
    if (!panel) return;
    void runTranslate(curOriginal(), v === "auto" ? null : v, panel.targetLang);
  };
  const onChangeTarget = (v: string) => {
    if (!panel) return;
    void patch({ translator: { targetLang: v } }); // 记住偏好,后续翻译沿用
    void runTranslate(curOriginal(), panel.sourceLang, v);
  };
  // 互换源/目标:源已知才有意义;源为「自动检测」(未知)时保持不变(安全 no-op)。
  const onSwapLang = () => {
    if (!panel?.sourceLang || !succ?.translation.trim()) return;
    const newTarget = panel.sourceLang;
    const newSource = panel.targetLang;
    setDraft(succ.translation);
    void patch({ translator: { targetLang: newTarget } });
    void runTranslate(succ.translation, newSource, newTarget);
  };

  return (
    <div className="relative select-none">
      {/* 右缘宽度调节手柄:拖拽改变窗口宽度并记忆(见 onGrip* / resize_popup_width) */}
      <div
        onPointerDown={onGripDown}
        onPointerMove={onGripMove}
        onPointerUp={onGripUp}
        role="separator"
        aria-orientation="vertical"
        aria-label={t("popup.resizeWidth")}
        title={t("popup.resizeWidth")}
        className="absolute right-0 top-0 z-10 h-full w-1.5 cursor-ew-resize touch-none"
      />
      {/* max-h 与下方 resizeToContent 的 480 夹紧值一致:内容超高时卡片本身封顶,
          由译文区(flex-1 min-h-0)内部滚动吸收溢出,底部 footer 始终固定可见。 */}
      <div
        ref={cardRef}
        className="ln-popup flex max-h-[480px] flex-col overflow-hidden rounded-[11px]"
      >
        <Header onClose={cancelAndHide} onDragStart={bumpDragGuard} />

        {view.kind === "loading" && (
          <div className="flex flex-col gap-3 p-3.5">
            <div className="flex items-center gap-2 text-[12px] text-sub">
              <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
              {t(`popup.stage.${view.stage}`, t("popup.stage.capture"))}
            </div>
            <div className="h-3 w-2/3 animate-pulse rounded bg-field" />
            <div className="h-3 w-full animate-pulse rounded bg-field" />
            <div className="h-3 w-5/6 animate-pulse rounded bg-field" />
          </div>
        )}

        {panel && (
          <>
            {/* 原文块(可编辑;回车重译,Shift+回车换行) */}
            {panel.original.trim() && (
              <div className="shrink-0 border-b border-line bg-field/60 px-3 py-2.5">
                <textarea
                  ref={originalRef}
                  value={draft}
                  spellCheck={false}
                  rows={1}
                  onChange={(e) => setDraft(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      const text = draft.trim();
                      if (text) void runTranslate(text, panel.sourceLang, panel.targetLang);
                    }
                  }}
                  className="block max-h-24 w-full resize-none select-text overflow-y-auto whitespace-pre-wrap rounded-[7px] border border-line bg-card px-2.5 py-2 text-[13px] leading-snug text-fg outline-none focus:border-accent"
                />
                <div className="mt-2 flex items-center gap-2.5">
                  <button
                    type="button"
                    onClick={() => void onCopy("original", draft)}
                    className={copiedId === "original" ? "text-accent-ink" : "text-sub hover:text-fg"}
                    aria-label={copiedId === "original" ? t("popup.copied") : t("popup.copy")}
                  >
                    <Icon name={copiedId === "original" ? "check" : "copy"} size={13} />
                  </button>
                  <span className="inline-flex items-center gap-1 rounded-[5px] bg-field px-2 py-0.5 text-[11px] text-sub">
                    {t("popup.detected")}{" "}
                    <span className="font-semibold text-accent-ink">
                      {langName(panel.sourceLang, i18n.language) || t("popup.auto")}
                    </span>
                  </span>
                  <span className="ml-auto font-mono text-[10px] text-sub">
                    {draft.length} {t("popup.chars")}
                  </span>
                </div>
              </div>
            )}

            {/* 语言切换条:源(含自动检测)/ 目标 均为样式化下拉,切换即时重译 */}
            <div className="flex shrink-0 items-center border-b border-line bg-field/30 px-2 py-1.5">
              <Select
                className="flex-1"
                value={panel.sourceLang ?? "auto"}
                onChange={onChangeSource}
                options={sourceOptions}
                ariaLabel={t("popup.sourceLang")}
              />
              <button
                type="button"
                onClick={onSwapLang}
                disabled={!panel.sourceLang || !succ?.translation.trim()}
                aria-label={t("popup.swapLang")}
                title={t("popup.swapLang")}
                className="mx-0.5 grid h-6 w-6 shrink-0 place-items-center rounded-md text-sub hover:bg-field hover:text-fg disabled:pointer-events-none disabled:opacity-40"
              >
                <Icon name="swap" size={15} />
              </button>
              <Select
                className="flex-1"
                value={panel.targetLang}
                onChange={onChangeTarget}
                options={targetOptions}
                ariaLabel={t("popup.targetLang")}
                accent
              />
            </div>

            {/* All enabled services share a scrollable list; each row settles independently. */}
            <div className="min-h-0 flex-1 overflow-y-auto" aria-label={t("popup.results")}>
              {panel.rows.length === 0 && (
                <p className="p-3 text-xs text-sub">{t("popup.error.noEngines")}</p>
              )}
              {panel.rows.map((row) => (
                <PopupServiceRow key={row.engineId} row={row} name={engineName(row.engineId)}
                  primary={row.engineId === panel.primaryEngineId} isDefault={row.engineId === defaultEngineId}
                  copied={copiedId === "translation:" + row.engineId}
                  errorText={row.status === "error" ? rowError(row.error) : ""}
                  saveError={preferenceError?.engineId === row.engineId ? preferenceError.message : undefined}
                  onCopy={(text) => void onCopy("translation:" + row.engineId, text)}
                  onRetry={() => translations.current.retry(row.engineId)} onCollapse={() => toggleService(row)}
                  onRemember={(enabled) => {
                    translations.current.configure(row.engineId, { rememberCollapse: enabled });
                    void saveServiceSettings(row.engineId, { rememberCollapse: enabled, collapsed: enabled && row.collapsed });
                  }}
                  onSkip={(enabled) => {
                    translations.current.configure(row.engineId, { skipWhenCollapsed: enabled });
                    void saveServiceSettings(row.engineId, { skipWhenCollapsed: enabled });
                  }} />
              ))}
            </div>

            {/* 底部 */}
            <div className="flex shrink-0 items-center gap-2 border-t border-line bg-field/40 px-3 py-1.5 text-[10px] text-sub">
              <span>{panel.rows.length === 1 ? t("popup.via", { engine: engineName(panel.rows[0].engineId) }) : t("popup.serviceCount", { count: panel.rows.length })}</span>
              <span className="ml-auto">
                <Kbd keys={["esc"]} />
              </span>
            </div>
          </>
        )}

        {view.kind === "error" && (
          <div className="flex flex-col gap-2.5 p-3.5">
            <div className="flex items-start gap-2 text-[12px] font-medium text-danger">
              <Icon name="info" size={14} className="mt-px shrink-0" />
              <span>
                {view.stage === "selection"
                  ? t(view.code === "selection_permission" ? "popup.error.selectionPermission"
                    : view.code === "selection_modifiers" ? "popup.error.selectionModifiers"
                      : "popup.error.selectionEmpty")
                  : view.code === "permission"
                  ? t("popup.error.permission")
                  : view.code === "package_required"
                    ? t("popup.error.package_required")
                    : view.code === "no_text"
                      ? t("popup.error.noText")
                      : view.code === "timeout"
                        ? t(view.stage === "capture" ? "popup.error.captureTimeout"
                          : view.stage === "ocr" ? "popup.error.ocrTimeout" : "popup.error.timeout")
                        : view.stage === "translate"
                          ? t("popup.error.translateFailed", { msg: view.message })
                          : ocrServiceErrorKey
                            ? t(ocrServiceErrorKey)
                            : t("popup.error.generic")}
              </span>
            </div>

            {/* 翻译失败:展示 OCR / 选中原文,避免空白(spec 06 §4)。 */}
            {view.original?.trim() && (
              <p className="max-h-32 overflow-y-auto whitespace-pre-wrap rounded-[7px] border border-line bg-field px-2.5 py-2 text-[12px] text-sub">
                {view.original}
              </p>
            )}

            <div className="flex gap-2">
              {(view.code === "permission" || view.code === "selection_permission") && (
                <button
                  type="button"
                  onClick={() => void (view.code === "selection_permission"
                    ? selectionApi.openAccessibilitySettings() : screenshotApi.openScreenSettings())}
                  className="rounded-md bg-accent px-2.5 py-1 text-[12px] font-medium text-accent-fg hover:opacity-90"
                >
                  {t("popup.error.openSettings")}
                </button>
              )}
              {view.stage === "translate" && view.original?.trim() && (
                <button
                  type="button"
                  onClick={() => void runTranslate(view.original!)}
                  className="rounded-md border border-line px-2.5 py-1 text-[12px] text-fg hover:bg-field"
                >
                  {t("popup.retry")}
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default PopupWindow;
