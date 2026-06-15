/**
 * Lenz 应用 logo —— 移植自设计交接包(docs/design/project/src/icons.jsx 的 LensMark)。
 * 使用设计提供的 logo.png(已复制到 src/assets/lenz-logo.png)。
 */
import logoUrl from "@/assets/lenz-logo.png";

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
