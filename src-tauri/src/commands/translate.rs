//! 翻译相关 Tauri commands(spec 04 §8)。

use std::time::Instant;

use tauri::State;

use crate::config::{ConfigStore, TranslatorEngine};
use crate::core::translator::{build_test_engine, TranslatorManager};
use crate::error::LenzError;
use crate::types::{TestEngineResult, TranslateRequest, TranslateResponse};

/// 单条翻译(`texts` 通常长度 1)。路由到目标 / 默认引擎。
#[tauri::command]
#[specta::specta]
pub async fn translate_text(
    req: TranslateRequest,
    mgr: State<'_, TranslatorManager>,
) -> Result<TranslateResponse, LenzError> {
    mgr.translate(req).await
}

/// 批量翻译。与 `translate_text` 同路径,引擎内部决定单请求批量或逐条。
#[tauri::command]
#[specta::specta]
pub async fn translate_batch(
    req: TranslateRequest,
    mgr: State<'_, TranslatorManager>,
) -> Result<TranslateResponse, LenzError> {
    mgr.translate(req).await
}

/// 测试引擎连通性:对传入(可能尚未保存的)引擎翻译 "Hello",返回延迟 / 结果。
///
/// 代理取自当前已保存配置;失败时 `ok=false` 并带错误说明(spec 04 §8)。
#[tauri::command]
#[specta::specta]
pub async fn test_engine(
    engine: TranslatorEngine,
    config: State<'_, ConfigStore>,
) -> Result<TestEngineResult, LenzError> {
    let cfg = config.get()?;
    let translator = build_test_engine(engine, &cfg.proxy);

    let target = if cfg.translator.target_lang.is_empty() {
        "zh-CN".to_string()
    } else {
        cfg.translator.target_lang.clone()
    };
    let req = TranslateRequest {
        texts: vec!["Hello".into()],
        source_lang: Some("en-US".into()),
        target_lang: target,
        engine_id: None,
    };

    let started = Instant::now();
    match translator.translate(req).await {
        Ok(resp) => Ok(TestEngineResult {
            ok: true,
            latency_ms: started.elapsed().as_millis() as u32,
            sample: resp.translations.into_iter().next().unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(TestEngineResult {
            ok: false,
            latency_ms: started.elapsed().as_millis() as u32,
            sample: String::new(),
            error: Some(e.to_string()),
        }),
    }
}
