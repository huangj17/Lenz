/** 可由浏览器 KeyboardEvent 或单元测试夹具提供的最小键盘状态。 */
export interface ShortcutKeyboardState {
  code: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

const MODIFIER_CODES = new Set([
  "AltLeft",
  "AltRight",
  "ControlLeft",
  "ControlRight",
  "MetaLeft",
  "MetaRight",
  "ShiftLeft",
  "ShiftRight",
]);

/** 把物理按键状态转换为 global-hotkey/Tauri 可解析的 accelerator。 */
export function acceleratorFromKeyboardState(e: ShortcutKeyboardState): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;
  let key = e.code;
  if (key.startsWith("Key")) key = key.slice(3);
  else if (key.startsWith("Digit")) key = key.slice(5);
  const modifiers = [
    e.metaKey ? "Super" : "",
    e.ctrlKey ? "Ctrl" : "",
    e.altKey ? "Alt" : "",
    e.shiftKey ? "Shift" : "",
  ].filter(Boolean);
  // 裸字母/数字会抢走所有应用的正常输入;仅功能键允许无修饰键。
  if (modifiers.length === 0 && !/^F(?:[1-9]|1\d|2[0-4])$/.test(key)) return null;
  return [...modifiers, key].join("+");
}

/** 把 accelerator 拆成设置页展示用的符号键帽。 */
export function formatShortcutKeys(accelerator: string | null): string[] {
  if (!accelerator) return ["—"];
  const symbols: Record<string, string> = {
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
  return accelerator
    .split("+")
    .map((part) => symbols[part.toLowerCase().trim()] ?? part.toUpperCase());
}
