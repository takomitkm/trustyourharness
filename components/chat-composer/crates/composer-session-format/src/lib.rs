//! composer-session-format：持久化模型（四层模型的 L2）。
//!
//! 本 crate 的 Schema **不依赖 composer-core**（四层规格 §3）：持久化类型独立定义、
//! 独立版本、独立校验；Domain ↔ Persisted 的转换（codec/migration）由上层 Adapter 在
//! Phase B 提供，本 crate 只定义"存成什么形状"。
//!
//! 版本语义（§3.1/§9）：`format_version` 是持久化格式版本——不是 Bridge 协议版本，
//! 不是 Domain 文档版本，也不是编辑器文档版本。四个版本线分开管理。
//!
//! Blob 纪律（§3.4）：附件记录只保留 `blob_hash` 引用；真实字节在独立 BlobStore，
//! 永不进入 Scope JSON。

pub mod content;
pub mod envelope;
pub mod scope;

pub use envelope::{PersistedEnvelopeV1, PERSISTED_FORMAT, PERSISTED_FORMAT_VERSION_V1};
pub use scope::{
    PersistedDeliveryResultV1, PersistedDeliveryV1, PersistedDraftV1, PersistedReceiptV1,
    PersistedScopeV1, PersistedStatusV1, PersistedSubmissionV1,
};
pub use content::{
    PersistedAssetAvailabilityV1, PersistedAssetV1, PersistedBlockV1, PersistedComposerContentV1,
    PersistedDocumentV1, PersistedInlineV1, PersistedReferenceV1, PersistedTextMarkV1,
};

/// 校验失败（结构/引用/扩展数据问题，聚合报告）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub problems: Vec<String>,
}

/// 扩展 JSON 的最大嵌套深度（与 core 内容校验同款防御上限）。
pub const MAX_EXTENSION_DEPTH: usize = 32;

pub(crate) fn check_extension(value: &serde_json::Value, problems: &mut Vec<String>, where_: &str) {
    if extension_depth(value, 0) > MAX_EXTENSION_DEPTH {
        problems.push(format!("{where_}: 扩展 JSON 嵌套过深"));
    }
    if contains_object_url(value) {
        problems.push(format!("{where_}: 扩展数据含 object/blob URL（禁止持久化）"));
    }
}

fn extension_depth(value: &serde_json::Value, depth: usize) -> usize {
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .map(|item| extension_depth(item, depth + 1))
            .max()
            .unwrap_or(depth),
        serde_json::Value::Object(map) => map
            .values()
            .map(|item| extension_depth(item, depth + 1))
            .max()
            .unwrap_or(depth),
        _ => depth,
    }
}

fn contains_object_url(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(s) => {
            let s = s.trim();
            s.starts_with("blob:") || s.starts_with("object-url:")
        }
        serde_json::Value::Array(items) => items.iter().any(contains_object_url),
        serde_json::Value::Object(map) => map.values().any(contains_object_url),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_url_is_caught_nested() {
        let mut problems = Vec::new();
        check_extension(
            &serde_json::json!({ "a": [{ "src": "blob:https://x/y" }] }),
            &mut problems,
            "test",
        );
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn plain_extension_passes() {
        let mut problems = Vec::new();
        check_extension(&serde_json::json!({ "rows": 2, "tags": ["x"] }), &mut problems, "t");
        assert!(problems.is_empty());
    }
}
