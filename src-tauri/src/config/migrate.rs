//! 配置版本迁移(spec 01 §7)。
//!
//! `migrate` 按 `version` 字段逐级提升。当前只有 v1,故为占位实现:
//! 未来新增 schema 版本时,在此处添加 `migrate_vN_to_vN+1` 并串联调用,
//! 同时补充单元测试(旧 fixture → 迁移后等价性)。

use serde_json::Value;

use super::schema::CURRENT_VERSION;

/// 将任意旧版本的原始 JSON 配置迁移到当前版本。
///
/// 输入是已解析为 `serde_json::Value` 的原始配置(可能缺字段或为旧结构),
/// 返回迁移到 `CURRENT_VERSION` 后的 `Value`,再由调用方反序列化为 `Config`。
///
/// # 参数
/// - `raw`:原始配置 JSON。
/// - `from_version`:原始配置的版本号。
pub fn migrate(mut raw: Value, from_version: u32) -> Value {
    // TODO(M+): 每次 schema 升级在此追加一级迁移分支,例如:
    // let mut version = from_version;
    // if version == 1 { raw = migrate_v1_to_v2(raw); version = 2; }
    let _ = from_version;

    // 兜底:确保 version 字段为当前版本。
    if let Value::Object(ref mut map) = raw {
        map.insert("version".into(), Value::from(CURRENT_VERSION));
    }
    raw
}
