import { useEffect } from "react";
import { emit, Events } from "@/lib/ipc";
import { isWindowsSettingsShortcut } from "@/lib/shortcuts";

/** Windows 托盘 accelerator 不处理窗口内按键,在各窗口补上 Ctrl+,。 */
export function useSettingsShortcut() {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!isWindowsSettingsShortcut(event, navigator.userAgent)) return;
      event.preventDefault();
      void emit(Events.ShortcutOpenMain).catch((error: unknown) => {
        console.error("[Lenz] open settings shortcut failed", error);
      });
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
