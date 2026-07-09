//! 配置版本迁移(spec 01 §7)。
//!
//! `migrate` 按 `version` 字段逐级提升。当前只有 v1,故为占位实现:
//! 未来新增 schema 版本时,在此处添加 `migrate_vN_to_vN+1` 并串联调用,
//! 同时补充单元测试(旧 fixture → 迁移后等价性)。

use serde_json::Value;

use super::schema::{default_engines, CURRENT_VERSION};

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

    // v2 → v3:追加百度翻译预设 + 谷歌默认开启(内置 Key)。
    if version < 3 {
        raw = migrate_v2_to_v3(raw);
        version = 3;
    }
    let _ = version;

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

/// v2 → v3:为老配置追加 `builtin-baidu` 预设;谷歌若仍是内置且未填 Key
/// (即未被用户改动),则默认开启,享受内置默认 Key 开箱即用。
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
        if let Some(baidu) = default_engines().into_iter().find(|e| e.id == "builtin-baidu") {
            if let Ok(v) = serde_json::to_value(baidu) {
                engines.push(v);
            }
        }
    }

    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_seeds_builtin_engines() {
        let raw = serde_json::json!({
            "version": 1,
            "translator": { "defaultEngineId": "", "targetLang": "zh-CN", "engines": [] }
        });
        let out = migrate(raw, 1);
        let engines = out["translator"]["engines"].as_array().unwrap();
        assert_eq!(engines.len(), 9);
        assert_eq!(out["translator"]["defaultEngineId"], "builtin-youdao");
        assert_eq!(out["version"], CURRENT_VERSION);
    }

    #[test]
    fn v2_appends_baidu_and_enables_google() {
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
        // 谷歌(未填 Key)默认开启。
        let google = engines.iter().find(|e| e["id"] == "builtin-google").unwrap();
        assert_eq!(google["enabled"], true);
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
                "engines": [{ "id": "mine" }]
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
}
