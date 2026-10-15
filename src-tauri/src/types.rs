//! 跨模块共享类型(spec 00 §5)。
//!
//! 线缆格式统一 camelCase(serde `rename_all = "camelCase"`),并尽量 derive
//! `specta::Type` 以便通过 tauri-specta 导出到前端 `src/lib/bindings.ts`。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 屏幕矩形区域,单位为逻辑像素。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// 触发翻译的来源种类。
///
/// 序列化为 "selection" / "screenshot" / "overlay"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TriggerKind {
    /// 划词翻译(F1)。
    Selection,
    /// 截图翻译(F2)。
    Screenshot,
    /// 原图覆盖翻译(F3)。
    Overlay,
}

/// 翻译请求(spec 00 §5)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranslateRequest {
    /// 待翻译的文本片段(批量)。
    pub texts: Vec<String>,
    /// 源语言;`None` 表示自动检测。
    pub source_lang: Option<String>,
    /// 目标语言(BCP-47)。
    pub target_lang: String,
    /// 指定翻译引擎 id;`None` 表示使用默认引擎。
    pub engine_id: Option<String>,
}

/// 翻译响应(spec 00 §5)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResponse {
    /// 与请求 `texts` 一一对应的译文。
    pub translations: Vec<String>,
    /// 检测到的源语言(若有)。
    pub detected_source_lang: Option<String>,
    /// 实际使用的引擎 id。
    pub engine_id: String,
    /// 本次翻译耗时(毫秒)。
    pub cost_ms: u32,
}

/// `test_engine` 命令的返回结果(spec 01 §5)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TestEngineResult {
    pub ok: bool,
    pub latency_ms: u32,
    pub sample: String,
    pub error: Option<String>,
}

/// `config://changed` 事件载荷:携带变更的 keypath 列表。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangedPayload {
    /// 形如 `["shortcuts.selection"]` 的点分路径列表。
    pub keypaths: Vec<String>,
}

/// `app://startup-warning` 事件载荷(spec 00 §7)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StartupWarning {
    /// 失败的启动步骤标识,如 "shortcuts"、"tray"。
    pub step: String,
    /// 人类可读的失败说明。
    pub message: String,
}

// ──────────────────────────────────────────────────────────────────────────
// 截图翻译 F2 事件载荷(spec 02 §3.3 / spec 06)
// ──────────────────────────────────────────────────────────────────────────

/// 前端 selection 窗口框选完成后回传(`capture://region-selected`,spec 02 §3.3)。
/// 坐标为虚拟屏逻辑坐标。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegionSelectedPayload {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// 选区起点所在显示器 id(可选)。
    #[serde(default)]
    pub display_id: Option<i32>,
    /// 触发来源(screenshot / overlay)。
    pub trigger: TriggerKind,
}

/// 各阶段耗时(毫秒),随 `screenshot://result` 上报(spec 06 §2.6)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotCost {
    pub capture: u32,
    pub ocr: u32,
    pub translate: u32,
}

/// `screenshot://result` 事件载荷(spec 06 §2.6)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotResult {
    #[serde(default)]
    pub session_id: u32,
    /// OCR 拼接原文。
    pub original: String,
    /// 译文。
    pub translation: String,
    pub source_lang: Option<String>,
    pub target_lang: String,
    pub engine_id: String,
    /// 选区逻辑坐标(供 popup 定位参考)。
    pub region: Rect,
    pub cost: ScreenshotCost,
}

/// `screenshot://translating` 事件载荷:进入翻译阶段、原文已知但译文未出时先发一份,
/// 让 popup 立即渲染「原文 + 语言条 + 引擎行(右侧转圈)」的完整布局,而非整块骨架
/// (spec 06 §2.6 / spec 09 §3)。`sourceLang` 此刻可能未检测(None)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotTranslating {
    #[serde(default)]
    pub session_id: u32,
    pub original: String,
    pub source_lang: Option<String>,
    pub target_lang: String,
    pub engine_id: String,
    /// 原生主服务请求标识,供弹窗单独取消;无自动请求时为 None。
    pub request_id: Option<String>,
}

/// `screenshot://error` 事件载荷(spec 06 §4)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotError {
    #[serde(default)]
    pub session_id: u32,
    /// 失败阶段:`capture` / `ocr` / `translate` / `timeout`。
    pub stage: String,
    /// 细分码:`permission` / `no_text` / `failed` / `timeout`。
    pub code: String,
    pub message: String,
    /// 翻译阶段失败时附带 OCR 原文,供 UI 展示原文 + 重试(spec 06 §4)。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
}

/// `screenshot://progress` 事件载荷(spec 06 §5)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotProgress {
    #[serde(default)]
    pub session_id: u32,
    /// 当前阶段:`capture` / `ocr` / `translate`。
    pub stage: String,
}

// ──────────────────────────────────────────────────────────────────────────
// 划词监听 F1 事件载荷(spec 05 §2 / §5)
// ──────────────────────────────────────────────────────────────────────────

/// `selection://detected` 事件载荷:监听到一次「划词/双击选中」手势。
/// 坐标为光标松开处的全局显示逻辑坐标(top-left 原点,单位 point),供前端在该处
/// 浮出划词指示图标(spec 05 §2.1 / §7)。此时**尚未**读取选中文本——只有用户
/// hover/点击图标才会去复制并翻译,避免无谓地改动剪贴板。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SelectionDetected {
    pub x: f64,
    pub y: f64,
}

/// `selection://permission-required` 事件载荷(spec 05 §2.1)。
/// macOS 划词监听依赖「辅助功能(Accessibility)」权限;未授时 emit,前端引导用户
/// 去系统设置开启。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequired {
    /// 平台标识:`macos` / `windows`。
    pub platform: String,
    /// 人类可读说明。
    pub detail: String,
}
