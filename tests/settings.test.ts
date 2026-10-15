import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { searchSettings, moveLanguage, type SettingsSearchItem } from "../src/lib/settingsSearch.ts";
import { fitOverlayFont, overlayLayout } from "../src/lib/overlayLayout.ts";

test("settings search handles bilingual keywords and requires every term", () => {
  const items: SettingsSearchItem[] = [
    { id: "ocr", section: "ocr", label: "OCR 语言", keywords: "language languages 识别" },
    { id: "timeout", section: "translate", label: "请求超时", keywords: "engine timeout 翻译" },
  ];
  assert.deepEqual(searchSettings(items, " OCR LANGUAGE ").map((item) => item.id), ["ocr"]);
  assert.deepEqual(searchSettings(items, "识别").map((item) => item.id), ["ocr"]);
  assert.deepEqual(searchSettings(items, "engine language"), []);
  assert.deepEqual(searchSettings(items, "  "), []);
});

test("language ordering preserves every entry and leaves the source unchanged", () => {
  const source = ["ja", "en-US", "zh-Hans"];
  assert.deepEqual(moveLanguage(source, "ja", "zh-Hans"), ["en-US", "zh-Hans", "ja"]);
  assert.deepEqual(moveLanguage(source, "zh-Hans", "ja"), ["zh-Hans", "ja", "en-US"]);
  assert.deepEqual(source, ["ja", "en-US", "zh-Hans"]);
  assert.equal(moveLanguage(source, "missing", "ja"), source);
});

test("overlay text scales to the box with a readable minimum and bounded maximum", () => {
  const measure = (text: string, size: number) => text.length * size;
  assert.equal(fitOverlayFont("Hello", 104, 40, measure), 20);
  assert.equal(fitOverlayFont("Long translation", 10, 20, measure), 8);
  assert.equal(fitOverlayFont("A", 300, 300, measure), 32);
});

test("overlay preserves negative screen coordinates and keeps close controls in the viewport", () => {
  const viewport = { x: -2560, y: -171, w: 2560, h: 1440 };
  const layout = overlayLayout({ x: -1610, y: 129, w: 700, h: 390 }, viewport);
  assert.equal(layout.left, 950);
  assert.equal(layout.top, 300);
  assert.equal(layout.toolbar.top, 244);
  for (const region of [
    { x: -2560, y: -171, w: 20, h: 20 },
    viewport,
    { x: -20, y: 1200, w: 20, h: 20 },
    { x: -10000, y: -9000, w: 100, h: 80 },
    { x: 3000, y: 3000, w: 100, h: 80 },
  ]) {
    const { toolbar } = overlayLayout(region, viewport);
    assert.ok(toolbar.left >= 12);
    assert.ok(toolbar.left + toolbar.width <= viewport.w - 12);
    assert.ok(toolbar.top >= 12 && toolbar.top + 44 <= viewport.h - 12);
  }
});

test("Chinese and English settings translation keys stay in sync", () => {
  function keys(value: unknown, prefix = ""): string[] {
    if (!value || typeof value !== "object") return [prefix];
    return Object.entries(value).flatMap(([key, child]) => keys(child, prefix ? prefix + "." + key : key));
  }
  const read = (locale: string) => JSON.parse(readFileSync(new URL("../src/locales/" + locale + ".json", import.meta.url), "utf8"));
  assert.deepEqual(keys(read("zh-CN")).sort(), keys(read("en-US")).sort());
});
