//! 配置存储(spec 01 §4)。
//!
//! `ConfigStore` 以 `RwLock<Config>` 持有内存态配置,并负责:
//! - 启动加载(容错:损坏则备份 + 重建,spec 01 §4.1)
//! - 原子写(tmp → rename,spec 01 §4.2)
//! - RFC 7396 JSON Merge Patch 的 `set_config`
//! - 变更后 emit `config://changed`(载荷含变更 keypath)

pub mod migrate;
pub mod schema;

use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::error::LenzError;
use crate::types::ConfigChangedPayload;

pub use schema::{Config, TranslatorEngine};

/// `config://changed` 事件名。
pub const EVENT_CONFIG_CHANGED: &str = "config://changed";
/// 配置文件解析失败时的提示事件名(spec 01 §4.1)。
pub const EVENT_CONFIG_CORRUPTED: &str = "config://corrupted";

const CONFIG_FILE: &str = "config.json";
const TMP_FILE: &str = "config.json.tmp";

/// 线程安全的配置存储,作为 Tauri managed state。
pub struct ConfigStore {
    inner: RwLock<Config>,
    /// 配置文件所在目录(app config dir)。
    dir: PathBuf,
}

impl ConfigStore {
    /// 从磁盘加载配置;不存在则写入默认配置(spec 01 §4.1)。
    ///
    /// 解析失败时:备份损坏文件为 `config.json.bak.<timestamp>`,写默认配置,
    /// 并 emit `config://corrupted`。
    pub fn load<R: Runtime>(app: &AppHandle<R>) -> Result<Self, LenzError> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|e| LenzError::config(format!("app_config_dir: {e}")))?;
        std::fs::create_dir_all(&dir)?;

        let path = dir.join(CONFIG_FILE);
        let config = if !path.exists() {
            let default = Config::default();
            write_atomic(&dir, &default)?;
            default
        } else {
            match std::fs::read_to_string(&path) {
                Ok(raw) => match parse_and_migrate(&raw) {
                    Ok((cfg, migrated)) => {
                        if migrated {
                            write_atomic(&dir, &cfg)?;
                        }
                        cfg
                    }
                    Err(parse_err) => {
                        // 损坏:备份 + 重建。
                        if let Err(backup_err) = backup_corrupted(&path) {
                            log::warn!("failed to backup corrupted config: {backup_err}");
                        }
                        log::warn!("config corrupted ({parse_err}), rebuilding default");
                        let default = Config::default();
                        write_atomic(&dir, &default)?;
                        let _ = app.emit(EVENT_CONFIG_CORRUPTED, ());
                        default
                    }
                },
                Err(io_err) => {
                    log::warn!("failed to read config ({io_err}), rebuilding default");
                    let default = Config::default();
                    write_atomic(&dir, &default)?;
                    default
                }
            }
        };

        Ok(ConfigStore {
            inner: RwLock::new(config),
            dir,
        })
    }

    /// 返回当前配置的克隆快照。
    pub fn get(&self) -> Result<Config, LenzError> {
        let guard = self
            .inner
            .read()
            .map_err(|_| LenzError::internal("config lock poisoned"))?;
        Ok(guard.clone())
    }

    /// 应用 RFC 7396 JSON Merge Patch,持久化并 emit `config://changed`。
    ///
    /// 返回合并后的最新配置。
    pub fn set<R: Runtime>(&self, app: &AppHandle<R>, patch: Value) -> Result<Config, LenzError> {
        let new_config = {
            let mut guard = self
                .inner
                .write()
                .map_err(|_| LenzError::internal("config lock poisoned"))?;

            // 1. 当前配置转 Value。
            let mut doc = serde_json::to_value(&*guard)?;
            // 2. 计算变更 keypath(在合并前 diff)。
            let keypaths = collect_changed_keypaths(&patch, "");
            // 3. 应用 merge patch(RFC 7396)。
            merge(&mut doc, &patch);
            // 4. 反序列化校验。
            let next: Config = serde_json::from_value(doc)
                .map_err(|e| LenzError::config(format!("invalid patch result: {e}")))?;
            // 5. 原子写盘。
            write_atomic(&self.dir, &next)?;
            // 6. 更新内存态。
            *guard = next.clone();

            // emit 在释放写锁后进行(此处先记录 keypaths)。
            (next, keypaths)
        };

        let (config, keypaths) = new_config;
        let _ = app.emit(EVENT_CONFIG_CHANGED, ConfigChangedPayload { keypaths });
        Ok(config)
    }

    /// 重置为默认配置,持久化并 emit `config://changed`。
    pub fn reset<R: Runtime>(&self, app: &AppHandle<R>) -> Result<Config, LenzError> {
        let default = Config::default();
        {
            let mut guard = self
                .inner
                .write()
                .map_err(|_| LenzError::internal("config lock poisoned"))?;
            write_atomic(&self.dir, &default)?;
            *guard = default.clone();
        }
        let _ = app.emit(
            EVENT_CONFIG_CHANGED,
            ConfigChangedPayload {
                keypaths: vec![String::new()], // 空 keypath 表示整体重置。
            },
        );
        Ok(default)
    }
}

/// 解析原始 JSON 并按需迁移。
fn parse_and_migrate(raw: &str) -> Result<(Config, bool), LenzError> {
    let value: Value = serde_json::from_str(raw)?;
    let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    let needs_migration = version < schema::CURRENT_VERSION;
    let migrated = if needs_migration {
        migrate::migrate(value, version)
    } else {
        value
    };

    let config: Config = serde_json::from_value(migrated)?;
    Ok((config, needs_migration))
}

/// 原子写:序列化到 `config.json.tmp` 后 `rename` 到 `config.json`(spec 01 §4.2)。
fn write_atomic(dir: &Path, config: &Config) -> Result<(), LenzError> {
    let tmp = dir.join(TMP_FILE);
    let dest = dir.join(CONFIG_FILE);
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(())
}

/// 备份损坏文件为 `config.json.bak.<timestamp>`。
fn backup_corrupted(path: &Path) -> Result<(), LenzError> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = path.with_extension(format!("json.bak.{ts}"));
    std::fs::rename(path, backup)?;
    Ok(())
}

/// RFC 7396 JSON Merge Patch:就地合并 `patch` 到 `target`。
///
/// - patch 中某 key 的值为 `null` → 从 target 中删除该 key。
/// - 双方均为对象 → 递归合并。
/// - 否则 → 用 patch 的值覆盖(数组整体替换)。
fn merge(target: &mut Value, patch: &Value) {
    match patch {
        Value::Object(patch_map) => {
            if !target.is_object() {
                *target = Value::Object(serde_json::Map::new());
            }
            let target_map = target.as_object_mut().expect("ensured object above");
            for (key, patch_val) in patch_map {
                if patch_val.is_null() {
                    target_map.remove(key);
                } else {
                    let entry = target_map.entry(key.clone()).or_insert(Value::Null);
                    merge(entry, patch_val);
                }
            }
        }
        _ => {
            *target = patch.clone();
        }
    }
}

/// 收集 patch 中涉及的叶子 keypath(点分),用于 `config://changed` 载荷。
///
/// 仅对对象递归;数组 / 标量视为叶子。
fn collect_changed_keypaths(patch: &Value, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    match patch {
        Value::Object(map) => {
            for (key, val) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                if val.is_object() {
                    out.extend(collect_changed_keypaths(val, &path));
                } else {
                    out.push(path);
                }
            }
        }
        _ => {
            if !prefix.is_empty() {
                out.push(prefix.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_overrides_scalar() {
        let mut target = serde_json::json!({"a": 1, "b": {"c": 2}});
        let patch = serde_json::json!({"a": 9});
        merge(&mut target, &patch);
        assert_eq!(target, serde_json::json!({"a": 9, "b": {"c": 2}}));
    }

    #[test]
    fn merge_recurses_objects() {
        let mut target = serde_json::json!({"b": {"c": 2, "d": 3}});
        let patch = serde_json::json!({"b": {"c": 20}});
        merge(&mut target, &patch);
        assert_eq!(target, serde_json::json!({"b": {"c": 20, "d": 3}}));
    }

    #[test]
    fn merge_null_removes_key() {
        let mut target = serde_json::json!({"a": 1, "b": 2});
        let patch = serde_json::json!({"b": null});
        merge(&mut target, &patch);
        assert_eq!(target, serde_json::json!({"a": 1}));
    }

    #[test]
    fn merge_replaces_array() {
        let mut target = serde_json::json!({"xs": [1, 2, 3]});
        let patch = serde_json::json!({"xs": [9]});
        merge(&mut target, &patch);
        assert_eq!(target, serde_json::json!({"xs": [9]}));
    }

    #[test]
    fn keypaths_are_dotted_leaves() {
        let patch =
            serde_json::json!({"shortcuts": {"selection": "Alt+D"}, "app": {"theme": "dark"}});
        let mut paths = collect_changed_keypaths(&patch, "");
        paths.sort();
        assert_eq!(paths, vec!["app.theme", "shortcuts.selection"]);
    }

    #[test]
    fn default_config_is_valid_roundtrip() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, schema::CURRENT_VERSION);
        assert_eq!(back.translator.target_lang, "zh-CN");
    }
}
