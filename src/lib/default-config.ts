import type { Config, TranslatorEngine } from "./types";

/**
 * 前端兜底默认配置。
 *
 * 权威默认值由 Rust 后端生成(spec 01 §3);此处仅在后端不可用 / 占位阶段
 * 让 store 与 UI 有一致的渲染基线,避免空指针。字段需与 spec 01 §2 对齐。
 *
 * 内置引擎预设镜像 Rust `default_engines()`(spec 04 §12),二者须同步。
 */
function builtinEngines(): TranslatorEngine[] {
  const e = (
    id: string,
    name: string,
    kind: TranslatorEngine["kind"],
    enabled: boolean,
    apiUrl: string,
    model: string | undefined,
    timeoutMs: number,
  ): TranslatorEngine => ({
    id,
    name,
    kind,
    enabled,
    apiUrl,
    apiKey: "",
    ...(model ? { model } : {}),
    timeoutMs,
  });

  return [
    // —— 默认开启 ——
    e("builtin-youdao", "有道翻译", "youdao", true, "", undefined, 15000),
    e("builtin-apple", "苹果翻译", "apple", true, "", undefined, 15000),
    // 谷歌翻译:内置默认 Key,留空即用内置,故默认开启、开箱即用。
    e("builtin-google", "Google 翻译", "google", true, "", undefined, 15000),
    e(
      "builtin-zhipu",
      "智谱 GLM",
      "openai",
      true,
      "https://open.bigmodel.cn/api/paas/v4",
      "glm-4-flash",
      30000,
    ),
    e(
      "builtin-siliconflow",
      "硅基流动",
      "openai",
      true,
      "https://api.siliconflow.cn/v1",
      "Qwen/Qwen2.5-7B-Instruct",
      30000,
    ),
    // —— 默认关闭(需用户自备凭据)——
    e("builtin-baidu", "百度翻译", "baidu", false, "", undefined, 15000),
    e("builtin-microsoft", "微软翻译", "microsoft", false, "", undefined, 15000),
    e(
      "builtin-ollama",
      "Ollama",
      "openai",
      false,
      "http://localhost:11434/v1",
      "qwen2.5",
      60000,
    ),
    e("builtin-lmstudio", "LM Studio", "openai", false, "http://localhost:1234/v1", undefined, 60000),
  ];
}

export function createDefaultConfig(): Config {
  return {
    version: 3,
    app: {
      locale: "system",
      theme: "system",
      autoStart: false,
      checkUpdate: true,
      openOnLaunch: false,
    },
    shortcuts: {
      selection: null,
      screenshot: null,
      overlay: null,
      openMain: null,
    },
    selection: {
      enabled: false,
      autoDetect: false,
      popupDelayMs: 300,
    },
    ocr: {
      engine: "system",
      preferredLanguages: [],
      minConfidence: 0.3,
    },
    translator: {
      defaultEngineId: "builtin-youdao",
      targetLang: "zh-CN",
      engines: builtinEngines(),
    },
    ui: {
      popup: {
        fontSize: 14,
        showOriginal: true,
        width: 360,
      },
    },
  };
}
