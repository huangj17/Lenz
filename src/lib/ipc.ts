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
  CaptureRegionSelectedPayload,
  Config,
  ConfigChangedPayload,
  DeepPartial,
  LenzError,
  LenzErrorCode,
  OverlayClosedPayload,
  Rect,
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
  "OcrFailed",
  "TranslateFailed",
  "Network",
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
  text: (req: TranslateRequest) =>
    invoke<TranslateResponse>("translate_text", { req }),
  batch: (req: TranslateRequest) =>
    invoke<TranslateResponse>("translate_batch", { req }),
};

/** 采集 / 选区命令(spec 02) */
export const captureApi = {
  region: (rect: Rect) =>
    invoke<unknown>("capture_screen_region", { rect }),
  startSelection: (trigger: TriggerKind) =>
    invoke<void>("start_selection", { trigger }),
  cancelSelection: () => invoke<void>("cancel_selection"),
};

/** OCR 命令(spec 03) */
export const ocrApi = {
  run: (rect: Rect) => invoke<unknown>("run_ocr", { rect }),
};

/** 覆盖层命令(spec 07) */
export const overlayApi = {
  show: (payload: unknown) => invoke<void>("show_overlay", { payload }),
  close: (id: string) => invoke<void>("close_overlay", { id }),
};

/** 划词监听开关(spec 05) */
export const selectionApi = {
  toggleListener: (enabled: boolean) =>
    invoke<void>("toggle_selection_listener", { enabled }),
};

/** 系统 / 更新命令(spec 10) */
export const systemApi = {
  checkUpdate: () => invoke<unknown>("check_update"),
};

// ──────────────────────────────────────────────────────────────────────────
// 事件名常量与类型化订阅
// ──────────────────────────────────────────────────────────────────────────

export const Events = {
  ShortcutSelection: "shortcut://selection",
  ShortcutScreenshot: "shortcut://screenshot",
  ShortcutOverlay: "shortcut://overlay",
  SelectionTextChanged: "selection://text-changed",
  CaptureRegionSelected: "capture://region-selected",
  OverlayClosed: "overlay://closed",
  ConfigChanged: "config://changed",
  StartupWarning: "app://startup-warning",
} as const;

/** 事件名 → 载荷类型映射,供类型化订阅 */
export interface EventPayloadMap {
  [Events.ShortcutSelection]: void;
  [Events.ShortcutScreenshot]: void;
  [Events.ShortcutOverlay]: void;
  [Events.SelectionTextChanged]: SelectionTextChangedPayload;
  [Events.CaptureRegionSelected]: CaptureRegionSelectedPayload;
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
