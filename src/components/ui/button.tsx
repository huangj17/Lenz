/**
 * 按钮 —— 对齐设计稿(settings.jsx):
 *  - primary:实心 lens,白字,带高光阴影(添加服务等)
 *  - secondary:0.5px 描边,透明底(测试连接、更新日志等)
 *  - warm:实心 warm-700(关于页 GitHub)
 */
import { clsx } from "clsx";
import type { ButtonHTMLAttributes } from "react";

type Variant = "primary" | "secondary" | "warm";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
}

const VARIANTS: Record<Variant, string> = {
  primary:
    "bg-accent text-accent-fg shadow-[0_1px_2px_rgba(45,127,179,.3),inset_0_1px_0_rgba(255,255,255,.2)]",
  secondary: "border-[0.5px] border-line bg-transparent text-fg",
  warm: "bg-warm-700 text-warm-white",
};

export function Button({
  variant = "secondary",
  className,
  ...rest
}: ButtonProps) {
  return (
    <button
      type="button"
      className={clsx(
        "inline-flex cursor-pointer items-center gap-1.5 rounded-md border-0 px-3 py-1.5 text-xs font-medium transition-colors",
        VARIANTS[variant],
        className,
      )}
      {...rest}
    />
  );
}
