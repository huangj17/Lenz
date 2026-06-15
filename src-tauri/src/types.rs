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
