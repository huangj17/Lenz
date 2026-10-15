import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon, Kbd } from "@/components/Icon";
import { overlayApi } from "@/lib/ipc";
import type { OverlayPayload } from "@/lib/types";
import { fitOverlayFont, overlayLayout } from "@/lib/overlayLayout";
import { observeOverlayProgress } from "@/lib/overlayProgress";

export function OverlayWindow() {
  const { t } = useTranslation();
  const [payload, setPayload] = useState<OverlayPayload | null>(null);
  const [closeError, setCloseError] = useState(false);
  const closing = useRef(false);
  const context = useMemo(() => document.createElement("canvas").getContext("2d"), []);

  const close = useCallback(() => {
    if (closing.current) return;
    closing.current = true;
    setCloseError(false);
    void overlayApi.close().catch(() => {
      closing.current = false;
      setCloseError(true);
    });
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.isComposing) return;
      event.preventDefault();
      event.stopPropagation();
      close();
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [close]);

  useEffect(() => observeOverlayProgress(
    (handler) => overlayApi.onUpdated((event) => handler(event.payload)),
    overlayApi.payload,
    setPayload,
    close,
  ), [close]);

  const generation = payload?.generation;
  useEffect(() => {
    if (generation === undefined) return;
    // 隐藏的原生 WebView 可能暂停动画帧;DOM 提交后直接通知显示并聚焦。
    void overlayApi.ready(generation).catch(close);
  }, [generation, close]);

  const layout = payload ? overlayLayout(payload.region, payload.viewport) : null;
  const phase = payload?.progress.phase;
  const loading = phase === "ocr" || phase === "translate";
  const error = payload?.progress.error;
  const errorText = error?.code === "Timeout" ? t(error.message.includes("ocr") ? "popup.error.ocrTimeout" : "popup.error.timeout")
    : error?.code === "OcrFailed" && error.message === "no text recognized" ? t("popup.error.noText")
      : error?.code === "OcrIdentityRequired" ? t("popup.error.package_required")
        : t("overlay.failed");
  return (
    <div role="dialog" aria-modal="true" aria-labelledby="overlay-title"
      className="relative h-screen w-screen overflow-hidden bg-black/45 text-fg">
      {payload && layout && <>
        <div aria-busy={loading} className="absolute shadow-2xl ring-1 ring-white/50"
          style={{ left: layout.left, top: layout.top, width: payload.region.w, height: payload.region.h }}>
          <img src={payload.imageDataUrl} alt={t("overlay.capture")} draggable={false}
            className="pointer-events-none absolute inset-0 h-full w-full select-none" />
          {payload.progress.blocks.map((block, index) => {
            const size = fitOverlayFont(block.translation, block.bbox.w, block.bbox.h, (text, font) => {
              if (!context) return text.length * font;
              context.font = font + "px sans-serif";
              return context.measureText(text).width;
            });
            return <div key={index} style={{ position: "absolute", left: block.bbox.x, top: block.bbox.y,
              width: block.bbox.w, height: block.bbox.h, background: "rgb(" + block.bgColor.join(",") + ")", color: block.fgColor,
              fontSize: size, lineHeight: 1, padding: "1px 2px", overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis",
              display: "flex", alignItems: "center", justifyContent: "center" }}>
              <span className="min-w-0 overflow-hidden text-ellipsis">{block.translation}</span>
            </div>;
          })}
        </div>
        <div className="absolute z-10" style={layout.toolbar}>
          <div className="flex h-11 items-center gap-2 rounded-xl border border-line bg-bg px-3 shadow-xl">
            {loading ? <span aria-hidden="true" className="h-[18px] w-[18px] shrink-0 animate-spin rounded-full border-2 border-accent/25 border-t-accent motion-reduce:animate-none" />
              : <Icon name="overlay" size={18} className="shrink-0 text-accent" />}
            <span id="overlay-title" role="status" aria-live="polite" className="min-w-0 flex-1 truncate text-[13px] font-medium">
              {loading ? t(`popup.stage.${phase}`) : t("overlay.title")}
            </span>
            <span className="flex shrink-0 items-center gap-1 text-[11px] text-sub"><Kbd keys={["esc"]} />{t("common.close")}</span>
            <span className="mx-1 h-4 w-px bg-line" aria-hidden="true" />
            <button type="button" autoFocus onClick={close} aria-label={t("overlay.close")}
              title={t("overlay.close")} className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md hover:bg-field focus-visible:outline-2 focus-visible:outline-accent">
              <Icon name="close" size={17} />
            </button>
          </div>
          {phase === "failed" && <p role="alert" className="mt-2 rounded-lg border border-line bg-bg p-3 text-xs text-danger">{errorText}</p>}
          {closeError && <p role="alert" className="mt-2 rounded-lg bg-bg p-2 text-xs text-danger">{t("overlay.closeFailed")}</p>}
        </div>
      </>}
    </div>
  );
}
export default OverlayWindow;
