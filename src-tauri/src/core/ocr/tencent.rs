//! 腾讯云高精度 OCR。固定官方端点,按实际 JSON 字节签名,只输出脱敏错误。

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::remote::{http, send};
use super::{map_pixel_bbox, OcrBlock, OcrEngine, OcrInput, OcrOptions, OcrResult};
use crate::config::schema::OcrService;
use crate::error::LenzError;

const HOST: &str = "ocr.tencentcloudapi.com";
const ACTION: &str = "GeneralAccurateOCR";
const CONTENT_TYPE: &str = "application/json; charset=utf-8";
const SIGNED_HEADERS: &str = "content-type;host;x-tc-action";
// 文档限制的是 Base64 后的大小;使用十进制 10M,不对截图隐式缩放。
const MAX_BASE64_BYTES: usize = 10_000_000;

pub struct TencentOcr {
    config: OcrService,
    endpoint: String,
}

impl TencentOcr {
    pub fn new(config: OcrService) -> Result<Self, LenzError> {
        if config.api_key.trim().is_empty() || config.secret_key.trim().is_empty() {
            return Err(LenzError::Config("ocr_tencent_credentials".into()));
        }
        Ok(Self {
            config,
            endpoint: format!("https://{HOST}/"),
        })
    }
}

fn invalid_response() -> LenzError {
    LenzError::OcrFailed("ocr_invalid_response".into())
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts keys of any size");
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

fn authorization(config: &OcrService, timestamp: i64, body: &[u8]) -> Result<String, LenzError> {
    let date = DateTime::<Utc>::from_timestamp(timestamp, 0)
        .ok_or_else(|| LenzError::OcrFailed("ocr_tencent_clock".into()))?
        .format("%Y-%m-%d")
        .to_string();
    let canonical = format!(
        "POST\n/\n\ncontent-type:{CONTENT_TYPE}\nhost:{HOST}\nx-tc-action:{}\n\n{SIGNED_HEADERS}\n{:x}",
        ACTION.to_ascii_lowercase(),
        Sha256::digest(body)
    );
    let scope = format!("{date}/ocr/tc3_request");
    let to_sign = format!(
        "TC3-HMAC-SHA256\n{timestamp}\n{scope}\n{:x}",
        Sha256::digest(canonical.as_bytes())
    );
    let date_key = hmac_sha256(
        format!("TC3{}", config.secret_key.trim()).as_bytes(),
        date.as_bytes(),
    );
    let service_key = hmac_sha256(&date_key, b"ocr");
    let signing_key = hmac_sha256(&service_key, b"tc3_request");
    let signature: String = hmac_sha256(&signing_key, to_sign.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(format!(
        "TC3-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={SIGNED_HEADERS}, Signature={signature}",
        config.api_key.trim()
    ))
}

fn request_body(input: &OcrInput, config: &OcrService) -> Result<Vec<u8>, LenzError> {
    if input.png.is_empty() {
        return Err(LenzError::OcrFailed("ocr_tencent_empty_image".into()));
    }
    if input.png.len().div_ceil(3).saturating_mul(4) > MAX_BASE64_BYTES {
        return Err(LenzError::OcrFailed("ocr_tencent_image_too_large".into()));
    }
    serde_json::to_vec(&json!({
        "ImageBase64": STANDARD.encode(&input.png),
        "ConfigID": config.tencent_config_id,
        "EnableDetectText": true,
        "IsWords": false,
    }))
    .map_err(|_| invalid_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentEnvelope {
    response: TencentResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentResponse {
    error: Option<TencentError>,
    request_id: Option<String>,
    text_detections: Option<Vec<TencentDetection>>,
    angle: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentError {
    code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentDetection {
    detected_text: String,
    confidence: Option<f64>,
    polygon: Option<Vec<Point>>,
    item_polygon: Option<ItemRect>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn service_error(code: &str) -> Option<LenzError> {
    let key = match code {
        "FailedOperation.ImageNoText" => return None,
        "AuthFailure.SignatureExpire" => "ocr_tencent_signature_expired",
        "FailedOperation.UnOpenError" => "ocr_tencent_not_enabled",
        "ResourceUnavailable.InArrears"
        | "ResourceUnavailable.ResourcePackageRunOut"
        | "ResourcesSoldOut.ChargeStatusException" => "ocr_tencent_quota",
        "FailedOperation.ImageSizeTooLarge"
        | "LimitExceeded.TooLargeFileError"
        | "RequestSizeLimitExceeded" => "ocr_tencent_image_too_large",
        "FailedOperation.EmptyImageError" => "ocr_tencent_empty_image",
        "FailedOperation.EngineRecognizeTimeout" => {
            return Some(LenzError::Timeout("ocr_timeout".into()));
        }
        _ if code.starts_with("AuthFailure.") || code == "UnauthorizedOperation" => {
            "ocr_tencent_auth"
        }
        _ if code == "RequestLimitExceeded" || code.starts_with("RequestLimitExceeded.") => {
            "ocr_tencent_rate_limit"
        }
        _ => "ocr_tencent_failed",
    };
    Some(LenzError::OcrFailed(key.into()))
}

fn blocks(
    response: TencentResponse,
    input: &OcrInput,
    opts: &OcrOptions,
) -> Result<Vec<OcrBlock>, LenzError> {
    if let Some(id) = response.request_id.filter(|id| {
        !id.is_empty()
            && id.len() <= 64
            && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }) {
        log::debug!("Tencent OCR request_id={id}");
    }
    if let Some(error) = response.error {
        return match service_error(&error.code) {
            Some(error) => Err(error),
            None => Ok(Vec::new()),
        };
    }
    let detections = response.text_detections.ok_or_else(invalid_response)?;
    let mut blocks = Vec::new();
    for detection in detections {
        let confidence = detection.confidence.map(|c| c / 100.0);
        if confidence.is_some_and(|c| !c.is_finite() || !(0.0..=1.0).contains(&c)) {
            return Err(invalid_response());
        }
        if detection.detected_text.trim().is_empty()
            || confidence.is_some_and(|c| (c as f32) < opts.min_confidence)
        {
            continue;
        }
        let (left, top, right, bottom) = if let Some(points) = detection.polygon {
            if points.len() != 4 || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                return Err(invalid_response());
            }
            (
                points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
                points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
                points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
                points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
            )
        } else {
            // ItemPolygon 在旋转纠正后的图像上。仅确认没有旋转时才允许回退。
            if !response
                .angle
                .is_some_and(|a| a.is_finite() && a.rem_euclid(360.0).abs() < 0.001)
            {
                return Err(invalid_response());
            }
            let rect = detection.item_polygon.ok_or_else(invalid_response)?;
            if [rect.x, rect.y, rect.width, rect.height]
                .iter()
                .any(|n| !n.is_finite())
                || rect.width <= 0.0
                || rect.height <= 0.0
            {
                return Err(invalid_response());
            }
            (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height)
        };
        let left = left.max(0.0);
        let top = top.max(0.0);
        let right = right.min(input.pixel_rect.w as f64);
        let bottom = bottom.min(input.pixel_rect.h as f64);
        if right <= left || bottom <= top {
            continue;
        }
        blocks.push(OcrBlock {
            text: detection.detected_text,
            bbox: map_pixel_bbox(
                left,
                top,
                right - left,
                bottom - top,
                &input.pixel_rect,
                input.scale_factor,
            ),
            confidence: confidence.unwrap_or(1.0) as f32,
        });
    }
    Ok(blocks)
}

#[async_trait]
impl OcrEngine for TencentOcr {
    fn name(&self) -> &'static str {
        "tencent-ocr"
    }
    fn supported_languages(&self) -> Vec<String> {
        Vec::new()
    }
    fn recognition_timeout(&self, _: &OcrOptions) -> Duration {
        Duration::from_millis(self.config.timeout_ms as u64)
    }
    async fn recognize(&self, input: OcrInput, opts: OcrOptions) -> Result<OcrResult, LenzError> {
        let started = Instant::now();
        let body = request_body(&input, &self.config)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LenzError::OcrFailed("ocr_tencent_clock".into()))?
            .as_secs() as i64;
        let authorization = authorization(&self.config, timestamp, &body)?;
        let response: TencentEnvelope = send(
            http()?
                .post(&self.endpoint)
                .timeout(self.recognition_timeout(&opts))
                .header("Content-Type", CONTENT_TYPE)
                .header("Host", HOST)
                .header("X-TC-Action", ACTION)
                .header("X-TC-Version", "2018-11-19")
                .header("X-TC-Timestamp", timestamp.to_string())
                .header("Authorization", authorization)
                .body(body),
        )
        .await?;
        Ok(OcrResult {
            blocks: blocks(response.response, &input, &opts)?,
            text: None,
            // MulOCR 的语种逐行返回,混合图片不能当成单一源语言。
            detected_language: None,
            cost_ms: started.elapsed().as_millis() as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{default_ocr_services, OcrServiceKind, TencentOcrConfigId};
    use crate::types::Rect;
    use serde_json::Value;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn config() -> OcrService {
        let mut config = default_ocr_services().remove(0);
        config.kind = OcrServiceKind::Tencent;
        config.api_key = "AKIDEXAMPLE".into();
        config.secret_key = "fixture-secret".into();
        config
    }

    fn input() -> OcrInput {
        OcrInput {
            png: include_bytes!("../../../resources/OcrSmoke.png").to_vec(),
            pixel_rect: Rect {
                x: -200,
                y: 100,
                w: 1024,
                h: 220,
            },
            scale_factor: 2.0,
        }
    }

    fn opts() -> OcrOptions {
        OcrOptions {
            preferred_languages: Vec::new(),
            fast_mode: false,
            auto_detect_language: true,
            min_confidence: 0.3,
        }
    }

    fn parse(value: Value) -> Result<Vec<OcrBlock>, LenzError> {
        blocks(serde_json::from_value(value).unwrap(), &input(), &opts())
    }

    async fn server(
        status: u16,
        body: &str,
        delay: Duration,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let body = body.to_owned();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|n| n.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            tokio::time::sleep(delay).await;
            let reply = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = stream.write_all(reply.as_bytes()).await;
            String::from_utf8(bytes).unwrap()
        });
        (url, task)
    }

    #[test]
    fn tc3_matches_independent_python_hmac_fixture_and_uses_utc_date() {
        // 按腾讯官方规范,用 Python hashlib/hmac 独立生成的固定期望值。
        let body = br#"{"ImageBase64":"dGVzdA==","ConfigID":"OCR","EnableDetectText":true,"IsWords":false}"#;
        let signed = authorization(&config(), 1_551_113_065, body).unwrap();
        assert_eq!(signed, "TC3-HMAC-SHA256 Credential=AKIDEXAMPLE/2019-02-25/ocr/tc3_request, SignedHeaders=content-type;host;x-tc-action, Signature=403ccbdc061a9bbe1b6c149220ead20dab18fdadfd543e2b8643b1549f7b7a2c");
        assert_ne!(
            signed,
            authorization(&config(), 1_551_113_065, b"{}").unwrap()
        );
        assert!(authorization(&config(), 0, body)
            .unwrap()
            .contains("/1970-01-01/ocr/"));
    }

    #[tokio::test]
    async fn sends_signed_png_and_multilingual_scene_without_sending_secret_key() {
        let (url, task) = server(200, r#"{"Response":{"TextDetections":[{"DetectedText":"LENZ OCR TEST 12345","Confidence":98,"Polygon":[{"X":20,"Y":40},{"X":420,"Y":40},{"X":420,"Y":100},{"X":20,"Y":100}]}]}}"#, Duration::ZERO).await;
        let mut config = config();
        config.tencent_config_id = TencentOcrConfigId::MulOcr;
        let mut engine = TencentOcr::new(config.clone()).unwrap();
        engine.endpoint = url;
        let result = super::super::recognize_with_timeout(&engine, input(), opts())
            .await
            .unwrap();
        assert_eq!(result.blocks.len(), 1);
        let bbox = result.blocks[0].bbox;
        assert_eq!((bbox.x, bbox.y, bbox.w, bbox.h), (-90, 70, 200, 30));
        assert!((result.blocks[0].confidence - 0.98).abs() < 0.0001);
        assert!(result.text.is_none());
        let request = task.await.unwrap();
        assert!(request.starts_with("POST / HTTP/1.1"));
        assert!(!request.contains("fixture-secret"));
        let (headers, body) = request.split_once("\r\n\r\n").unwrap();
        let header = |name: &str| {
            headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case(name).then(|| value.trim())
                })
                .unwrap()
        };
        assert_eq!(header("host"), HOST);
        assert_eq!(header("content-type"), CONTENT_TYPE);
        assert_eq!(header("x-tc-action"), ACTION);
        assert_eq!(header("x-tc-version"), "2018-11-19");
        assert_eq!(
            header("authorization"),
            authorization(
                &config,
                header("x-tc-timestamp").parse().unwrap(),
                body.as_bytes()
            )
            .unwrap()
        );
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["ConfigID"], "MulOCR");
        assert_eq!(body["ImageBase64"], STANDARD.encode(input().png));
        assert_eq!(body["EnableDetectText"], true);
        assert_eq!(body["IsWords"], false);
        assert!(body.get("ImageUrl").is_none());
    }

    #[test]
    fn maps_original_polygon_even_when_deskewed_rect_differs_and_clips_edges() {
        let result = parse(json!({"Angle":12,"TextDetections":[
            {"DetectedText":"original","Confidence":90,"Polygon":[{"X":20,"Y":40},{"X":120,"Y":50},{"X":110,"Y":80},{"X":10,"Y":70}],"ItemPolygon":{"X":300,"Y":0,"Width":90,"Height":20}},
            {"DetectedText":"clipped","Polygon":[{"X":-10,"Y":200},{"X":1100,"Y":200},{"X":1100,"Y":250},{"X":-10,"Y":250}]},
            {"DetectedText":"low","Confidence":10},
            {"DetectedText":"threshold","Confidence":30,"Polygon":[{"X":0,"Y":0},{"X":100,"Y":0},{"X":100,"Y":20},{"X":0,"Y":20}]},
            {"DetectedText":" ","Confidence":100}
        ]})).unwrap();
        let bbox = result[0].bbox;
        assert_eq!((bbox.x, bbox.y, bbox.w, bbox.h), (-95, 70, 55, 20));
        let bbox = result[1].bbox;
        assert_eq!((bbox.x, bbox.y, bbox.w, bbox.h), (-100, 150, 512, 10));
        assert_eq!(result.len(), 3);
        assert_eq!(result[1].confidence, 1.0);
    }

    #[test]
    fn only_uses_item_polygon_without_rotation_and_rejects_malformed_results() {
        let rect = json!({"X":20,"Y":40,"Width":100,"Height":20});
        assert_eq!(parse(json!({"Angle":0,"TextDetections":[{"DetectedText":"ok","Polygon":null,"ItemPolygon":rect}]})).unwrap().len(), 1);
        for response in [
            json!({}),
            json!({"Angle":12,"TextDetections":[{"DetectedText":"rotated","ItemPolygon":rect}]}),
            json!({"TextDetections":[{"DetectedText":"unknown rotation","ItemPolygon":rect}]}),
            json!({"TextDetections":[{"DetectedText":"invalid","Confidence":101}]}),
            json!({"TextDetections":[{"DetectedText":"invalid","Polygon":[]}]}),
        ] {
            assert!(parse(response).is_err());
        }
        assert!(parse(json!({"TextDetections":[]})).unwrap().is_empty());
        assert!(
            parse(json!({"Error":{"Code":"FailedOperation.ImageNoText"}}))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn maps_service_errors_without_echoing_messages_or_arbitrary_codes() {
        for (code, expected) in [
            (
                "AuthFailure.SignatureExpire",
                "ocr_tencent_signature_expired",
            ),
            ("AuthFailure.SecretIdNotFound", "ocr_tencent_auth"),
            ("FailedOperation.UnOpenError", "ocr_tencent_not_enabled"),
            (
                "RequestLimitExceeded.UinLimitExceeded",
                "ocr_tencent_rate_limit",
            ),
            ("ResourceUnavailable.InArrears", "ocr_tencent_quota"),
            (
                "ResourceUnavailable.ResourcePackageRunOut",
                "ocr_tencent_quota",
            ),
            (
                "LimitExceeded.TooLargeFileError",
                "ocr_tencent_image_too_large",
            ),
            ("FailedOperation.EngineRecognizeTimeout", "ocr_timeout"),
            ("echoed-sensitive-code", "ocr_tencent_failed"),
        ] {
            let error = parse(json!({"Error":{"Code":code,"Message":"echoed-sensitive-body"},"RequestId":"echoed sensitive request id"})).unwrap_err();
            assert!(error.to_string().ends_with(expected));
            assert!(!error.to_string().contains("echoed"));
        }
    }

    #[test]
    fn validates_credentials_and_base64_size_before_network_request() {
        let mut config = config();
        config.api_key.clear();
        assert!(matches!(TencentOcr::new(config), Err(LenzError::Config(_))));
        let mut input = input();
        input.png.clear();
        assert!(request_body(&input, &self::config())
            .unwrap_err()
            .to_string()
            .contains("ocr_tencent_empty_image"));
        input.png.resize(7_500_000, 0);
        assert!(request_body(&input, &self::config()).is_ok());
        input.png.push(0);
        assert!(request_body(&input, &self::config())
            .unwrap_err()
            .to_string()
            .contains("ocr_tencent_image_too_large"));
    }

    #[tokio::test]
    async fn handles_http_json_business_errors_and_shared_timeout() {
        for (status, body, expected) in [
            (401, "echoed-sensitive-body", "ocr_http_401"),
            (200, "invalid-json", "ocr_invalid_response"),
            (
                200,
                r#"{"Response":{"Error":{"Code":"AuthFailure.SignatureFailure","Message":"echoed-sensitive-body"}}}"#,
                "ocr_tencent_auth",
            ),
        ] {
            let (url, task) = server(status, body, Duration::ZERO).await;
            let mut engine = TencentOcr::new(config()).unwrap();
            engine.endpoint = url;
            let error = engine.recognize(input(), opts()).await.unwrap_err();
            assert!(error.to_string().ends_with(expected));
            assert!(!error.to_string().contains("echoed"));
            task.await.unwrap();
        }
        let (url, task) = server(
            200,
            r#"{"Response":{"TextDetections":[]}}"#,
            Duration::from_millis(150),
        )
        .await;
        let mut engine = TencentOcr::new(config()).unwrap();
        engine.endpoint = url;
        engine.config.timeout_ms = 30;
        assert!(matches!(
            super::super::recognize_with_timeout(&engine, input(), opts()).await,
            Err(LenzError::Timeout(_))
        ));
        task.await.unwrap();
    }
}
