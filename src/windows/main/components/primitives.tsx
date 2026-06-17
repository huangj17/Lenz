/**
 * 设置页布局原子 —— 对齐设计稿(settings.jsx 的 Section / Row / Field / Card)。
 */
import { useRef, type ReactNode } from "react";
import { clsx } from "clsx";
import { Icon } from "@/components/Icon";

const PROMPT_VAR = /(\{sourceLang\}|\{targetLang\})/g;

/**
 * 高亮 prompt 中的 `{sourceLang}` / `{targetLang}`。
 * 只改背景/字色/圆角(不加 padding/margin),以免错开背板与 textarea 的字符位置。
 */
function highlightPrompt(text: string): ReactNode[] {
  return text.split(PROMPT_VAR).map((part, i) =>
    /^\{(sourceLang|targetLang)\}$/.test(part) ? (
      <span
        key={i}
        style={{
          background: "var(--tint)",
          color: "var(--accent-ink)",
          borderRadius: 3,
        }}
      >
        {part}
      </span>
    ) : (
      <span key={i}>{part}</span>
    ),
  );
}

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

/** 可编辑文本字段:mono 大写标签 + input(可带尾随节点 / 提示 / mono) */
export function InputField({
  label,
  value,
  onChange,
  onBlur,
  placeholder,
  mono,
  type = "text",
  trailing,
  hint,
  autoComplete,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  onBlur?: () => void;
  placeholder?: string;
  mono?: boolean;
  type?: string;
  trailing?: ReactNode;
  hint?: string;
  autoComplete?: string;
}) {
  return (
    <div className="mb-2.5">
      <div className="mb-1 font-mono text-[11px] uppercase tracking-[0.06em] text-sub">
        {label}
      </div>
      <div className="flex items-center gap-1.5 rounded-md border-[0.5px] border-line bg-field px-2.5 py-1.5 text-[12.5px] focus-within:border-accent">
        <input
          type={type}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onBlur={onBlur}
          placeholder={placeholder}
          autoComplete={autoComplete}
          // 密钥 / URL / 模型等机读字段不做拼写检查,避免红色波浪线。
          spellCheck={mono ? false : undefined}
          className={clsx(
            "min-w-0 flex-1 border-0 bg-transparent text-fg outline-none placeholder:text-sub",
            mono && "font-mono",
          )}
        />
        {trailing}
      </div>
      {hint && <div className="mt-1 text-[11px] text-sub">{hint}</div>}
    </div>
  );
}

/** 下拉选择字段:mono 大写标签 + 原生 select(覆盖自绘箭头) */
export function SelectField({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <div className="mb-2.5">
      <div className="mb-1 font-mono text-[11px] uppercase tracking-[0.06em] text-sub">
        {label}
      </div>
      <div className="relative flex items-center rounded-md border-[0.5px] border-line bg-field">
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="w-full cursor-pointer appearance-none bg-transparent px-2.5 py-1.5 text-[12.5px] text-fg outline-none"
        >
          {options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
        <Icon
          name="chevronDown"
          size={13}
          className="pointer-events-none absolute right-2.5 text-sub"
        />
      </div>
    </div>
  );
}

/** 可编辑多行字段:mono 大写标签 + textarea */
export function TextAreaField({
  label,
  value,
  onChange,
  onBlur,
  placeholder,
  rows = 3,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  onBlur?: () => void;
  placeholder?: string;
  rows?: number;
}) {
  return (
    <div className="mb-2.5">
      <div className="mb-1 font-mono text-[11px] uppercase tracking-[0.06em] text-sub">
        {label}
      </div>
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onBlur}
        placeholder={placeholder}
        rows={rows}
        className="w-full resize-none rounded-md border-[0.5px] border-line bg-field px-2.5 py-2 text-xs leading-relaxed text-fg outline-none placeholder:text-sub focus:border-accent"
      />
    </div>
  );
}

/**
 * 高亮可编辑 Prompt 字段:透明文本的 textarea 叠在高亮背板上,
 * 二者用同一套字体/行高/内边距对齐;滚动时同步背板。
 */
export function PromptField({
  label,
  value,
  onChange,
  onBlur,
  placeholder,
  rows = 3,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  onBlur?: () => void;
  placeholder?: string;
  rows?: number;
}) {
  const taRef = useRef<HTMLTextAreaElement>(null);
  const bgRef = useRef<HTMLDivElement>(null);
  const syncScroll = () => {
    if (bgRef.current && taRef.current)
      bgRef.current.scrollTop = taRef.current.scrollTop;
  };
  return (
    <div className="mb-2.5">
      <div className="mb-1 font-mono text-[11px] uppercase tracking-[0.06em] text-sub">
        {label}
      </div>
      <div className="relative rounded-md border-[0.5px] border-line bg-field focus-within:border-accent">
        <div
          ref={bgRef}
          aria-hidden
          className="pointer-events-none absolute inset-0 overflow-hidden whitespace-pre-wrap break-words px-2.5 py-2 text-xs leading-relaxed text-fg"
        >
          {/* 末尾换行补一个空格,保证背板和 textarea 行数一致 */}
          {highlightPrompt(value + (value.endsWith("\n") ? " " : ""))}
        </div>
        <textarea
          ref={taRef}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onBlur={onBlur}
          onScroll={syncScroll}
          placeholder={placeholder}
          rows={rows}
          spellCheck={false}
          style={{ caretColor: "var(--fg)" }}
          className="relative block w-full resize-none border-0 bg-transparent px-2.5 py-2 text-xs leading-relaxed text-transparent outline-none placeholder:text-sub"
        />
      </div>
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
