//! Provider codes stay inside adapters; IPC always uses BCP-47 language tags.
use crate::error::LenzError;

pub const BAIDU: &[(&str, &str)] = &[
    ("zh-CN", "zh"),
    ("zh-TW", "cht"),
    ("en", "en"),
    ("ja", "jp"),
    ("ko", "kor"),
    ("fr", "fra"),
    ("de", "de"),
    ("es", "spa"),
    ("it", "it"),
    ("pt", "pt"),
    ("ru", "ru"),
    ("ar", "ara"),
    ("th", "th"),
    ("vi", "vie"),
];
pub const YOUDAO: &[(&str, &str)] = &[
    ("zh-CN", "zh-CHS"),
    ("zh-TW", "zh-CHT"),
    ("en", "en"),
    ("ja", "ja"),
    ("ko", "ko"),
    ("fr", "fr"),
    ("de", "de"),
    ("es", "es"),
    ("ru", "ru"),
];

pub fn to_provider(code: &str, table: &'static [(&str, &str)]) -> Result<&'static str, LenzError> {
    let lower = code.trim().to_ascii_lowercase();
    if let Some((_, provider)) = table
        .iter()
        .find(|(_, provider)| provider.eq_ignore_ascii_case(&lower))
    {
        return Ok(provider);
    }
    let canonical = match lower.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "zh-chs" => "zh-CN",
        "zh-tw" | "zh-hant" | "zh-hk" | "zh-cht" => "zh-TW",
        other => other.split('-').next().unwrap_or(other),
    };
    table
        .iter()
        .find(|(bcp, _)| bcp.eq_ignore_ascii_case(canonical))
        .map(|(_, provider)| *provider)
        .ok_or_else(|| LenzError::TranslateFailed(format!("unsupported language: {code}")))
}

pub fn from_provider(code: &str, table: &[(&str, &str)]) -> Option<String> {
    table
        .iter()
        .find(|(_, provider)| provider.eq_ignore_ascii_case(code.trim()))
        .map(|(bcp, _)| bcp.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detected_language_survives_retranslation() {
        for table in [BAIDU, YOUDAO] {
            for (bcp, native) in table {
                let detected = from_provider(native, table).unwrap();
                assert_eq!(detected, *bcp);
                assert_eq!(to_provider(&detected, table).unwrap(), *native);
            }
        }
        assert_eq!(to_provider("ja-JP", BAIDU).unwrap(), "jp");
        assert_eq!(to_provider("zh-Hant", YOUDAO).unwrap(), "zh-CHT");
    }
    #[test]
    fn unsupported_language_is_never_silently_english() {
        assert!(to_provider("unknown", BAIDU).is_err());
        assert!(to_provider("it", YOUDAO).is_err());
        assert_eq!(from_provider("unknown", BAIDU), None);
    }
}
