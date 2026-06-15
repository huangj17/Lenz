//! 屏幕位图采集(spec 02)。M1 仅骨架。
//!
//! 计划用 `xcap` 包装:完整屏幕位图、单屏抓取、逻辑坐标→物理像素裁剪、跨屏拼接。

use crate::error::LenzError;
use crate::types::Rect;

/// 采集结果:PNG 字节、物理像素矩形、缩放因子(spec 02 §4)。
// TODO(M2): 落地为完整结构体并接入共享类型导出。
pub struct CaptureResult {
    pub png: Vec<u8>,
    pub pixel_rect: Rect,
    pub scale_factor: f64,
}

/// 屏幕采集器抽象。
pub trait Capturer: Send + Sync {
    /// 采集指定逻辑区域。
    fn capture_region(&self, rect: Rect) -> Result<CaptureResult, LenzError>;
}

// TODO(M2): 实现基于 xcap 的 Capturer;macOS 无屏幕录制权限时返回 LenzError::Permission。
