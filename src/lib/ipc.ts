/**
 * 对 `@tauri-apps/api` 的薄封装:
 *  - 类型安全的 invoke / emit / listen wrapper(camelCase 调用名映射到 snake_case 命令);
 *  - 统一错误拦截:把 Tauri 抛出的 reject 归一化为 {@link LenzError},
 *    再按 `code` 分发副作用(Permission→引导、Network→可重试提示、其它→toast/log)。
 *
 * 参考:docs/spec/00-architecture.md §4、§6;docs/spec/01-config-storage.md §5
 *
 * 后端命令在 M1 多为占位,本封装只负责把链路搭好。真实数据后续接入。
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  emit as tauriEmit,
  listen as tauriListen,
  type EventCallback,
  type UnlistenFn,
} from "@tauri-apps/api/event";
import type {
  SystemInfo, SystemResource, UpdateInfo, OverlayPayload,
  CaptureRegionSelectedPayload,
  CaptureResult,
  Config,
  ConfigChangedPayload,
  DeepPartial,
  LenzError,
  LenzErrorCode,
  OcrResult,
  OverlayClosedPayload,
  PermissionRequiredPayload,
  Rect,
  ScreenshotErrorPayload,
  ScreenshotProgressPayload,
  ScreenshotResult,
  ScreenshotTranslatingPayload,
  SelectionDetectedPayload,
  SelectionTextChangedPayload,
  StartupWarningPayload,
  TestEngineResult,
  TranslateRequest,
  TranslateResponse,
  TranslatorEngine,
  TriggerKind,
} from "./types";

// ──────────────────────────────────────────────────────────────────────────
// 错误归一化与分发
// ──────────────────────────────────────────────────────────────────────────

const ERROR_CODES: readonly LenzErrorCode[] = [
  "Config",
  "OcrUnavailable",
  "OcrIdentityRequired",
  "OcrFailed",
  "TranslateFailed",
  "Network",
  "Timeout",
  "Cancelled",
  "Permission",
  "Internal",
];

/** 运行时判定一个未知值是否为 LenzError 形态 */
export function isLenzError(value: unknown): value is LenzError {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.code === "string" &&
    (ERROR_CODES as readonly string[]).includes(v.code) &&
    typeof v.message === "string"
  );
}

/** 把任意 reject 值归一化为 LenzError */
export function toLenzError(value: unknown): LenzError {
  if (isLenzError(value)) return value;
  if (value instanceof Error) {
    return { code: "Internal", message: value.message };
  }
  if (typeof value === "string") {
    return { code: "Internal", message: value };
  }
  return { code: "Internal", message: "Unknown error" };
}

/** 错误处理器签名;返回 true 表示已消费(阻止默认分发) */
export type LenzErrorHandler = (err: LenzError) => boolean | void;

const handlers = new Set<LenzErrorHandler>();

/**
 * 注册全局错误处理器(如挂接 toast / 权限引导弹窗)。
 * UI 层在启动时注册;返回解绑函数。
 */
export function onLenzError(handler: LenzErrorHandler): () => void {
  handlers.add(handler);
  return () => handlers.delete(handler);
}

/**
 * 默认错误分发:在没有处理器消费时执行。
 *  - Permission → 引导用户去系统设置授权;
 *  - Network → 可重试提示;
 *  - 其它 → toast / log。
 * M1 暂以 console 占位,UI 就绪后由 {@link onLenzError} 接管。
 */
function dispatchError(err: LenzError): void {
  if (err.code === "Cancelled") return;
  for (const handler of handlers) {
    if (handler(err) === true) return;
  }
  switch (err.code) {
    case "Permission":
      // TODO(M4/M5): 弹出"打开系统设置"引导
      console.warn(`[Lenz][Permission] ${err.message}`);
      break;
    case "Network":
      // TODO: toast + 重试入口
      console.warn(`[Lenz][Network] ${err.message}`);
      break;
    default:
      console.error(`[Lenz][${err.code}] ${err.message}`);
  }
}

// ──────────────────────────────────────────────────────────────────────────
// invoke / emit / listen 薄封装
// ──────────────────────────────────────────────────────────────────────────

/**
 * 类型安全的 invoke。捕获 reject,归一化为 LenzError 后分发,再继续 reject,
 * 让调用方仍可 try/catch 做局部处理(已分发的全局副作用不重复)。
 */
export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args);
  } catch (raw) {
    const err = toLenzError(raw);
    dispatchError(err);
    throw err;
  }
}

/** 透传 emit(本封装不改写载荷) */
export async function emit(event: string, payload?: unknown): Promise<void> {
  return tauriEmit(event, payload);
}

/** 类型安全的 listen,返回解绑函数 */
export async function listen<T>(
  event: string,
  handler: EventCallback<T>,
): Promise<UnlistenFn> {
  return tauriListen<T>(event, handler);
}

// ──────────────────────────────────────────────────────────────────────────
// 命令清单(camelCase wrapper → snake_case 命令)
// ──────────────────────────────────────────────────────────────────────────

/** 配置相关命令(spec 01 §5) */
export const configApi = {
  get: () => invoke<Config>("get_config"),
  /**
   * JSON Merge Patch(RFC 7396)。
   * 线缆层传 JSON 字符串(后端 parse):patch 可含 null 删字段、任意深度,
   * 且后端 specta 无法安全导出 serde_json::Value,故在此 stringify。
   */
  patch: (patch: DeepPartial<Config>) =>
    invoke<Config>("set_config", { patch: JSON.stringify(patch) }),
  reset: () => invoke<Config>("reset_config"),
};

/** 翻译引擎相关命令(spec 01/04) */
export const engineApi = {
  list: () => invoke<TranslatorEngine[]>("list_engines"),
  test: (engine: TranslatorEngine) =>
    invoke<TestEngineResult>("test_engine", { engine }),
};

/** 翻译命令(spec 04) */
export const translateApi = {
  beginPopupSession: () => invoke<number>("begin_popup_session"),
  popup: (req: TranslateRequest, sessionId: number) =>
    invoke<TranslateResponse>("translate_popup", { req, sessionId }),
  text: (req: TranslateRequest) =>
    invoke<TranslateResponse>("translate_text", { req }),
  batch: (req: TranslateRequest) =>
    invoke<TranslateResponse>("translate_batch", { req }),
};

/** 采集 / 选区命令(spec 02) */
export const captureApi = {
  region: (rect: Rect) =>
    invoke<CaptureResult>("capture_screen_region", { rect }),
  startSelection: (trigger: TriggerKind) =>
    invoke<void>("start_selection", { trigger }),
  cancelSelection: () => invoke<void>("cancel_selection"),
};

/** 截图翻译 F2 命令(spec 06) */
export const screenshotApi = {
  /** 取消进行中的会话并隐藏 popup(ESC / 关闭) */
  cancel: () => invoke<void>("cancel_screenshot"),
  /** 打开系统「屏幕录制」设置(权限引导) */
  openScreenSettings: () => invoke<void>("open_screen_settings"),
  /** 内容渲染后按实际高度调整 popup 窗口(macOS 同时刷新窗口阴影,消除残影) */
  resizePopup: (height: number) => invoke<void>("resize_popup", { height }),
  /** 拖拽右缘实时调节 popup 宽度(夹紧 300~720;持久化由前端在拖拽结束时 patch config) */
  resizePopupWidth: (width: number) => invoke<void>("resize_popup_width", { width }),
};

/** OCR 命令(spec 03):截图区域 → 识别 */
export const ocrApi = {
  run: (rect: Rect) => invoke<OcrResult>("run_ocr", { rect }),
};

/** 覆盖层命令(spec 07) */
export const overlayApi = {
  show: (payload: OverlayPayload) => invoke<void>("show_overlay", { payload }),
  close: () => invoke<void>("close_overlay"),
  payload: () => invoke<OverlayPayload | null>("get_overlay_payload"),
  ready: (generation: number) => invoke<void>("overlay_ready", { generation }),
};

/** 划词监听(spec 05) */
export const selectionApi = {
  toggleListener: (enabled: boolean) =>
    invoke<void>("toggle_selection_listener", { enabled }),
  /** 划词指示图标被 hover/点击 → 合成复制 + 翻译并弹框 */
  trigger: () => invoke<void>("trigger_selection_translate"),
  /** 打开系统「辅助功能」设置(权限引导) */
  openAccessibilitySettings: () =>
    invoke<void>("open_accessibility_settings"),
};

/** 系统 / 更新命令(spec 10) */
export const systemApi = {
  setAutoStart: (enabled: boolean) => invoke<void>("set_auto_start", { enabled }),
  checkUpdate: () => invoke<UpdateInfo>("check_for_update"),
  info: () => invoke<SystemInfo>("get_system_info"),
  updateState: () => invoke<UpdateInfo | null>("get_update_state"),
  open: (resource: SystemResource) => invoke<void>("open_system_resource", { resource }),
  licenses: () => invoke<string>("get_license_notices"),
};

// ──────────────────────────────────────────────────────────────────────────
// 事件名常量与类型化订阅
// ──────────────────────────────────────────────────────────────────────────

export const Events = {
  ShortcutOpenMain: "shortcut://open-main",
  ShortcutSelection: "shortcut://selection",
  ShortcutScreenshot: "shortcut://screenshot",
  ShortcutOverlay: "shortcut://overlay",
  SelectionTextChanged: "selection://text-changed",
  SelectionDetected: "selection://detected",
  SelectionPermissionRequired: "selection://permission-required",
  CaptureRegionSelected: "capture://region-selected",
  CaptureRegionCancelled: "capture://region-cancelled",
  ScreenshotProgress: "screenshot://progress",
  ScreenshotTranslating: "screenshot://translating",
  ScreenshotResult: "screenshot://result",
  ScreenshotError: "screenshot://error",
  OverlayClosed: "overlay://closed",
  ConfigChanged: "config://changed",
  StartupWarning: "app://startup-warning",
} as const;

/** 事件名 → 载荷类型映射,供类型化订阅 */
export interface EventPayloadMap {
  [Events.ShortcutOpenMain]: void;
  [Events.ShortcutSelection]: void;
  [Events.ShortcutScreenshot]: void;
  [Events.ShortcutOverlay]: void;
  [Events.SelectionTextChanged]: SelectionTextChangedPayload;
  [Events.SelectionDetected]: SelectionDetectedPayload;
  [Events.SelectionPermissionRequired]: PermissionRequiredPayload;
  [Events.CaptureRegionSelected]: CaptureRegionSelectedPayload;
  [Events.CaptureRegionCancelled]: void;
  [Events.ScreenshotProgress]: ScreenshotProgressPayload;
  [Events.ScreenshotTranslating]: ScreenshotTranslatingPayload;
  [Events.ScreenshotResult]: ScreenshotResult;
  [Events.ScreenshotError]: ScreenshotErrorPayload;
  [Events.OverlayClosed]: OverlayClosedPayload;
  [Events.ConfigChanged]: ConfigChangedPayload;
  [Events.StartupWarning]: StartupWarningPayload;
}

/** 类型化事件订阅;事件名受 EventPayloadMap 约束 */
export function listenEvent<K extends keyof EventPayloadMap>(
  event: K,
  handler: EventCallback<EventPayloadMap[K]>,
): Promise<UnlistenFn> {
  return listen<EventPayloadMap[K]>(event, handler);
}
