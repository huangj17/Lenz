import type { ReactNode } from "react";

/** 设置页区块标题 + 内容容器 */
export function Section({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children?: ReactNode;
}) {
  return (
    <section className="mb-6">
      <h2 className="mb-1 text-base font-semibold text-fg">{title}</h2>
      {description && (
        <p className="mb-3 text-xs text-fg-muted">{description}</p>
      )}
      <div>{children}</div>
    </section>
  );
}

/** 标签 + 控件的一行 */
export function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between border-b border-border py-2.5 last:border-b-0">
      <div className="min-w-0 pr-4">
        <div className="text-[13px] text-fg">{label}</div>
        {hint && <div className="mt-0.5 text-[11px] text-fg-muted">{hint}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

/** Tab 内的"待实现"占位 */
export function Placeholder({ label }: { label: string }) {
  return (
    <div className="flex h-40 items-center justify-center rounded-[var(--radius)] border border-dashed border-border text-sm text-fg-muted">
      {label}
    </div>
  );
}
