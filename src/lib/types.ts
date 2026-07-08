/**
 * 前后端共享类型契约(手写)。
 *
 * 注意:Rust 后端 agent 会用 tauri-specta 生成 `src/lib/bindings.ts`。
 * 在生成链路打通前,本文件作为前端与后端约定的"权威契约",二者必须保持一致。
 * 线缆格式统一为 camelCase。
 *
 * 参考:docs/spec/00-architecture.md §5、docs/spec/01-config-storage.md §2
 */

// ──────────────────────────────────────────────────────────────────────────
// 基础几何 / 触发类型
// ──────────────────────────────────────────────────────────────────────────

/** 矩形(逻辑坐标) */
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** 功能触发类型 */
export type TriggerKind = "selection" | "screenshot" | "overlay";

// ──────────────────────────────────────────────────────────────────────────
// 错误
// ──────────────────────────────────────────────────────────────────────────

/** 统一错误码,与 Rust 侧 LenzError 枚举判别值同构 */
export type LenzErrorCode =
  | "Config"
  | "OcrUnavailable"
  | "OcrFailed"
  | "TranslateFailed"
  | "Network"
  | "Permission"
  | "Internal";

/**
 * 统一错误结构。
 *
 * Rust 侧使用 `#[serde(tag = "code", content = "message")]`,
 * 因此线缆形态为 `{ code: "Network", message: "timeout" }`。
 */
export interface LenzError {
  code: LenzErrorCode;
  message: string;
}

// ──────────────────────────────────────────────────────────────────────────
// 翻译引擎
// ──────────────────────────────────────────────────────────────────────────

export type TranslatorEngineKind =
  | "openai"
  | "google"
  | "deepl"
  | "youdao"
  | "microsoft"
  | "apple";

export interface TranslatorEngine {
  /** uuid 或内置 slug(如 "builtin-youdao") */
  id: string;
  /** 用户自定义名称 */
  name: string;
  kind: TranslatorEngineKind;
  enabled: boolean;
  apiUrl: string;
  apiKey: string;
  /** LLM 类必填 */
  model?: string;
  /** LLM 类,支持 {sourceLang}/{targetLang} 变量 */
  systemPrompt?: string;
  /** 默认 15000 */
  timeoutMs: number;
  /** Azure 翻译区域(仅 microsoft 引擎用) */
  region?: string;
  /** LLM 最大输出 token(仅 openai);省略=不限 */
  maxTokens?: number;
  /** LLM 采样温度(仅 openai);省略=默认 0.1 */
  temperature?: number;
}

// ──────────────────────────────────────────────────────────────────────────
// 配置(无 history 字段 —— 历史记录功能已删除)
// ──────────────────────────────────────────────────────────────────────────

export type LocaleSetting = "system" | "zh-CN" | "en-US";
export type ThemeSetting = "system" | "light" | "dark";

export interface AppConfig {
  locale: LocaleSetting;
  theme: ThemeSetting;
  autoStart: boolean;
  checkUpdate: boolean;
  /** 启动时是否展示主窗口 */
  openOnLaunch: boolean;
}

export interface ShortcutsConfig {
  /** 如 "Alt+D",null 表示禁用 */
  selection: string | null;
  screenshot: string | null;
  overlay: string | null;
  openMain: string | null;
}

export interface SelectionConfig {
  /** 总开关 */
  enabled: boolean;
  /** 是否启用自动弹框 */
  autoDetect: boolean;
  /** 选中后延迟触发,防抖 */
  popupDelayMs: number;
}

export interface OcrConfig {
  /** MVP 仅系统 */
  engine: "system";
  /** BCP-47 */
  preferredLanguages: string[];
  /** 0~1 */
  minConfidence: number;
}

export interface TranslatorConfig {
  defaultEngineId: string;
  /** 默认 "zh-CN" */
  targetLang: string;
  engines: TranslatorEngine[];
}

export interface PopupUiConfig {
  /** 默认 14 */
  fontSize: number;
  showOriginal: boolean;
  /** 弹框逻辑宽度(可拖拽调节并记忆),默认 360;夹 [300, 720] */
  width: number;
}

export interface UiConfig {
  popup: PopupUiConfig;
}

export interface Config {
  /** schema 版本号,用于迁移 */
  version: number;
  app: AppConfig;
  shortcuts: ShortcutsConfig;
  selection: SelectionConfig;
  ocr: OcrConfig;
  translator: TranslatorConfig;
  ui: UiConfig;
}

// ──────────────────────────────────────────────────────────────────────────
// 翻译请求 / 响应(spec 00 §5)
// ──────────────────────────────────────────────────────────────────────────

export interface TranslateRequest {
  texts: string[];
  sourceLang?: string;
  targetLang: string;
  /** undefined = 默认引擎 */
  engineId?: string;
}

export interface TranslateResponse {
  translations: string[];
  detectedSourceLang?: string;
  engineId: string;
  costMs: number;
}

/** test_engine 命令返回 */
export interface TestEngineResult {
  ok: boolean;
  latencyMs: number;
  sample: string;
  error?: string;
}

// ──────────────────────────────────────────────────────────────────────────
// 事件载荷(spec 00 §4.3)
// ──────────────────────────────────────────────────────────────────────────

/** `selection://text-changed` */
export interface SelectionTextChangedPayload {
  text: string;
  lang?: string;
}

/** `selection://detected` —— 监听到划词/双击选中,坐标为光标处全局逻辑坐标 */
export interface SelectionDetectedPayload {
  x: number;
  y: number;
}

/** `selection://permission-required` —— 缺少辅助功能权限 */
export interface PermissionRequiredPayload {
  platform: string;
  detail: string;
}

/** `capture://region-selected` */
export interface CaptureRegionSelectedPayload extends Rect {
  displayId?: number;
  trigger?: TriggerKind;
}

/** `overlay://closed` */
export interface OverlayClosedPayload {
  id: string;
}

// ── 截图翻译 F2 事件载荷(spec 06)──────────────────────────────────────────

/** 各阶段耗时(毫秒) */
export interface ScreenshotCost {
  capture: number;
  ocr: number;
  translate: number;
}

/** `screenshot://result` */
export interface ScreenshotResult {
  original: string;
  translation: string;
  sourceLang: string | null;
  targetLang: string;
  engineId: string;
  region: Rect;
  cost: ScreenshotCost;
}

/** `screenshot://error` —— stage: capture/ocr/translate/timeout */
export interface ScreenshotErrorPayload {
  stage: "capture" | "ocr" | "translate" | "timeout";
  /** permission / no_text / failed / timeout */
  code: string;
  message: string;
  /** 翻译阶段失败时附 OCR 原文 */
  original?: string;
}

/** `screenshot://progress` */
export interface ScreenshotProgressPayload {
  stage: "capture" | "ocr" | "translate";
}

/** `screenshot://translating` —— 进入翻译阶段(原文已知、译文未出),供 popup 显示完整布局 + 引擎行转圈 */
export interface ScreenshotTranslatingPayload {
  original: string;
  sourceLang: string | null;
  targetLang: string;
  engineId: string;
}

/** `config://changed` —— 变更的 keypath 列表(如 ["shortcuts.selection"]) */
export type ConfigChangedPayload = string[];

/** `app://startup-warning` */
export interface StartupWarningPayload {
  step: string;
  message: string;
}

// ──────────────────────────────────────────────────────────────────────────
// 工具类型
// ──────────────────────────────────────────────────────────────────────────

/** 递归可选(用于 set_config 的 JSON Merge Patch) */
export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends Array<infer U>
    ? Array<U>
    : T[P] extends object
      ? DeepPartial<T[P]>
      : T[P];
};

// ──────────────────────────────────────────────────────────────────────────
// OCR / 截图(spec 02 / 03)
// ──────────────────────────────────────────────────────────────────────────

export interface OcrBlock {
  text: string;
  /** 已回映射的屏幕逻辑坐标 */
  bbox: Rect;
  confidence: number;
}

export interface OcrResult {
  blocks: OcrBlock[];
  detectedLanguage: string | null;
  costMs: number;
}

export interface OcrOptions {
  preferredLanguages: string[];
  fastMode: boolean;
  minConfidence: number;
}

/** 截图采集结果;png 经 IPC 序列化为字节数组 */
export interface CaptureResult {
  png: number[];
  pixelRect: Rect;
  scaleFactor: number;
}
