/**
 * 分段控件 —— 对齐设计稿(settings.jsx)。
 *  - size="md":服务页主分段(带图标,padding 大)
 *  - size="sm":行内分段(主题、OCR 精度)
 * 选中态:浅色 #fff / 深色 #2a2721 实心 + 阴影,文字 accent-ink(md)。
 */
import { clsx } from "clsx";
import { Icon, type IconName } from "@/components/Icon";

export interface SegOption<T extends string> {
  value: T;
  label: string;
  icon?: IconName;
}

interface SegmentedProps<T extends string> {
  options: SegOption<T>[];
  value: T;
  onChange: (value: T) => void;
  size?: "sm" | "md";
  "aria-label"?: string;
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  size = "md",
  ...rest
}: SegmentedProps<T>) {
  const md = size === "md";
  return (
    <div
      role="tablist"
      className={clsx(
        "inline-flex bg-field",
        md
          ? "gap-[3px] rounded-[9px] border-[0.5px] border-line p-[3px]"
          : "gap-1.5 rounded-[7px] p-0.5",
      )}
      {...rest}
    >
      {options.map((o) => {
        const active = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(o.value)}
            className={clsx(
              "flex cursor-pointer items-center border-0 bg-transparent transition-colors",
              md
                ? "gap-[7px] rounded-md px-4 py-1.5 text-[12.5px]"
                : "rounded-[5px] px-2.5 py-[3px] text-xs",
              active
                ? clsx(
                    "bg-white dark:bg-[#2a2721]",
                    md
                      ? "font-semibold text-accent-ink shadow-[0_1px_3px_rgba(0,0,0,.1)]"
                      : "font-medium text-fg shadow-[0_1px_2px_rgba(0,0,0,.08)]",
                  )
                : clsx(md ? "font-medium text-sub" : "font-normal text-fg"),
            )}
          >
            {o.icon && <Icon name={o.icon} size={14} />}
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
