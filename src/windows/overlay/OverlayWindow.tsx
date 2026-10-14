import { useEffect, useState } from "react";
import { overlayApi } from "@/lib/ipc";
import type { OverlayPayload } from "@/lib/types";
import { fitOverlayFont } from "@/lib/overlayLayout";

export function OverlayWindow() {
  const [payload, setPayload] = useState<OverlayPayload | null>(null);
  useEffect(() => {
    let active = true;
    void overlayApi.payload().then((value) => { if (active) setPayload(value); }).catch(() => {});
    return () => { active = false; };
  }, []);
  useEffect(() => {
    if (!payload) return;
    const frame = requestAnimationFrame(() => { void overlayApi.ready(payload.generation).catch(() => {}); });
    return () => cancelAnimationFrame(frame);
  }, [payload]);
  const context = document.createElement("canvas").getContext("2d");
  return <div className="relative h-screen w-screen overflow-hidden">
    {payload?.blocks.map((block, index) => {
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
  </div>;
}
export default OverlayWindow;
