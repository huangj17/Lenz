/**
 * 设置页布局原子 —— 对齐设计稿(settings.jsx 的 Section / Row / Field / Card)。
 */
import type { ReactNode } from "react";
import { clsx } from "clsx";
import { Icon } from "@/components/Icon";

/** 卡片容器:0.5px 描边 + 9px 圆角 + card 底 */
export function Card({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={clsx(
        "overflow-hidden rounded-[9px] border-[0.5px] border-line bg-card",
        className,
      )}
    >
      {children}
    </div>
  );
}

/** 分组:mono 大写小标题 + 可选描述 */
export function Section({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="mb-[22px]">
      <div className="mb-2 font-mono text-[11px] font-medium uppercase tracking-[0.1em] text-sub">
        {title}
      </div>
      {description && (
        <div className="mb-2.5 text-xs text-sub">{description}</div>
      )}
      {children}
    </div>
  );
}

/** 列表行:左 label(+hint)/ 右控件,底边分隔线 */
export function Row({
  label,
  hint,
  children,
  last,
}: {
  label: string;
  hint?: string;
  children?: ReactNode;
  last?: boolean;
}) {
  return (
    <div
      className={clsx(
        "flex items-center justify-between px-[14px] py-2.5",
        !last && "border-b-[0.5px] border-line",
      )}
    >
      <div>
        <div className="text-[13px]">{label}</div>
        {hint && <div className="mt-0.5 text-[11px] text-sub">{hint}</div>}
      </div>
      <div>{children}</div>
    </div>
  );
}

/** 字段:mono 大写标签 + 取值框(可选 mono / 下拉箭头) */
export function Field({
  label,
  value,
  mono,
  dropdown,
}: {
  label: string;
  value: ReactNode;
  mono?: boolean;
  dropdown?: boolean;
}) {
  return (
    <div className="mb-2.5">
      <div className="mb-1 font-mono text-[11px] uppercase tracking-[0.06em] text-sub">
        {label}
      </div>
      <div className="flex items-center rounded-md border-[0.5px] border-line bg-field px-2.5 py-1.5 text-[12.5px]">
        <span className={clsx("flex-1", mono && "font-mono")}>{value}</span>
        {dropdown && <Icon name="chevronDown" size={13} className="text-sub" />}
      </div>
    </div>
  );
}
