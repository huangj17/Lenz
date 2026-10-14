/**
 * 开关 —— 对齐设计稿(settings.jsx 的 Toggle):36×21,开启为实心 lens,白色滑块。
 */
interface ToggleProps {
  checked: boolean;
  disabled?: boolean;
  onCheckedChange?: (value: boolean) => void;
  "aria-label"?: string;
}

export function Toggle({ checked, onCheckedChange, ...rest }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onCheckedChange?.(!checked)}
      className="relative h-[21px] w-9 shrink-0 cursor-pointer disabled:cursor-not-allowed disabled:opacity-40 rounded-full border-0 transition-colors"
      style={{
        background: checked
          ? "var(--lens)"
          : "color-mix(in srgb, var(--fg) 18%, transparent)",
        boxShadow: "inset 0 0.5px 2px rgba(0,0,0,.1)",
      }}
      {...rest}
    >
      <span
        className="absolute top-[2px] size-[17px] rounded-full bg-white transition-[left]"
        style={{
          left: checked ? 17 : 2,
          boxShadow: "0 1px 3px rgba(0,0,0,.25)",
        }}
      />
    </button>
  );
}
