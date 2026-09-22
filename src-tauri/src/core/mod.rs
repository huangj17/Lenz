//! 业务核心层(spec 00 §2)。
//!
//! 沿 "采集 → 识别 → 翻译 → 展示" 主干切分,各层通过 trait 抽象,平台差异收拢
//! 于此。M1 仅放置 trait 定义与空骨架,具体实现见后续里程碑。
//!
//! 注:除 `shortcut`(M1 即用)外,以下子模块的 trait / 类型均为后续里程碑
//! 预留接口(spec 00 §8 验收标准:trait 存在但无实现),故整体允许 dead_code。
#![allow(dead_code)]

pub mod capture;
pub mod cursor;
pub mod indicator;
pub mod keystroke;
pub mod ocr;
pub mod overlay;
pub mod screenshot;
pub mod selection;
pub mod selection_text;
pub mod selection_window;
pub mod shortcut;
pub mod translator;

pub mod clipboard_selection;
