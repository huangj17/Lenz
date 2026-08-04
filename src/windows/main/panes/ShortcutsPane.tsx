import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Card } from "../components/primitives";
import { Icon, Kbd, type IconName } from "@/components/Icon";
import { acceleratorFromKeyboardState, formatShortcutKeys } from "@/lib/shortcuts";
import { useConfigStore } from "@/stores/config";

type ShortcutKey = "selection" | "screenshot" | "overlay" | "openMain";

const ITEMS: { key: ShortcutKey; icon: IconName }[] = [
  { key: "selection", icon: "selection" },
  { key: "screenshot", icon: "capture" },
  { key: "overlay", icon: "overlay" },
  { key: "openMain", icon: "settings" },
];

/** 快捷键 —— 4 项 + 权限提示(对齐设计 ShortcutsPane) */
export function ShortcutsPane() {
  const { t } = useTranslation();
  const shortcuts = useConfigStore((s) => s.config.shortcuts);
  const patch = useConfigStore((s) => s.patch);
  const [editing, setEditing] = useState<ShortcutKey | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!editing) return;
    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setEditing(null);
        setError(null);
        return;
      }
      if (e.key === "Backspace" || e.key === "Delete") {
        void patch({ shortcuts: { [editing]: null } });
        setEditing(null);
        setError(null);
        return;
      }
      const accelerator = acceleratorFromKeyboardState(e);
      if (!accelerator) {
        setError(t("shortcuts.modifierRequired"));
        return;
      }
      const duplicate = ITEMS.find(
        ({ key }) => key !== editing && shortcuts[key]?.toLowerCase() === accelerator.toLowerCase(),
      );
      if (duplicate) {
        setError(t("shortcuts.duplicate", { name: t(`shortcuts.${duplicate.key}`) }));
        return;
      }
      void patch({ shortcuts: { [editing]: accelerator } });
      setEditing(null);
      setError(null);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [editing, patch, shortcuts, t]);

  return (
    <div>
      <Card>
        {ITEMS.map((it, i) => (
          <div
            key={it.key}
            className={`flex items-center gap-3 px-[14px] py-[14px] ${
              i < ITEMS.length - 1 ? "border-b-[0.5px] border-line" : ""
            }`}
          >
            <div className="flex size-7 items-center justify-center rounded-[7px] bg-tint text-accent-ink">
              <Icon name={it.icon} size={15} />
            </div>
            <div className="flex-1">
              <div className="text-[13px] font-medium">
                {t(`shortcuts.${it.key}`)}
              </div>
              <div className="mt-0.5 text-[11px] text-sub">
                {t(`shortcuts.${it.key}Hint`)}
              </div>
            </div>
            <Kbd keys={formatShortcutKeys(shortcuts[it.key])} />
            <button
              type="button"
              onClick={() => {
                setEditing(it.key);
                setError(null);
              }}
              aria-pressed={editing === it.key}
              className="flex cursor-pointer items-center gap-1 rounded-[5px] border-[0.5px] border-line bg-transparent px-2 py-1 text-[11px] text-sub"
            >
              <Icon name="settings" size={11} />
              {editing === it.key ? t("shortcuts.recording") : t("shortcuts.change")}
            </button>
          </div>
        ))}
      </Card>

      {(editing || error) && (
        <div className={`mt-2 text-xs ${error ? "text-danger" : "text-sub"}`} role="status">
          {error ?? t("shortcuts.recordingHint")}
        </div>
      )}

      <div className="mt-[14px] flex items-start gap-2.5 rounded-lg border-[0.5px] border-dashed border-line p-3 text-xs text-sub">
        <Icon name="info" size={14} className="mt-px shrink-0" />
        <div>{t("shortcuts.permissionNote")}</div>
      </div>
    </div>
  );
}
