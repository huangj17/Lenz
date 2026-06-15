//! 划词监听(spec 05)。M1 仅骨架。
//!
//! 平台实现:macOS `CGEventTap` + `NSPasteboard`,Windows `SetWinEventHook` +
//! 剪贴板读取。统一以 `SelectionListener` 抽象 start/stop/restart。

use crate::error::LenzError;

/// 划词监听器抽象(spec 05 §2.3)。
// TODO(M5): 实现平台监听 + 去抖 + 长度过滤 + 去重 + 权限检测引导。
pub trait SelectionListener: Send + Sync {
    /// 启动监听。
    fn start(&self) -> Result<(), LenzError>;
    /// 停止监听。
    fn stop(&self) -> Result<(), LenzError>;
    /// 重启监听(配置变更后)。
    fn restart(&self) -> Result<(), LenzError> {
        self.stop()?;
        self.start()
    }
}
