import type { Config } from "./types";

/**
 * 前端兜底默认配置。
 *
 * 权威默认值由 Rust 后端生成(spec 01 §3);此处仅在后端不可用 / 占位阶段
 * 让 store 与 UI 有一致的渲染基线,避免空指针。字段需与 spec 01 §2 对齐。
 */
export function createDefaultConfig(): Config {
  return {
    version: 1,
    app: {
      locale: "system",
      theme: "system",
      autoStart: false,
      checkUpdate: true,
      openOnLaunch: true,
    },
    shortcuts: {
      selection: null,
      screenshot: null,
      overlay: null,
      openMain: null,
    },
    selection: {
      enabled: true,
      autoDetect: false,
      popupDelayMs: 300,
    },
    ocr: {
      engine: "system",
      preferredLanguages: [],
      minConfidence: 0.3,
    },
    translator: {
      defaultEngineId: "",
      targetLang: "zh-CN",
      engines: [],
    },
    proxy: {
      enabled: false,
      scheme: "http",
      host: "",
      port: 0,
      bypassLocal: true,
    },
    ui: {
      popup: {
        fontSize: 14,
        showOriginal: true,
      },
    },
  };
}
