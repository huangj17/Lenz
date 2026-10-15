import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "@/components/Icon";
import type { PopupTranslationRow } from "@/lib/popupTranslations";

interface Props {
  row: PopupTranslationRow;
  name: string;
  primary: boolean;
  isDefault: boolean;
  copied: boolean;
  errorText: string;
  saveError?: string;
  onCopy: (text: string) => void;
  onRetry: () => void;
  onCollapse: () => void;
  onRemember: (enabled: boolean) => void;
  onSkip: (enabled: boolean) => void;
}

export function PopupServiceRow({ row, name, primary, isDefault, copied, errorText, saveError,
  onCopy, onRetry, onCollapse, onRemember, onSkip }: Props) {
  const { t } = useTranslation();
  const [menuOpen, setMenuOpen] = useState(false);
  const root = useRef<HTMLElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const menuButton = useRef<HTMLButtonElement>(null);
  const contentId = "popup-service-content-" + row.engineId;

  useEffect(() => {
    if (!menuOpen) return;
    menu.current?.querySelector<HTMLElement>('[role^="menuitem"]')?.focus();
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setMenuOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [menuOpen]);

  const onMenuKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      setMenuOpen(false);
      menuButton.current?.focus();
      return;
    }
    if (event.key === "Tab") { setMenuOpen(false); return; }
    const items = [...(menu.current?.querySelectorAll<HTMLElement>('[role^="menuitem"]') ?? [])];
    const index = items.indexOf(document.activeElement as HTMLElement);
    const next = event.key === "ArrowDown" ? (index + 1) % items.length
      : event.key === "ArrowUp" ? (index - 1 + items.length) % items.length
        : event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : -1;
    if (next >= 0) { event.preventDefault(); items[next]?.focus(); }
  };

  return (
    <section ref={root} aria-label={name}
      className={"border-b border-l-2 border-b-line px-3 py-2.5 last:border-b-0 " + (primary ? "border-l-accent" : "border-l-transparent")}>
      <div className="flex items-center gap-1.5">
        <button type="button" onClick={() => { setMenuOpen(false); onCollapse(); }} aria-expanded={!row.collapsed} aria-controls={contentId}
          aria-label={t(row.collapsed ? "popup.expandService" : "popup.collapseService", { service: name })}
          className="flex min-w-0 flex-1 items-center gap-1.5 rounded text-left hover:text-accent-ink">
          <Icon name={row.collapsed ? "chevronRight" : "chevronDown"} size={12} className="shrink-0 text-sub" />
          <span className="ln-engine-badge grid size-[18px] shrink-0 place-items-center rounded font-mono text-[9px] font-bold text-white">
            {name.trim().replace(/\s+/g, "").slice(0, 2).toUpperCase() || "EN"}
          </span>
          <span className="truncate text-[12px] font-semibold text-fg">{name}</span>
          {isDefault && <span className="shrink-0 rounded bg-tint px-1 text-[9px] text-accent-ink">{t("engines.default")}</span>}
        </button>
        <span className="flex shrink-0 items-center gap-1 text-[10px] text-sub">
          {row.status === "success" ? <><Icon name="check" size={10} className="text-[color:var(--sage)]" /><span className="font-mono">{row.costMs}ms</span></>
            : row.status === "error" ? <Icon name="info" size={12} className="text-danger" />
              : row.status === "idle" ? <span>{t("popup.waitingToExpand")}</span>
                : <span role="status" aria-label={t("popup.translating")} className="h-3 w-3 animate-spin rounded-full border-[1.5px] border-line border-t-accent" />}
        </span>
        <button ref={menuButton} type="button" onClick={() => setMenuOpen((open) => !open)}
          aria-label={t("popup.serviceOptions", { service: name })} aria-haspopup="menu" aria-expanded={menuOpen}
          className="grid size-6 shrink-0 place-items-center rounded text-sub hover:bg-field hover:text-fg">
          <Icon name="gear" size={14} />
        </button>
      </div>

      {menuOpen && (
        <div ref={menu} role="menu" aria-label={t("popup.serviceOptions", { service: name })} onKeyDown={onMenuKey}
          className="mt-2 rounded-md border border-line bg-card p-1 text-xs text-fg">
          <button type="button" role="menuitemcheckbox" aria-checked={row.rememberCollapse} onClick={() => onRemember(!row.rememberCollapse)}
            className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left hover:bg-field focus:bg-field">
            <span className="grid size-3 place-items-center">{row.rememberCollapse && <Icon name="check" size={12} />}</span>
            {t("popup.rememberCollapse")}
          </button>
          <button type="button" role="menuitemcheckbox" aria-checked={row.skipWhenCollapsed} onClick={() => onSkip(!row.skipWhenCollapsed)}
            className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left hover:bg-field focus:bg-field">
            <span className="grid size-3 place-items-center">{row.skipWhenCollapsed && <Icon name="check" size={12} />}</span>
            {t("popup.skipCollapsed")}
          </button>
          <p className="px-2 py-1 text-[10px] leading-relaxed text-sub">{t("popup.collapseHint")}</p>
          {saveError && <p role="alert" className="px-2 py-1 text-[10px] text-danger">{saveError}</p>}
        </div>
      )}

      {!row.collapsed && (
        <div id={contentId}>
          {row.status === "success" ? <>
            <p className="mt-2 select-text whitespace-pre-wrap break-words pl-[34px] text-[13.5px] leading-relaxed text-fg [overflow-wrap:anywhere]">{row.translation}</p>
            <button type="button" onClick={() => onCopy(row.translation)} className="ml-[34px] mt-2.5 inline-flex items-center gap-1.5 text-[11px] text-sub hover:text-fg">
              <Icon name={copied ? "check" : "copy"} size={13} />{t(copied ? "popup.copied" : "popup.copy")}
            </button>
          </> : row.status === "error" ? <div className="mt-2 pl-[34px]">
            <p role="alert" className="select-text whitespace-pre-wrap break-words text-xs text-danger [overflow-wrap:anywhere]">{errorText}</p>
            <button type="button" onClick={onRetry} className="mt-2 text-[11px] text-accent-ink">{t("popup.retry")}</button>
          </div> : <div className="mt-2 flex flex-col gap-2 pl-[34px]">
            <div className="h-3 w-5/6 animate-pulse rounded bg-field" /><div className="h-3 w-2/3 animate-pulse rounded bg-field" />
          </div>}
        </div>
      )}
    </section>
  );
}
