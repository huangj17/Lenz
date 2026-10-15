//! 配置版本迁移(spec 01 §7)。
//!
//! `migrate` 按 `version` 字段逐级提升。新增 schema 版本时,在此处添加
//! `migrate_vN_to_vN+1` 并串联调用,同时补充单元测试(旧 fixture → 迁移后等价性)。

use serde_json::Value;

use super::schema::{default_engines, default_ocr_services, CURRENT_VERSION, SYSTEM_OCR_ID};

/// 将任意旧版本的原始 JSON 配置迁移到当前版本。
///
/// 输入是已解析为 `serde_json::Value` 的原始配置(可能缺字段或为旧结构),
/// 返回迁移到 `CURRENT_VERSION` 后的 `Value`,再由调用方反序列化为 `Config`。
///
/// # 参数
/// - `raw`:原始配置 JSON。
/// - `from_version`:原始配置的版本号。
pub fn migrate(mut raw: Value, from_version: u32) -> Value {
    let mut version = from_version;

    // v1 → v2:注入内置默认引擎预设(spec 04 §12)。
    if version < 2 {
        raw = migrate_v1_to_v2(raw);
        version = 2;
    }

    // v2 → v3:重放历史迁移(追加百度翻译预设 + 曾默认开启 Google)。
    if version < 3 {
        raw = migrate_v2_to_v3(raw);
        version = 3;
    }

    // v3 → v4:移除内置 Google Key 后,关闭所有未配置凭据或尚未实现的内置引擎。
    if version < 4 {
        raw = migrate_v3_to_v4(raw);
    }
    if from_version < 5 {
        if let Some(ocr) = raw.get_mut("ocr").and_then(Value::as_object_mut) {
            ocr.entry("fastMode").or_insert(Value::Bool(false));
            ocr.entry("autoDetectLanguage").or_insert(Value::Bool(true));
        }
    }
    if from_version < 6 {
        if let Some(ocr) = raw.get_mut("ocr").and_then(Value::as_object_mut) {
            ocr.remove("engine");
            ocr.entry("defaultServiceId")
                .or_insert(Value::from(SYSTEM_OCR_ID));
            ocr.entry("services").or_insert_with(|| {
                serde_json::to_value(default_ocr_services()).expect("serializable OCR defaults")
            });
        }
    }

    if from_version < 7 {
        if let Some(popup) = raw.pointer_mut("/ui/popup").and_then(Value::as_object_mut) {
            popup
                .entry("services")
                .or_insert_with(|| serde_json::json!({}));
        }
    }

    // 兜底:确保 version 字段为当前版本。
    if let Value::Object(ref mut map) = raw {
        map.insert("version".into(), Value::from(CURRENT_VERSION));
    }
    raw
}

/// v1 → v2:旧配置 `translator.engines` 为空时,补齐内置默认引擎与默认引擎 id。
///
/// 仅在引擎列表为空时注入,避免覆盖用户已添加的引擎。
fn migrate_v1_to_v2(mut raw: Value) -> Value {
    let Some(translator) = raw.get_mut("translator").and_then(Value::as_object_mut) else {
        return raw;
    };

    let engines_empty = translator
        .get("engines")
        .and_then(Value::as_array)
        .map(|a| a.is_empty())
        .unwrap_or(true);
    if !engines_empty {
        return raw;
    }

    if let Ok(engines) = serde_json::to_value(default_engines()) {
        translator.insert("engines".into(), engines);
    }
    let default_empty = translator
        .get("defaultEngineId")
        .and_then(Value::as_str)
        .map(str::is_empty)
        .unwrap_or(true);
    if default_empty {
        translator.insert("defaultEngineId".into(), Value::from("builtin-youdao"));
    }

    raw
}

/// v2 → v3 的历史迁移:追加 `builtin-baidu`,并曾开启未填 Key 的内置 Google。
/// 后续 v3 → v4 会立即撤销不安全的无凭据启用状态。
///
/// 只做增量补丁,不动用户已改的引擎;`builtin-baidu` 已存在则跳过。
fn migrate_v2_to_v3(mut raw: Value) -> Value {
    let Some(engines) = raw
        .get_mut("translator")
        .and_then(Value::as_object_mut)
        .and_then(|t| t.get_mut("engines"))
        .and_then(Value::as_array_mut)
    else {
        return raw;
    };

    // 谷歌:仍是内置且未填 Key → 默认开启。
    for engine in engines.iter_mut() {
        let is_google = engine.get("id").and_then(Value::as_str) == Some("builtin-google");
        let key_empty = engine
            .get("apiKey")
            .and_then(Value::as_str)
            .map(str::is_empty)
            .unwrap_or(true);
        if is_google && key_empty {
            if let Value::Object(map) = engine {
                map.insert("enabled".into(), Value::Bool(true));
            }
        }
    }

    // 追加百度预设(仅当尚不存在)。
    let has_baidu = engines
        .iter()
        .any(|e| e.get("id").and_then(Value::as_str) == Some("builtin-baidu"));
    if !has_baidu {
        if let Some(baidu) = default_engines()
            .into_iter()
            .find(|e| e.id == "builtin-baidu")
        {
            if let Ok(v) = serde_json::to_value(baidu) {
                engines.push(v);
            }
        }
    }

    raw
}

/// v3 → v4:旧版本曾默认开启若干没有凭据的在线引擎,并把未实现的 Apple 引擎
/// 加入运行态。仅修正内置预设;用户自建的无鉴权本地端点保持原样。
fn migrate_v3_to_v4(mut raw: Value) -> Value {
    let Some(translator) = raw.get_mut("translator").and_then(Value::as_object_mut) else {
        return raw;
    };
    let default_id = translator
        .get("defaultEngineId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(engines) = translator.get_mut("engines").and_then(Value::as_array_mut) else {
        return raw;
    };

    for engine in engines.iter_mut() {
        let id = engine.get("id").and_then(Value::as_str).unwrap_or_default();
        let key_empty = engine
            .get("apiKey")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .is_empty();
        let unavailable = id == "builtin-apple"
            || (key_empty
                && matches!(
                    id,
                    "builtin-youdao"
                        | "builtin-google"
                        | "builtin-zhipu"
                        | "builtin-siliconflow"
                        | "builtin-baidu"
                        | "builtin-microsoft"
                ));
        if unavailable {
            if let Value::Object(map) = engine {
                map.insert("enabled".into(), Value::Bool(false));
            }
        }
    }

    let default_enabled = engines.iter().any(|engine| {
        engine.get("id").and_then(Value::as_str) == Some(default_id.as_str())
            && engine.get("enabled").and_then(Value::as_bool) == Some(true)
    });
    if !default_enabled {
        let fallback = engines
            .iter()
            .find(|engine| engine.get("enabled").and_then(Value::as_bool) == Some(true))
            .and_then(|engine| engine.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        translator.insert("defaultEngineId".into(), Value::String(fallback));
    }

    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v6_adds_empty_popup_policies_without_resetting_existing_settings() {
        let mut raw = serde_json::to_value(crate::config::Config::default()).unwrap();
        raw["version"] = serde_json::json!(6);
        raw["ui"]["popup"]
            .as_object_mut()
            .unwrap()
            .remove("services");
        raw["ui"]["popup"]["width"] = serde_json::json!(520);
        raw["translator"]["engines"][0]["apiKey"] = serde_json::json!("fixture-only");
        let before = raw.clone();
        let out = migrate(raw, 6);
        assert_eq!(out["version"], CURRENT_VERSION);
        assert_eq!(out["ui"]["popup"]["services"], serde_json::json!({}));
        assert_eq!(out["ui"]["popup"]["width"], 520);
        assert_eq!(out["translator"], before["translator"]);
        assert_eq!(out["ocr"], before["ocr"]);
        let config: crate::config::Config = serde_json::from_value(out).unwrap();
        assert!(config.ui.popup.services.is_empty());
    }

    #[test]
    fn popup_policy_migration_keeps_service_values_and_defaults_missing_fields() {
        let mut raw = serde_json::to_value(crate::config::Config::default()).unwrap();
        raw["ui"]["popup"]["services"] = serde_json::json!({
            "a": { "rememberCollapse": true, "collapsed": true, "skipWhenCollapsed": true },
            "b": { "skipWhenCollapsed": true }
        });
        let out = migrate(raw, 6);
        let config: crate::config::Config = serde_json::from_value(out).unwrap();
        assert!(config.ui.popup.services["a"].skip_initial_translation());
        assert!(!config.ui.popup.services["b"].collapsed);
        assert!(!config.ui.popup.services["b"].remember_collapse);
        assert!(config.ui.popup.services["b"].skip_when_collapsed);
        let restored: crate::config::Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.ui.popup.services, config.ui.popup.services);
    }

    #[test]
    fn v5_migrates_to_system_service_without_losing_preferences_or_translation_credentials() {
        let mut raw = serde_json::to_value(crate::config::Config::default()).unwrap();
        raw["version"] = serde_json::json!(5);
        raw["ocr"] = serde_json::json!({"engine":"system","preferredLanguages":["ja","en-US"],"minConfidence":0.7,"fastMode":true,"autoDetectLanguage":false});
        raw["translator"]["engines"][0]["apiKey"] = serde_json::json!("fixture-only");
        let out = migrate(raw, 5);
        let config: crate::config::Config = serde_json::from_value(out.clone()).unwrap();
        crate::config::schema::validate_ocr_config(&config.ocr).unwrap();
        assert!(out["ocr"].get("engine").is_none());
        assert_eq!(config.ocr.default_service_id, SYSTEM_OCR_ID);
        assert_eq!(config.ocr.services.len(), 1);
        assert_eq!(config.ocr.preferred_languages, ["ja", "en-US"]);
        assert_eq!(config.ocr.min_confidence, 0.7);
        assert!(config.ocr.fast_mode);
        assert!(!config.ocr.auto_detect_language);
        assert_eq!(config.translator.engines[0].api_key, "fixture-only");
    }

    #[test]
    fn v4_adds_ocr_preferences_without_resetting_user_values() {
        let raw = serde_json::json!({
            "version": 4,
            "ocr": { "engine": "system", "preferredLanguages": ["ja", "en-US"], "minConfidence": 0.7 },
            "translator": { "defaultEngineId": "local", "engines": [{ "id": "local", "enabled": true }] }
        });
        let out = migrate(raw, 4);
        assert_eq!(out["version"], CURRENT_VERSION);
        assert_eq!(out["ocr"]["fastMode"], false);
        assert_eq!(out["ocr"]["autoDetectLanguage"], true);
        assert_eq!(
            out["ocr"]["preferredLanguages"],
            serde_json::json!(["ja", "en-US"])
        );
        assert_eq!(out["ocr"]["minConfidence"], 0.7);
        assert_eq!(out["translator"]["engines"][0]["enabled"], true);
    }

    #[test]
    fn migration_preserves_explicit_ocr_preferences() {
        let raw = serde_json::json!({ "version": 4, "ocr": { "fastMode": true, "autoDetectLanguage": false } });
        let out = migrate(raw, 4);
        assert_eq!(out["ocr"]["fastMode"], true);
        assert_eq!(out["ocr"]["autoDetectLanguage"], false);
    }

    #[test]
    fn v1_seeds_builtin_engines() {
        let raw = serde_json::json!({
            "version": 1,
            "translator": { "defaultEngineId": "", "targetLang": "zh-CN", "engines": [] }
        });
        let out = migrate(raw, 1);
        let engines = out["translator"]["engines"].as_array().unwrap();
        assert_eq!(engines.len(), 9);
        assert_eq!(out["translator"]["defaultEngineId"], "");
        assert_eq!(out["version"], CURRENT_VERSION);
    }

    #[test]
    fn v2_appends_baidu_then_v4_disables_keyless_google() {
        let raw = serde_json::json!({
            "version": 2,
            "translator": {
                "defaultEngineId": "builtin-youdao",
                "targetLang": "zh-CN",
                "engines": [
                    { "id": "builtin-google", "enabled": false, "apiKey": "" },
                    { "id": "builtin-youdao", "enabled": true, "apiKey": "" }
                ]
            }
        });
        let out = migrate(raw, 2);
        let engines = out["translator"]["engines"].as_array().unwrap();
        // 追加了 builtin-baidu。
        assert!(engines.iter().any(|e| e["id"] == "builtin-baidu"));
        // v3 曾开启未填 Key 的 Google;完整迁移到 v4 后必须重新关闭。
        let google = engines
            .iter()
            .find(|e| e["id"] == "builtin-google")
            .unwrap();
        assert_eq!(google["enabled"], false);
        assert_eq!(out["version"], CURRENT_VERSION);
    }

    #[test]
    fn v2_keeps_user_google_key_disabled() {
        // 用户填过谷歌 Key 但停用:迁移不得强行开启。
        let raw = serde_json::json!({
            "version": 2,
            "translator": {
                "defaultEngineId": "builtin-youdao",
                "targetLang": "zh-CN",
                "engines": [
                    { "id": "builtin-google", "enabled": false, "apiKey": "my-key" }
                ]
            }
        });
        let out = migrate(raw, 2);
        let google = out["translator"]["engines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == "builtin-google")
            .unwrap()
            .clone();
        assert_eq!(google["enabled"], false);
    }

    #[test]
    fn v1_keeps_user_engines() {
        let raw = serde_json::json!({
            "version": 1,
            "translator": {
                "defaultEngineId": "mine",
                "targetLang": "zh-CN",
                "engines": [{ "id": "mine", "enabled": true }]
            }
        });
        let out = migrate(raw, 1);
        let engines = out["translator"]["engines"].as_array().unwrap();
        // v1→v2 不覆盖用户引擎;v2→v3 追加 builtin-baidu,故为「mine + baidu」。
        assert_eq!(engines.len(), 2);
        assert!(engines.iter().any(|e| e["id"] == "mine"));
        assert!(engines.iter().any(|e| e["id"] == "builtin-baidu"));
        assert_eq!(out["translator"]["defaultEngineId"], "mine");
    }

    #[test]
    fn v3_disables_unconfigured_builtins_and_repairs_default() {
        let raw = serde_json::json!({
            "version": 3,
            "translator": {
                "defaultEngineId": "builtin-youdao",
                "targetLang": "zh-CN",
                "engines": [
                    { "id": "builtin-youdao", "enabled": true, "apiKey": "" },
                    { "id": "builtin-apple", "enabled": true, "apiKey": "" },
                    { "id": "local", "enabled": true, "apiKey": "" }
                ]
            }
        });
        let out = migrate(raw, 3);
        let engines = out["translator"]["engines"].as_array().unwrap();
        assert_eq!(engines[0]["enabled"], false);
        assert_eq!(engines[1]["enabled"], false);
        assert_eq!(engines[2]["enabled"], true);
        assert_eq!(out["translator"]["defaultEngineId"], "local");
        assert_eq!(out["version"], CURRENT_VERSION);
    }
}
