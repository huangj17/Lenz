import assert from "node:assert/strict";
import test from "node:test";
import {
  acceleratorFromKeyboardState,
  formatShortcutKeys,
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
});
