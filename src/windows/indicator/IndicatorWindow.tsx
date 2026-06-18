/**
 * 划词指示图标窗口(spec 05 §6 / 设计调整 2026-06-18)。
 *
 * 监听到划词/双击选中后,Rust 侧把本窗口定位到光标处并 show(自动隐藏由 Rust 的
 * `IndicatorSession` 计时控制);前端只负责渲染一个 Lens 小图标,并在 **hover 或
 * 点击** 时 invoke `trigger_selection_translate` —— 由后端合成 Cmd+C → 翻译 → 弹框。
 *
 * 注:本窗口 `focus:false` + `acceptFirstMouse:true`,无需成为 key 窗口即可响应点击;
 * hover(pointerenter)在非活动窗口上是否触发取决于系统,故 hover 与 click 双保险。
 */
import { useCallback, useRef } from "react";
import { LensMark } from "@/components/LensMark";
import { selectionApi } from "@/lib/ipc";

export function IndicatorWindow() {
  // 去抖:hover 与 click 可能同时触发;一次显示周期内只发一次(1s 内不重复)。
  const lastFiredRef = useRef(0);

  const trigger = useCallback(() => {
    const now = Date.now();
    if (now - lastFiredRef.current < 1000) return;
    lastFiredRef.current = now;
    void selectionApi.trigger();
  }, []);

  return (
    <button
      type="button"
      onPointerEnter={trigger}
      onClick={trigger}
      aria-label="划词翻译"
      className="grid h-screen w-screen cursor-pointer place-items-center border-0 bg-transparent p-0"
    >
      <span className="grid h-[26px] w-[26px] place-items-center rounded-full bg-card shadow-md ring-[0.5px] ring-line transition-transform duration-100 hover:scale-110">
        <LensMark size={16} />
      </span>
    </button>
  );
}

export default IndicatorWindow;
