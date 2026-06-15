import { useTranslation } from "react-i18next";
import { Card } from "../components/primitives";
import { Icon, Kbd, type IconName } from "@/components/Icon";
import { useConfigStore } from "@/stores/config";

type ShortcutKey = "selection" | "screenshot" | "overlay" | "openMain";

const ITEMS: { key: ShortcutKey; icon: IconName }[] = [
  { key: "selection", icon: "selection" },
  { key: "screenshot", icon: "capture" },
  { key: "overlay", icon: "overlay" },
  { key: "openMain", icon: "settings" },
];

/** 把 "Alt+D" / "Alt+Cmd+T" 等加速键串拆成展示用的符号键帽 */
function formatKeys(accel: string | null): string[] {
  if (!accel) return ["—"];
  const map: Record<string, string> = {
    alt: "⌥",
    option: "⌥",
    cmd: "⌘",
    command: "⌘",
    super: "⌘",
    meta: "⌘",
    ctrl: "⌃",
    control: "⌃",
    shift: "⇧",
  };
  return accel
    .split("+")
    .map((part) => map[part.toLowerCase().trim()] ?? part.toUpperCase());
}

/** 快捷键 —— 4 项 + 权限提示(对齐设计 ShortcutsPane) */
export function ShortcutsPane() {
  const { t } = useTranslation();
  const shortcuts = useConfigStore((s) => s.config.shortcuts);

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
            <Kbd keys={formatKeys(shortcuts[it.key])} />
            <button
              type="button"
              className="flex cursor-pointer items-center gap-1 rounded-[5px] border-[0.5px] border-line bg-transparent px-2 py-1 text-[11px] text-sub"
            >
              <Icon name="settings" size={11} />
              {t("shortcuts.change")}
            </button>
          </div>
        ))}
      </Card>

      <div className="mt-[14px] flex items-start gap-2.5 rounded-lg border-[0.5px] border-dashed border-line p-3 text-xs text-sub">
        <Icon name="info" size={14} className="mt-px shrink-0" />
        <div>{t("shortcuts.permissionNote")}</div>
      </div>
    </div>
  );
}
