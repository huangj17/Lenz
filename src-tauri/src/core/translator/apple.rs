//! 苹果系统翻译引擎(spec 04 §11)。
//!
//! macOS 的 `Translation` 框架没有干净的无界面公开 API(`TranslationSession`
//! 仅能经 SwiftUI 获取,通常需隐藏宿主窗口 + 按需下载语言包),故此处先占位:
//! 引擎可被配置 / 选中,但调用返回明确的「暂未实现」错误,待后续接入原生桥接。

use async_trait::async_trait;

use super::Translator;
use crate::config::schema::{EngineKind, TranslatorEngine};
use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

pub struct AppleTranslator {
    cfg: TranslatorEngine,
}

impl AppleTranslator {
    /// 占位实现不需要 HTTP client。
    pub fn new(cfg: TranslatorEngine, _client: reqwest::Client) -> Self {
        AppleTranslator { cfg }
    }
}

#[async_trait]
impl Translator for AppleTranslator {
    fn id(&self) -> &str {
        &self.cfg.id
    }
    fn name(&self) -> &str {
        &self.cfg.name
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Apple
    }
    fn supports_batch(&self) -> bool {
        false
    }

    async fn translate(&self, _req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        Err(LenzError::TranslateFailed("apple_not_implemented".into()))
    }
}
