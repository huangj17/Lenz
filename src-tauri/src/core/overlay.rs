//! 原图覆盖层(spec 07)。M1 仅骨架。
//!
//! 运行时创建透明、无边框、置顶、跳过任务栏、鼠标穿透的 overlay 窗口,
//! 在 bbox 位置绝对定位渲染译文;支持多屏多实例。

use crate::error::LenzError;

/// 覆盖层管理器抽象。
// TODO(M6): 实现 show / close、颜色采样(前景色)、字号自适应、临时 ESC 快捷键。
pub trait OverlayManager: Send + Sync {
    /// 关闭所有覆盖层。
    fn close_all(&self) -> Result<(), LenzError>;
}
