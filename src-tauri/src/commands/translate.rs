//! 翻译相关 Tauri commands(spec 04)。M1 仅占位。

use crate::config::TranslatorEngine;
use crate::error::LenzError;
use crate::types::{TestEngineResult, TranslateRequest, TranslateResponse};

/// 单条翻译。
// TODO(M3): 接入 TranslatorManager,实现 OpenAI 兼容 / 传统 API 翻译。
#[tauri::command]
#[specta::specta]
#[allow(unused_variables)]
pub async fn translate_text(req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
    Err(LenzError::not_implemented())
}

/// 批量翻译。
// TODO(M3): 同 translate_text,批量路径。
#[tauri::command]
#[specta::specta]
#[allow(unused_variables)]
pub async fn translate_batch(req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
    Err(LenzError::not_implemented())
}

/// 测试引擎连通性(对传入引擎发送样例文本,返回延迟 / 结果)。
///
/// 入参为完整引擎对象,以便测试尚未保存到配置的草稿引擎。
// TODO(M3): 实际发送 "Hello" 请求并测量延迟。
#[tauri::command]
#[specta::specta]
#[allow(unused_variables)]
pub async fn test_engine(engine: TranslatorEngine) -> Result<TestEngineResult, LenzError> {
    Err(LenzError::not_implemented())
}
