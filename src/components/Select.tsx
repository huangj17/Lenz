/**
 * 轻量样式化下拉选择器。用自绘菜单取代原生 `<select>`(原生选项菜单无法跨平台主题化),
 * 视觉贴合弹框:透明触发按钮 + `.ln-menu` 不透明浮层 + 选中项高亮打勾。
 *
 * 菜单是**卡片内**的绝对定位浮层,不撑开卡片/窗口:展开时按触发按钮上下的可用空间选择
 * 向下或向上弹出,并把菜单高度夹到该侧可用空间(超出内部滚动),从而始终落在窗口内、
 * 不被 `overflow-hidden` 裁剪,也不会把弹框拉高留白。
 */
import { useEffect, useRef, useState } from "react";
import { Icon } from "@/components/Icon";

export interface SelectOption {
  value: string;
  label: string;
}

export function Select({
  value,
  onChange,
  options,
  ariaLabel,
  accent,
  className,
  maxMenuHeight = 240,
}: {
  value: string;
  onChange: (v: string) => void;
  options: SelectOption[];
  ariaLabel: string;
  /** 用强调色显示触发文字(如目标语言)。 */
  accent?: boolean;
  className?: string;
  /** 菜单最大高度(与可用空间取小,超出内部滚动)。 */
  maxMenuHeight?: number;
}) {
  const [open, setOpen] = useState(false);
  // 弹出方向 + 实际最大高度(据触发按钮上下可用空间在打开时算定)。
  const [place, setPlace] = useState<{ dir: "down" | "up"; maxH: number }>({
    dir: "down",
    maxH: maxMenuHeight,
  });
  const rootRef = useRef<HTMLDivElement>(null);
  const btnRef = useRef<HTMLButtonElement>(null);

  const openMenu = () => {
    const r = btnRef.current?.getBoundingClientRect();
    if (r) {
      const below = window.innerHeight - r.bottom - 8;
      const above = r.top - 8;
      const dir = below >= above ? "down" : "up";
      const maxH = Math.min(maxMenuHeight, Math.max(0, dir === "down" ? below : above));
      setPlace({ dir, maxH });
    }
    setOpen(true);
  };

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    };
    // 捕获阶段拦 ESC:先于 window 上「ESC 关弹框」的冒泡监听执行,只关菜单不关弹框。
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey, true);
    };
  }, [open]);

  const current = options.find((o) => o.value === value);

  return (
    <div
      ref={rootRef}
      className={`relative ${accent ? "text-accent-ink" : "text-fg"} ${className ?? ""}`}
    >
      <button
        ref={btnRef}
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => (open ? setOpen(false) : openMenu())}
        className="flex w-full items-center justify-center gap-1 rounded-md py-1 pl-3 pr-2 text-[12.5px] font-semibold text-inherit hover:bg-field/60"
      >
        <span className="truncate">{current?.label ?? value}</span>
        <span
          className={`shrink-0 text-[8px] leading-none opacity-55 transition-transform ${
            open ? "rotate-180" : ""
          }`}
          aria-hidden
        >
          ▼
        </span>
      </button>
      {open && (
        <div
          role="listbox"
          style={{ maxHeight: place.maxH }}
          className={`ln-menu absolute left-1/2 z-50 w-max min-w-full max-w-[220px] -translate-x-1/2 overflow-auto rounded-lg p-1 ${
            place.dir === "down" ? "top-full mt-1" : "bottom-full mb-1"
          }`}
        >
          {options.map((o) => {
            const active = o.value === value;
            return (
              <button
                key={o.value}
                type="button"
                role="option"
                aria-selected={active}
                onClick={() => {
                  onChange(o.value);
                  setOpen(false);
                }}
                className={`flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-[12.5px] ${
                  active ? "bg-field font-semibold text-accent-ink" : "text-fg hover:bg-field"
                }`}
              >
                <span className="flex-1 truncate">{o.label}</span>
                {active && <Icon name="check" size={12} className="shrink-0 text-accent-ink" />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

export default Select;
