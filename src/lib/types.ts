/**
 * 前后端共享类型契约。
 *
 * Rust command 与事件类型由 tauri-specta 生成到 `src/lib/bindings.ts`;本文件只做
 * 前端友好别名、有限值收窄及尚无 Rust 事件的本地类型。不要在这里复制 Rust schema。
 *
 * 参考:docs/spec/00-architecture.md §5、docs/spec/01-config-storage.md §2
 */

import type {
  AppConfig as RustAppConfig,
  CaptureResult as RustCaptureResult,
  ConfigChangedPayload as RustConfigChangedPayload,
  Config_Serialize as RustConfig,
  EngineKind,
  LenzError as RustLenzError,
  Locale,
  OcrBlock as RustOcrBlock,
  OcrConfig as RustOcrConfig,
  OcrResult as RustOcrResult,
  PermissionRequired,
  PopupUiConfig as RustPopupUiConfig,
  Rect as RustRect,
  RegionSelectedPayload,
  ScreenshotCost as RustScreenshotCost,
  ScreenshotError_Serialize,
  ScreenshotProgress,
  ScreenshotResult as RustScreenshotResult,
  ScreenshotTranslating,
  SelectionConfig as RustSelectionConfig,
  SelectionDetected,
  ShortcutConfig,
  StartupWarning,
  TestEngineResult as RustTestEngineResult,
  Theme,
  TranslateResponse as RustTranslateResponse,
  TranslatorConfig_Serialize,
  TranslatorEngine_Serialize,
  TriggerKind as RustTriggerKind,
  UiConfig as RustUiConfig,
} from "./bindings";
export type { SystemInfo, SystemResource, UpdateInfo, OverlayPayload, OverlayBlock, OverlayProgress, OverlayUpdated } from "./bindings";

// ──────────────────────────────────────────────────────────────────────────
// 基础几何 / 触发类型
// ──────────────────────────────────────────────────────────────────────────

/** 矩形(逻辑坐标) */
export type Rect = RustRect;

/** 功能触发类型 */
export type TriggerKind = RustTriggerKind;

// ──────────────────────────────────────────────────────────────────────────
// 错误
// ──────────────────────────────────────────────────────────────────────────

/** 统一错误码,与 Rust 侧 LenzError 枚举判别值同构 */
export type LenzErrorCode = RustLenzError["code"];

/**
 * 统一错误结构。
 *
 * Rust 侧使用 `#[serde(tag = "code", content = "message")]`,
 * 因此线缆形态为 `{ code: "Network", message: "timeout" }`。
 */
export type LenzError = RustLenzError;

// ──────────────────────────────────────────────────────────────────────────
// 翻译引擎
// ──────────────────────────────────────────────────────────────────────────

export type TranslatorEngineKind = EngineKind;
export type TranslatorEngine = TranslatorEngine_Serialize;

// ──────────────────────────────────────────────────────────────────────────
// 配置(无 history 字段 —— 历史记录功能已删除)
// ──────────────────────────────────────────────────────────────────────────

export type LocaleSetting = Locale;
export type ThemeSetting = Theme;
export type AppConfig = RustAppConfig;
export type ShortcutsConfig = ShortcutConfig;
export type SelectionConfig = RustSelectionConfig;
export type OcrConfig = Omit<RustOcrConfig, "minConfidence"> & { minConfidence: number };
export type TranslatorConfig = TranslatorConfig_Serialize;
export type PopupUiConfig = RustPopupUiConfig;
export type UiConfig = RustUiConfig;
export type Config = Omit<RustConfig, "ocr"> & { ocr: OcrConfig };

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

export type TranslateResponse = RustTranslateResponse;

/** test_engine 命令返回 */
export type TestEngineResult = RustTestEngineResult;

// ──────────────────────────────────────────────────────────────────────────
// 事件载荷(spec 00 §4.3)
// ──────────────────────────────────────────────────────────────────────────

/** `selection://text-changed` */
export interface SelectionTextChangedPayload {
  text: string;
  lang?: string;
}

/** `selection://detected` —— 监听到划词/双击选中,坐标为光标处全局逻辑坐标 */
export type SelectionDetectedPayload = Omit<SelectionDetected, "x" | "y"> & {
  x: number;
  y: number;
};

/** `selection://permission-required` —— 缺少辅助功能权限 */
export type PermissionRequiredPayload = PermissionRequired;

/** `capture://region-selected` */
export type CaptureRegionSelectedPayload = RegionSelectedPayload;

/** `overlay://closed` */
export type OverlayClosedPayload = import("./bindings").OverlayClosed;

// ── 截图翻译 F2 事件载荷(spec 06)──────────────────────────────────────────

/** 各阶段耗时(毫秒) */
export type ScreenshotCost = RustScreenshotCost;

/** `screenshot://result` */
export type ScreenshotResult = RustScreenshotResult;

/** `screenshot://error` —— stage: capture/ocr/translate/timeout */
export type ScreenshotErrorPayload = Omit<ScreenshotError_Serialize, "stage"> & {
  stage: "capture" | "ocr" | "translate" | "timeout";
};

/** `screenshot://progress` */
export type ScreenshotProgressPayload = Omit<ScreenshotProgress, "stage"> & {
  stage: "capture" | "ocr" | "translate";
};

/** `screenshot://translating` —— 进入翻译阶段(原文已知、译文未出),供 popup 显示完整布局 + 引擎行转圈 */
export type ScreenshotTranslatingPayload = ScreenshotTranslating;

/** `config://changed` —— 变更的 keypath 列表(如 ["shortcuts.selection"]) */
export type ConfigChangedPayload = RustConfigChangedPayload;

/** `app://startup-warning` */
export type StartupWarningPayload = StartupWarning;

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

export type OcrBlock = Omit<RustOcrBlock, "confidence"> & { confidence: number };
export type OcrResult = Omit<RustOcrResult, "blocks"> & { blocks: OcrBlock[] };

export interface OcrOptions {
  preferredLanguages: string[];
  fastMode: boolean;
  minConfidence: number;
}

/** 截图采集结果;png 经 IPC 序列化为字节数组 */
export type CaptureResult = Omit<RustCaptureResult, "scaleFactor"> & { scaleFactor: number };
