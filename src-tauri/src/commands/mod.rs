//! 前端可调用的 Tauri commands 入口(spec 00 §4)。
//!
//! 各 command 仅做参数校验与转发,业务逻辑落在 `core/` 与 `config/`。

pub mod capture;
pub mod config;
pub mod ocr;
pub mod overlay;
pub mod translate;
