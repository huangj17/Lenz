//! 翻译抽象层(spec 04)。M1 仅 trait 骨架。
//!
//! `TranslatorManager` 维护共享 `reqwest::Client` 与默认引擎 id,支持热更新
//! (订阅 `config://changed` 触发 reload)。

use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

/// 翻译器抽象(spec 04 §1)。
// TODO(M3): 实现 OpenAI 兼容、Google、DeepL、有道等 Translator。
#[allow(async_fn_in_trait)]
pub trait Translator: Send + Sync {
    /// 执行一次(批量)翻译。
    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError>;
}

// TODO(M3): TranslatorManager { client: reqwest::Client, default_id: String, ... }
//           + reload(configs, proxy) 热更新 + build_http_client(proxy)。
