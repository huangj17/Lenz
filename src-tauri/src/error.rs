//! 统一错误枚举 `LenzError`(spec 00 §5、§6)。
//!
//! 序列化采用 `tag = "code", content = "message"`,前端 `lib/ipc.ts` 据此按
//! `code` 分发处理(Permission→引导授权、Network→重试、其他→toast)。

use serde::{Deserialize, Serialize};
use specta::Type;

/// Lenz 全局错误类型。所有 Tauri command 统一返回 `Result<T, LenzError>`。
#[derive(Debug, Clone, Serialize, Deserialize, Type, thiserror::Error)]
#[serde(tag = "code", content = "message")]
pub enum LenzError {
    /// 配置读写 / 解析错误。
    #[error("config error: {0}")]
    Config(String),
    /// OCR 引擎不可用(如缺少系统语言包、无权限)。
    #[error("ocr unavailable: {0}")]
    OcrUnavailable(String),
    /// Windows OCR 需要安装包身份,不能用语言包错误替代。
    #[error("ocr package identity required: {0}")]
    OcrIdentityRequired(String),
    /// OCR 执行失败。
    #[error("ocr failed: {0}")]
    OcrFailed(String),
    /// 翻译失败(鉴权、解析、服务端错误等)。
    #[error("translate failed: {0}")]
    TranslateFailed(String),
    #[error("cancelled: {0}")]
    Cancelled(String),
    #[error("timeout: {0}")]
    Timeout(String),
    /// 网络错误(连接失败等)。
    #[error("network error: {0}")]
    Network(String),
    /// 权限缺失(如 macOS 屏幕录制 / 辅助功能未授权)。
    #[error("permission denied: {0}")]
    Permission(String),
    /// 其他内部错误。
    #[error("internal error: {0}")]
    Internal(String),
}

impl LenzError {
    /// 便捷构造内部错误。
    pub fn internal(msg: impl Into<String>) -> Self {
        LenzError::Internal(msg.into())
    }

    /// 便捷构造配置错误。
    pub fn config(msg: impl Into<String>) -> Self {
        LenzError::Config(msg.into())
    }

    /// 占位:尚未在当前里程碑实现的功能。
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub fn not_implemented() -> Self {
        LenzError::Internal("not_implemented".into())
    }
}

impl From<std::io::Error> for LenzError {
    fn from(e: std::io::Error) -> Self {
        LenzError::Config(format!("io: {e}"))
    }
}

impl From<serde_json::Error> for LenzError {
    fn from(e: serde_json::Error) -> Self {
        LenzError::Config(format!("json: {e}"))
    }
}

impl From<tauri::Error> for LenzError {
    fn from(e: tauri::Error) -> Self {
        LenzError::Internal(format!("tauri: {e}"))
    }
}
