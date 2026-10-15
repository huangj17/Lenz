import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { tencentOcrErrorKey } from "../src/lib/ocr-errors.ts";

test("Tencent errors localize across sample, screenshot Display, and overlay payloads", () => {
  for (const message of [
    "ocr_tencent_auth",
    "ocr failed: ocr_tencent_auth",
    "config error: ocr_tencent_auth",
  ]) {
    assert.equal(tencentOcrErrorKey(message), "ocr.errors.ocr_tencent_auth");
  }
  for (const message of ["ocr_tencent_auth sensitive", "ocr_tencent_unknown", "no text recognized", "ocr_baidu_17"]) {
    assert.equal(tencentOcrErrorKey(message), null);
  }
});

test("Tencent error and service labels are translated in both locales", () => {
  const zh = JSON.parse(readFileSync(new URL("../src/locales/zh-CN.json", import.meta.url), "utf8")).ocr;
  const en = JSON.parse(readFileSync(new URL("../src/locales/en-US.json", import.meta.url), "utf8")).ocr;
  const codes = Object.keys(zh.errors).filter((key) => key.startsWith("ocr_tencent_"));
  for (const code of codes) {
    assert.equal(tencentOcrErrorKey(code), `ocr.errors.${code}`);
    assert.ok(en.errors[code]);
  }
  assert.deepEqual(Object.keys(en.errors).filter((key) => key.startsWith("ocr_tencent_")), codes);
  for (const locale of [zh, en]) {
    assert.ok(locale.kinds.tencent);
    assert.ok(locale.tencentScene);
    assert.ok(locale.tencentGeneral);
    assert.ok(locale.tencentMultilingual);
  }
});
