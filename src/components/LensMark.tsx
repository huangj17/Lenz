/**
 * Lenz 应用 logo —— 矢量重绘版(src/assets/lenz-logo.svg):圆角 L + 文/A 双气泡。
 * 任意尺寸清晰;脚本为 PNG 版(lenz-logo.png)加冰蓝渐变底板,用于 dock/安装包。
 */
import logoUrl from "@/assets/lenz-logo.svg";

export function LensMark({
  size = 28,
  className,
}: {
  size?: number;
  className?: string;
}) {
  return (
    <img
      src={logoUrl}
      alt="Lenz"
      width={size}
      height={size}
      draggable={false}
      className={className}
      style={{
        display: "block",
        width: size,
        height: size,
        objectFit: "contain",
        userSelect: "none",
      }}
    />
  );
}
