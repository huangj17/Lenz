/**
 * 截图选区蒙层窗口(spec 02 §3 / spec 09 §4)。
 *
 * 全屏 canvas:半透遮罩 + 选区镂空 + 边框 + 角 handle + 尺寸提示。
 * 鼠标 down/move/up 绘制;ESC 取消。框选完成时把 CSS 坐标 + 窗口逻辑原点换算为
 * 虚拟屏逻辑坐标,emit `capture://region-selected`;过小或 ESC 则 emit
 * `capture://region-cancelled`,均由 Rust 侧销毁本窗口。
 */
import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { Kbd } from "@/components/Icon";
import { LensMark } from "@/components/LensMark";
import { emit, Events } from "@/lib/ipc";
import type { TriggerKind } from "@/lib/types";

function readTrigger(): TriggerKind {
  const t = new URLSearchParams(window.location.search).get("trigger");
  return t === "overlay" ? "overlay" : "screenshot";
}

interface DragState {
  dragging: boolean;
  sx: number;
  sy: number;
  cx: number;
  cy: number;
}

export function SelectionWindow() {
  const { t } = useTranslation();
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvasRef.current) return;
    const canvas = canvasRef.current;
    // 非空断言:effect 在挂载后运行,ref 必有值;2d 上下文恒可得。
    // (TS 不跨嵌套闭包保留窄化,故在声明处即固定为非空类型。)
    const ctx = canvas.getContext("2d")!;

    const trigger = readTrigger();
    const drag: DragState = { dragging: false, sx: 0, sy: 0, cx: 0, cy: 0 };
    const dpr = window.devicePixelRatio || 1;

    function draw() {
      const w = window.innerWidth;
      const h = window.innerHeight;
      ctx.clearRect(0, 0, w, h);
      ctx.fillStyle = "rgba(0,0,0,0.35)";
      ctx.fillRect(0, 0, w, h);
      if (!drag.dragging) return;

      const x = Math.min(drag.sx, drag.cx);
      const y = Math.min(drag.sy, drag.cy);
      const rw = Math.abs(drag.cx - drag.sx);
      const rh = Math.abs(drag.cy - drag.sy);

      // 选区镂空。
      ctx.clearRect(x, y, rw, rh);
      // 十字辅助线(沿选区右/下边缘,faint lens;对齐设计 capture-f2)。
      ctx.strokeStyle = "rgba(78,163,217,0.3)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(x + rw + 0.5, 0);
      ctx.lineTo(x + rw + 0.5, h);
      ctx.moveTo(0, y + rh + 0.5);
      ctx.lineTo(w, y + rh + 0.5);
      ctx.stroke();
      // 边框(天空蓝,与品牌色一致)。
      ctx.strokeStyle = "#4ea3d9";
      ctx.lineWidth = 1.5;
      ctx.strokeRect(x + 0.5, y + 0.5, rw, rh);
      // 四角 handle。
      ctx.fillStyle = "#ffffff";
      for (const [hx, hy] of [
        [x, y],
        [x + rw, y],
        [x, y + rh],
        [x + rw, y + rh],
      ]) {
        ctx.fillRect(hx - 3, hy - 3, 6, 6);
      }
      // 尺寸提示。
      const label = `${Math.round(rw)} × ${Math.round(rh)}`;
      ctx.font = '12px -apple-system, "PingFang SC", system-ui, sans-serif';
      const tw = ctx.measureText(label).width;
      const lx = Math.max(2, Math.min(x, w - tw - 14));
      const ly = y > 26 ? y - 8 : y + rh + 20;
      ctx.fillStyle = "rgba(0,0,0,0.72)";
      ctx.fillRect(lx - 6, ly - 14, tw + 12, 20);
      ctx.fillStyle = "#ffffff";
      ctx.fillText(label, lx, ly);
    }

    function resize() {
      canvas.width = Math.round(window.innerWidth * dpr);
      canvas.height = Math.round(window.innerHeight * dpr);
      canvas.style.width = `${window.innerWidth}px`;
      canvas.style.height = `${window.innerHeight}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      draw();
    }

    /** 窗口左上角的虚拟屏逻辑坐标(spec 09 §4.3)。 */
    async function originLogical(): Promise<{ x: number; y: number }> {
      const win = getCurrentWindow();
      const [pos, sf] = await Promise.all([win.outerPosition(), win.scaleFactor()]);
      const lp = pos.toLogical(sf);
      return { x: lp.x, y: lp.y };
    }

    function onDown(e: MouseEvent) {
      drag.dragging = true;
      drag.sx = e.clientX;
      drag.sy = e.clientY;
      drag.cx = e.clientX;
      drag.cy = e.clientY;
      draw();
    }
    function onMove(e: MouseEvent) {
      if (!drag.dragging) return;
      drag.cx = e.clientX;
      drag.cy = e.clientY;
      draw();
    }
    async function onUp() {
      if (!drag.dragging) return;
      drag.dragging = false;
      const x = Math.min(drag.sx, drag.cx);
      const y = Math.min(drag.sy, drag.cy);
      const rw = Math.abs(drag.cx - drag.sx);
      const rh = Math.abs(drag.cy - drag.sy);
      draw();
      // 过小视为点击取消(spec 02 §6)。
      if (rw < 4 || rh < 4) {
        await emit(Events.CaptureRegionCancelled);
        return;
      }
      const o = await originLogical();
      await emit(Events.CaptureRegionSelected, {
        x: Math.round(o.x + x),
        y: Math.round(o.y + y),
        w: Math.round(rw),
        h: Math.round(rh),
        trigger,
      });
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        void emit(Events.CaptureRegionCancelled);
      }
    }

    resize();
    window.addEventListener("resize", resize);
    window.addEventListener("mousedown", onDown);
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("resize", resize);
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  return (
    <>
      <canvas ref={canvasRef} className="block h-screen w-screen cursor-crosshair" />
      {/* 顶部指示条(对齐设计 capture-f2);pointer-events-none 不拦截框选。 */}
      <div className="pointer-events-none fixed left-1/2 top-4 -translate-x-1/2">
        <div
          className="flex items-center gap-2.5 rounded-full px-3.5 py-1.5 text-[12px] text-white"
          style={{ background: "rgba(0,0,0,0.8)", boxShadow: "0 8px 20px rgba(0,0,0,0.3)" }}
        >
          <LensMark size={14} />
          <span>{t("capture.hint")}</span>
          <span className="h-3 w-px bg-white/20" />
          <span className="flex items-center gap-1.5 text-[11px] opacity-75">
            <Kbd keys={["esc"]} /> {t("capture.cancel")}
          </span>
        </div>
      </div>
    </>
  );
}

export default SelectionWindow;
