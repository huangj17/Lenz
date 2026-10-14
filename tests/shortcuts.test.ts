import assert from "node:assert/strict";
import test from "node:test";
import {
  acceleratorFromKeyboardState,
  formatShortcutKeys,
  isWindowsSettingsShortcut,
  type SettingsShortcutKeyboardState,
  type ShortcutKeyboardState,
} from "../src/lib/shortcuts.ts";

function key(code: string, overrides: Partial<ShortcutKeyboardState> = {}): ShortcutKeyboardState {
  return {
    code,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...overrides,
  };
}

test("records modified letter shortcuts in parser-compatible form", () => {
  assert.equal(acceleratorFromKeyboardState(key("KeyD", { altKey: true })), "Alt+D");
  assert.equal(
    acceleratorFromKeyboardState(key("Digit2", { metaKey: true, shiftKey: true })),
    "Super+Shift+2",
  );
});

test("rejects bare typing keys but permits function keys", () => {
  assert.equal(acceleratorFromKeyboardState(key("KeyA")), null);
  assert.equal(acceleratorFromKeyboardState(key("F12")), "F12");
  assert.equal(acceleratorFromKeyboardState(key("AltLeft", { altKey: true })), null);
});

test("formats accelerators as platform-style key caps", () => {
  assert.deepEqual(formatShortcutKeys("Super+Shift+K"), ["⌘", "⇧", "K"]);
  assert.deepEqual(formatShortcutKeys(null), ["—"]);
  assert.deepEqual(formatShortcutKeys("Control+Alt+Shift+Super+K", "windows"), ["Ctrl", "Alt", "Shift", "Win", "K"]);
});

const WINDOWS_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)";

function settingsKey(overrides: Partial<SettingsShortcutKeyboardState> = {}): SettingsShortcutKeyboardState {
  return {
    ...key("Comma", { ctrlKey: true }),
    key: ",",
    repeat: false,
    isComposing: false,
    defaultPrevented: false,
    ...overrides,
  };
}

test("matches Ctrl+Comma only on Windows", () => {
  assert.equal(isWindowsSettingsShortcut(settingsKey(), WINDOWS_USER_AGENT), true);
  assert.equal(isWindowsSettingsShortcut(settingsKey({ code: "Semicolon" }), WINDOWS_USER_AGENT), true);
  assert.equal(isWindowsSettingsShortcut(settingsKey(), "Mozilla/5.0 (Macintosh; Intel Mac OS X)"), false);
  assert.equal(isWindowsSettingsShortcut(settingsKey(), "Mozilla/5.0 (X11; Linux x86_64)"), false);
  for (const overrides of [
    { ctrlKey: false }, { metaKey: true }, { altKey: true }, { shiftKey: true }, { key: "." },
  ]) {
    assert.equal(isWindowsSettingsShortcut(settingsKey(overrides), WINDOWS_USER_AGENT), false);
  }
});

test("settings shortcut respects recording, IME composition and held keys", () => {
  for (const overrides of [
    { defaultPrevented: true }, { isComposing: true }, { repeat: true },
  ]) {
    assert.equal(isWindowsSettingsShortcut(settingsKey(overrides), WINDOWS_USER_AGENT), false);
  }
});
