//! 百度翻译开放平台「通用翻译 API」引擎(spec 04 §13)。
//!
//! 不支持单请求批量(为保证「译文条数 == 原文条数」不变量,不用 `\n` 拼接批量),
//! `supports_batch = false`,内部逐条串行。凭据在配置里以 `appid:secret`
//! 形式存于 `apiKey`(与有道 `appKey:appSecret` 同构)。
//!
//! 签名(参见 <https://fanyi-api.baidu.com/doc/21>):
//! `sign = md5(appid + q + salt + 密钥)`,32 位小写十六进制,`q` 用未编码原文。

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use md5::{Digest, Md5};
use serde::Deserialize;

use super::Translator;
use crate::config::schema::{EngineKind, TranslatorEngine};
use crate::error::LenzError;
use crate::types::{TranslateRequest, TranslateResponse};

const DEFAULT_ENDPOINT: &str = "https://fanyi-api.baidu.com/api/trans/vip/translate";

pub struct BaiduTranslator {
    cfg: TranslatorEngine,
    client: reqwest::Client,
}

impl BaiduTranslator {
    pub fn new(cfg: TranslatorEngine, client: reqwest::Client) -> Self {
        BaiduTranslator { cfg, client }
    }

    /// 拆分 `appid:secret`。
    fn credentials(&self) -> Result<(&str, &str), LenzError> {
        self.cfg
            .api_key
            .split_once(':')
            .filter(|(id, secret)| !id.is_empty() && !secret.is_empty())
            .ok_or_else(|| LenzError::Config("baidu_credentials".into()))
    }

    async fn translate_one(
        &self,
        text: &str,
        appid: &str,
        secret: &str,
        from: &str,
        to: &str,
    ) -> Result<BaiduResp, LenzError> {
        // salt 用毫秒时间戳即可(官方仅要求随机整数)。
        let salt = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
            .to_string();
        let sign = sign(appid, text, &salt, secret);

        let endpoint = if self.cfg.api_url.trim().is_empty() {
            DEFAULT_ENDPOINT
        } else {
            self.cfg.api_url.trim()
        };

        let params = [
            ("q", text),
            ("from", from),
            ("to", to),
            ("appid", appid),
            ("salt", salt.as_str()),
            ("sign", sign.as_str()),
        ];

        let rb = self
            .client
            .post(endpoint)
            .timeout(Duration::from_millis(self.cfg.timeout_ms.max(1) as u64))
            .form(&params);

        super::send_json(rb).await
    }
}

#[async_trait]
impl Translator for BaiduTranslator {
    fn id(&self) -> &str {
        &self.cfg.id
    }
    fn name(&self) -> &str {
        &self.cfg.name
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Baidu
    }
    fn supports_batch(&self) -> bool {
        false
    }

    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError> {
        let started = Instant::now();
        let (appid, secret) = self.credentials()?;
        let from = req
            .source_lang
            .as_deref()
            .filter(|code| !code.eq_ignore_ascii_case("auto"))
            .map(baidu_lang_code)
            .transpose()?
            .unwrap_or("auto");
        let to = baidu_lang_code(&req.target_lang)?;

        let mut translations = Vec::with_capacity(req.texts.len());
        let mut detected: Option<String> = None;
        for text in &req.texts {
            let resp = self.translate_one(text, appid, secret, from, to).await?;
            if let Some(code) = resp.error_code.as_deref() {
                // "52000" 亦为成功码;其余非空 error_code 视为失败。
                if code != "0" && code != "52000" {
                    return Err(LenzError::TranslateFailed(format!("baidu_{code}")));
                }
            }
            if detected.is_none() {
                detected = resp.from.as_deref().and_then(|code| {
                    super::languages::from_provider(code, super::languages::BAIDU)
                });
            }
            // trans_result 逐段拼回(百度会按标点/换行切段);单条原文取全部 dst 拼接。
            let result = resp
                .trans_result
                .filter(|v| !v.is_empty())
                .ok_or_else(|| LenzError::TranslateFailed("empty_response".into()))?;
            let joined = result
                .into_iter()
                .map(|r| r.dst)
                .collect::<Vec<_>>()
                .join("\n");
            translations.push(joined);
        }

        Ok(TranslateResponse {
            translations,
            detected_source_lang: detected,
            engine_id: self.cfg.id.clone(),
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

/// 百度签名:`md5(appid + q + salt + secret)`,32 位小写十六进制。
fn sign(appid: &str, q: &str, salt: &str, secret: &str) -> String {
    let raw = format!("{appid}{q}{salt}{secret}");
    let digest = Md5::digest(raw.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// BCP-47 → 百度语言代码。百度用一批非标准码(日 `jp`、韩 `kor`、法 `fra`、西 `spa`、
/// 阿 `ara`、繁体 `cht`),需单独映射。
fn baidu_lang_code(code: &str) -> Result<&'static str, LenzError> {
    super::languages::to_provider(code, super::languages::BAIDU)
}

#[derive(Deserialize)]
struct BaiduResp {
    /// 成功响应无此字段;失败时形如 "54001"。
    error_code: Option<String>,
    /// 检测到的源语言(百度码)。
    from: Option<String>,
    trans_result: Option<Vec<BaiduSeg>>,
}

#[derive(Deserialize)]
struct BaiduSeg {
    dst: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_matches_official_example() {
        // 百度官方文档示例:appid=2015063000000001, q=apple, salt=1435660288,
        // 密钥=12345678,sign 应为 f89f9594663708c1605f3d736d01d2d4。
        assert_eq!(
            sign("2015063000000001", "apple", "1435660288", "12345678"),
            "f89f9594663708c1605f3d736d01d2d4"
        );
    }

    #[test]
    fn sign_is_stable_32_hex() {
        let s = sign("appid", "hello", "123", "secret");
        assert_eq!(s.len(), 32);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn baidu_codes() {
        assert_eq!(baidu_lang_code("zh-CN").unwrap(), "zh");
        assert_eq!(baidu_lang_code("zh-Hant").unwrap(), "cht");
        assert_eq!(baidu_lang_code("ja").unwrap(), "jp");
        assert_eq!(baidu_lang_code("ko-KR").unwrap(), "kor");
        assert_eq!(baidu_lang_code("fr").unwrap(), "fra");
        assert_eq!(baidu_lang_code("es").unwrap(), "spa");
        assert_eq!(baidu_lang_code("en-US").unwrap(), "en");
    }
}
