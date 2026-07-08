/**
 * 翻译结果弹框窗口(spec 06 §3 / spec 09 §3)。视觉对齐设计交接包 popup-f1/f2:
 * 暖色毛玻璃卡片 + Lens 头部 + 原文块(识别语言 chip + 字数)+ 语言切换条 +
 * 引擎结果行(徽标 + 名称 + 默认 + 用时)+ 底部 esc 提示。
 *
 * 订阅 `screenshot://progress|result|error`,在 loading / success / error 三态切换。
 * 头部为窗口拖拽区(`data-tauri-drag-region`)。内容渲染后用 `setSize` 动态调高;
 * ESC / 失焦 / 关闭隐藏;复制译文;翻译失败展示原文 + 重试;权限缺失引导打开系统设置。
 */
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { Icon, Kbd } from "@/components/Icon";
import { LensMark } from "@/components/LensMark";
import { emit, Events, listenEvent, screenshotApi, translateApi } from "@/lib/ipc";
import { useConfigStore } from "@/stores/config";
import type { ScreenshotErrorPayload } from "@/lib/types";

/** 弹框可拖拽宽度上/下限(逻辑像素),与后端 `POPUP_MIN/MAX_WIDTH` 一致。 */
const POPUP_MIN_WIDTH = 300;
const POPUP_MAX_WIDTH = 720;

type View =
  | { kind: "loading"; stage: "capture" | "ocr" | "translate" }
  // 翻译中:原文已知、译文未出。渲染与 success 相同的布局,仅引擎行右侧转圈、译文区占位。
  | {
      kind: "translating";
      original: string;
      sourceLang: string | null;
      targetLang: string;
      engineId: string;
    }
  | {
      kind: "success";
      original: string;
      translation: string;
      sourceLang: string | null;
      targetLang: string;
      engineId: string;
      costMs: number;
    }
  | ({ kind: "error" } & ScreenshotErrorPayload);

/** BCP-47 → 当前 UI 语言下的语言名;失败回退原 code。 */
function langName(code: string | null | undefined, uiLocale: string): string {
  if (!code) return "自动检测";
  try {
    return new Intl.DisplayNames([uiLocale], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}

/** 引擎名取前两个字符作徽标。 */
function badgeOf(name: string): string {
  const s = name.trim().replace(/\s+/g, "");
  return (s.slice(0, 2) || "EN").toUpperCase();
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
      className="flex items-center gap-2 border-b border-line px-3 py-2.5"
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
  const patch = useConfigStore((s) => s.patch);

  const [view, setView] = useState<View>({ kind: "loading", stage: "capture" });
  // 哪个复制按钮刚被点过("original" / "translation");原文与译文各自独立反馈。
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  // 原文可编辑草稿:同步自最近一次结果的原文;用户可改写后回车重译。
  const [draft, setDraft] = useState("");
  const originalRef = useRef<HTMLTextAreaElement>(null);
  // 供失焦回调读取最新状态(回调在挂载时绑定,闭包内 view 会过期)。
  const viewRef = useRef(view);
  viewRef.current = view;
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
    view.kind === "success" || view.kind === "translating" ? view.original : null;
  useEffect(() => {
    if (incomingOriginal !== null) setDraft(incomingOriginal);
  }, [incomingOriginal]);

  // ── 订阅 F2 / F1 事件 ─────────────────────────────────────────────────
  useEffect(() => {
    const subs = [
      listenEvent(Events.ScreenshotProgress, (e) => {
        setCopiedId(null);
        setView({ kind: "loading", stage: e.payload.stage });
      }),
      listenEvent(Events.ScreenshotTranslating, (e) => {
        const p = e.payload;
        setCopiedId(null);
        setView({
          kind: "translating",
          original: p.original,
          sourceLang: p.sourceLang,
          targetLang: p.targetLang,
          engineId: p.engineId,
        });
      }),
      listenEvent(Events.ScreenshotResult, (e) => {
        const r = e.payload;
        setView({
          kind: "success",
          original: r.original,
          translation: r.translation,
          sourceLang: r.sourceLang,
          targetLang: r.targetLang,
          engineId: r.engineId,
          costMs: r.cost.translate,
        });
      }),
      listenEvent(Events.ScreenshotError, (e) => setView({ kind: "error", ...e.payload })),
    ];
    // 三个监听全部挂上后,通知后端 popup 可接收事件——关闭冷启动首帧竞态:
    // 首次划词/截图时后端会先等这个 `popup://ready` 再投递 progress/result(方案 A)。
    void Promise.all(subs).then(() => void emit("popup://ready"));
    return () => {
      for (const s of subs) void s.then((fn) => fn());
    };
  }, []);

  // ── ESC 取消 / 失焦隐藏 ───────────────────────────────────────────────
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        void screenshotApi.cancel();
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
      // 处理中(loading / translating)不因失焦而消失;仅已出结果 / 报错后点窗口外才隐藏。
      const k = viewRef.current.kind;
      if (k !== "loading" && k !== "translating") void hidePopup();
    });
    return () => {
      window.removeEventListener("keydown", onKey);
      void unmoved.then((fn) => fn());
      void unfocus.then((fn) => fn());
    };
  }, [bumpDragGuard]);

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

  const onRetryTranslate = useCallback(
    async (original: string) => {
      // 重译也用「翻译中」布局(原文 + 引擎行转圈),而非整块骨架。
      setView({
        kind: "translating",
        original,
        sourceLang: null,
        targetLang,
        engineId: defaultEngineId,
      });
      try {
        const resp = await translateApi.text({ texts: [original], targetLang });
        setView({
          kind: "success",
          original,
          translation: resp.translations[0] ?? "",
          sourceLang: resp.detectedSourceLang ?? null,
          targetLang,
          engineId: resp.engineId,
          costMs: resp.costMs,
        });
      } catch (err) {
        setView({
          kind: "error",
          stage: "translate",
          code: "failed",
          message: err instanceof Error ? err.message : String(err),
          original,
        });
      }
    },
    [targetLang, defaultEngineId],
  );

  const engineName = (id: string) => engines.find((e) => e.id === id)?.name ?? id;

  // success 与 translating 共用同一套「原文 + 语言条 + 引擎行」布局:
  // `panel` 提供公共字段(原文/语言/引擎);`succ` 非空表示译文已出(否则引擎行转圈)。
  const panel =
    view.kind === "success" || view.kind === "translating" ? view : null;
  const succ = view.kind === "success" ? view : null;

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
        <Header onClose={() => void hidePopup()} onDragStart={bumpDragGuard} />

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
              <div className="border-b border-line bg-field/60 px-3 py-2.5">
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
                      if (text) void onRetryTranslate(text);
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
                      {langName(panel.sourceLang, i18n.language)}
                    </span>
                  </span>
                  <span className="ml-auto font-mono text-[10px] text-sub">
                    {draft.length} {t("popup.chars")}
                  </span>
                </div>
              </div>
            )}

            {/* 语言切换条 */}
            <div className="flex items-center border-b border-line bg-field/30 px-4 py-2">
              <div className="flex flex-1 justify-center text-[12.5px] font-semibold text-fg">
                {langName(panel.sourceLang, i18n.language)}
              </div>
              <Icon name="swap" size={15} className="text-sub" />
              <div className="flex flex-1 justify-center text-[12.5px] font-semibold text-accent-ink">
                {langName(panel.targetLang, i18n.language)}
              </div>
            </div>

            {/* 引擎结果行(卡片封顶时,本块作为可伸缩滚动区吸收溢出) */}
            <div className="flex min-h-0 flex-1 flex-col border-l-2 border-accent px-3 py-2.5">
              <div className="flex items-center gap-2">
                <span className="ln-engine-badge grid h-[18px] w-[18px] place-items-center rounded font-mono text-[9px] font-bold text-white">
                  {badgeOf(engineName(panel.engineId))}
                </span>
                <span className="text-[12px] font-semibold text-fg">
                  {engineName(panel.engineId)}
                </span>
                {succ ? (
                  <span className="ml-auto flex items-center gap-1.5 font-mono text-[10px] text-sub">
                    <Icon name="check" size={10} className="text-[color:var(--sage)]" />
                    {succ.costMs}ms
                  </span>
                ) : (
                  // 翻译中:引擎行右侧转圈
                  <span className="ml-auto flex items-center gap-1.5 text-[10px] text-sub">
                    <span
                      className="h-3 w-3 animate-spin rounded-full border-[1.5px] border-line border-t-accent"
                      aria-label={t("popup.translating")}
                    />
                  </span>
                )}
              </div>
              {succ ? (
                <>
                  <p className="mt-2 min-h-0 flex-1 select-text overflow-y-auto whitespace-pre-wrap pl-[26px] text-[13.5px] leading-relaxed text-fg">
                    {succ.translation}
                  </p>
                  <div className="mt-2.5 flex items-center gap-3 pl-[26px]">
                    <button
                      type="button"
                      onClick={() => void onCopy("translation", succ.translation)}
                      className="inline-flex items-center gap-1.5 text-[11px] text-sub hover:text-fg"
                    >
                      <Icon name={copiedId === "translation" ? "check" : "copy"} size={13} />
                      {copiedId === "translation" ? t("popup.copied") : t("popup.copy")}
                    </button>
                  </div>
                </>
              ) : (
                // 翻译中:译文区占位(骨架行),保持面板高度不塌陷
                <div className="mt-2 flex flex-col gap-2 pl-[26px]">
                  <div className="h-3 w-5/6 animate-pulse rounded bg-field" />
                  <div className="h-3 w-2/3 animate-pulse rounded bg-field" />
                </div>
              )}
            </div>

            {/* 底部 */}
            <div className="flex items-center gap-2 border-t border-line bg-field/40 px-3 py-1.5 text-[10px] text-sub">
              <span>{t("popup.via", { engine: engineName(panel.engineId) })}</span>
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
                {view.code === "permission"
                  ? t("popup.error.permission")
                  : view.code === "no_text"
                    ? t("popup.error.noText")
                    : view.stage === "timeout"
                      ? t("popup.error.timeout")
                      : view.stage === "translate"
                        ? t("popup.error.translateFailed", { msg: view.message })
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
              {view.code === "permission" && (
                <button
                  type="button"
                  onClick={() => void screenshotApi.openScreenSettings()}
                  className="rounded-md bg-accent px-2.5 py-1 text-[12px] font-medium text-accent-fg hover:opacity-90"
                >
                  {t("popup.error.openSettings")}
                </button>
              )}
              {view.stage === "translate" && view.original?.trim() && (
                <button
                  type="button"
                  onClick={() => void onRetryTranslate(view.original!)}
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
