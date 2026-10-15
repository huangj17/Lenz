export function fitOverlayFont(text: string, width: number, height: number, measure: (text: string, size: number) => number): number {
  let size = Math.max(8, Math.min(32, height * 0.8));
  while (size > 8 && measure(text, size) > Math.max(0, width - 4)) size -= 1;
  return Math.max(8, size);
}

interface OverlayRect { x: number; y: number; w: number; h: number }

/** 全局逻辑坐标转为蒙层坐标;工具条优先放在截图外,并始终留在屏幕内。 */
export function overlayLayout(region: OverlayRect, viewport: OverlayRect) {
  const left = region.x - viewport.x;
  const top = region.y - viewport.y;
  const toolbarWidth = Math.min(300, Math.max(0, viewport.w - 24));
  const toolbarTop = top >= 68 ? top - 56
    : top + region.h + 56 <= viewport.h ? top + region.h + 12 : 12;
  return {
    left,
    top,
    toolbar: {
      left: Math.max(12, Math.min(left + region.w - toolbarWidth, viewport.w - toolbarWidth - 12)),
      top: Math.max(12, Math.min(toolbarTop, viewport.h - 56)),
      width: toolbarWidth,
    },
  };
}
