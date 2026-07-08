/**
 * Lenz 定制图标集 —— 移植自设计交接包 docs/design/project/src/icons.jsx。
 * 20×20 viewBox,1.5px 描边,currentColor;克制、不喧宾夺主。
 *
 * 严格保持与设计稿一致的图形,故不使用 lucide-react。
 */
import type { CSSProperties, ReactElement } from "react";

export type IconName =
  | "lens"
  | "selection"
  | "capture"
  | "overlay"
  | "settings"
  | "keyboard"
  | "cpu"
  | "ocr"
  | "info"
  | "close"
  | "plus"
  | "check"
  | "chevronDown"
  | "chevronRight"
  | "copy"
  | "search"
  | "play"
  | "pause"
  | "moon"
  | "sun"
  | "swap"
  | "globe"
  | "dot"
  | "drag"
  | "external"
  | "trash"
  | "star"
  | "command"
  | "option";

interface IconProps {
  name: IconName;
  size?: number;
  stroke?: number;
  className?: string;
  style?: CSSProperties;
}

export function Icon({
  name,
  size = 16,
  stroke = 1.5,
  className,
  style,
}: IconProps) {
  const p = {
    fill: "none",
    stroke: "currentColor",
    strokeWidth: stroke,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
  };
  const f = { fill: "currentColor", stroke: "none" };

  const paths: Record<IconName, ReactElement> = {
    lens: (
      <>
        <circle cx="10" cy="10" r="7" {...p} />
        <circle cx="10" cy="10" r="3" {...p} />
      </>
    ),
    selection: (
      <>
        <path
          d="M4 4h3M13 4h3M16 7v3M16 13v3M13 16h3M4 16h3M4 13v-3M4 10V7"
          {...p}
        />
        <path d="M9 9l2 5 1-2 2-1z" {...p} />
      </>
    ),
    capture: (
      <>
        <path
          d="M3 7V5a1 1 0 0 1 1-1h2M14 4h2a1 1 0 0 1 1 1v2M17 13v2a1 1 0 0 1-1 1h-2M6 16H4a1 1 0 0 1-1-1v-2"
          {...p}
        />
        <circle cx="10" cy="10" r="3" {...p} />
      </>
    ),
    overlay: (
      <>
        <rect x="3" y="3" width="10" height="10" rx="1.5" {...p} />
        <rect x="7" y="7" width="10" height="10" rx="1.5" {...p} />
      </>
    ),
    settings: (
      <>
        <circle cx="10" cy="10" r="2.5" {...p} />
        <path
          d="M10 2v2M10 16v2M18 10h-2M4 10H2M15.7 4.3l-1.4 1.4M5.7 14.3l-1.4 1.4M15.7 15.7l-1.4-1.4M5.7 5.7L4.3 4.3"
          {...p}
        />
      </>
    ),
    keyboard: (
      <>
        <rect x="2" y="5" width="16" height="10" rx="2" {...p} />
        <path d="M5 9h.01M8 9h.01M11 9h.01M14 9h.01M6 12h8" {...p} />
      </>
    ),
    cpu: (
      <>
        <rect x="5" y="5" width="10" height="10" rx="1.5" {...p} />
        <path d="M8 8h4v4H8z" {...p} />
        <path d="M8 2v3M12 2v3M8 15v3M12 15v3M2 8h3M2 12h3M15 8h3M15 12h3" {...p} />
      </>
    ),
    ocr: (
      <>
        <rect x="3" y="3" width="14" height="14" rx="2" {...p} />
        <path d="M6 8h2M10 8h4M6 12h5M13 12h1" {...p} />
      </>
    ),
    info: (
      <>
        <circle cx="10" cy="10" r="7" {...p} />
        <path d="M10 9v4M10 7v.01" {...p} />
      </>
    ),
    close: <path d="M5 5l10 10M15 5L5 15" {...p} />,
    plus: <path d="M10 4v12M4 10h12" {...p} />,
    check: <path d="M4 10l4 4 8-8" {...p} />,
    chevronDown: <path d="M5 8l5 5 5-5" {...p} />,
    chevronRight: <path d="M8 5l5 5-5 5" {...p} />,
    copy: (
      <>
        <rect x="6" y="6" width="10" height="10" rx="1.5" {...p} />
        <path d="M4 12V5a1 1 0 0 1 1-1h7" {...p} />
      </>
    ),
    search: (
      <>
        <circle cx="9" cy="9" r="5" {...p} />
        <path d="M13 13l3 3" {...p} />
      </>
    ),
    play: <path d="M6 4l10 6-10 6z" {...p} />,
    pause: <path d="M7 5v10M13 5v10" {...p} />,
    moon: <path d="M15 11a5 5 0 0 1-6-6 6 6 0 1 0 6 6z" {...p} />,
    sun: (
      <>
        <circle cx="10" cy="10" r="3" {...p} />
        <path
          d="M10 2v2M10 16v2M18 10h-2M4 10H2M15.7 4.3l-1.4 1.4M5.7 14.3l-1.4 1.4M15.7 15.7l-1.4-1.4M5.7 5.7L4.3 4.3"
          {...p}
        />
      </>
    ),
    swap: (
      <path
        d="M5 7h10M5 7l3-3M5 7l3 3M15 13H5M15 13l-3-3M15 13l-3 3"
        {...p}
      />
    ),
    globe: (
      <>
        <circle cx="10" cy="10" r="7" {...p} />
        <path
          d="M3 10h14M10 3c2 2 3 4.5 3 7s-1 5-3 7c-2-2-3-4.5-3-7s1-5 3-7z"
          {...p}
        />
      </>
    ),
    dot: <circle cx="10" cy="10" r="2" {...f} />,
    drag: (
      <>
        <circle cx="7" cy="6" r="1" {...f} />
        <circle cx="7" cy="10" r="1" {...f} />
        <circle cx="7" cy="14" r="1" {...f} />
        <circle cx="13" cy="6" r="1" {...f} />
        <circle cx="13" cy="10" r="1" {...f} />
        <circle cx="13" cy="14" r="1" {...f} />
      </>
    ),
    external: (
      <path
        d="M11 3h6v6M17 3l-7 7M9 5H5a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-4"
        {...p}
      />
    ),
    trash: (
      <path
        d="M4 6h12M8 6V4h4v2M6 6v10a2 2 0 0 0 2 2h4a2 2 0 0 0 2-2V6"
        {...p}
      />
    ),
    star: <path d="M10 3l2.2 4.6 5 .7-3.6 3.5.9 5-4.5-2.4L5.5 17l.9-5L2.8 8.3l5-.7z" {...p} />,
    command: (
      <path
        d="M6 6a2 2 0 1 0 2 2V6m0 0h4m0 0v2a2 2 0 1 0 2-2m-2 2v4m0 0v2a2 2 0 1 1-2-2m2 0H8m0 0a2 2 0 1 1-2 2v-2"
        {...p}
      />
    ),
    option: <path d="M3 5h4l5 10h5M12 5h5" {...p} />,
  };

  return (
    <svg
      viewBox="0 0 20 20"
      width={size}
      height={size}
      className={className}
      style={style}
      aria-hidden="true"
    >
      {paths[name]}
    </svg>
  );
}

/** macOS 键帽组合(如 ⌥ D) */
export function Kbd({ keys, onDark = false }: { keys: string[]; onDark?: boolean }) {
  return (
    <span className="inline-flex items-center gap-[3px]">
      {keys.map((k, i) => (
        <span key={i} className={onDark ? "ln-kbd ln-kbd--on-dark" : "ln-kbd"}>
          {k}
        </span>
      ))}
    </span>
  );
}
