import * as React from "react";
import { cn } from "@/lib/utils";

/**
 * 轻量原生 <select> 包装(占位用)。
 * M7 如需更丰富的下拉交互可替换为 @radix-ui/react-select。
 */
export type SelectProps = React.SelectHTMLAttributes<HTMLSelectElement>;

const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ className, children, ...props }, ref) => (
    <select
      ref={ref}
      className={cn(
        "h-8 rounded-[var(--radius)] border border-border bg-field px-2 text-[13px] text-fg",
        "outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    >
      {children}
    </select>
  ),
);
Select.displayName = "Select";

export { Select };
