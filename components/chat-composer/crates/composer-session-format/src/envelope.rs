//! 持久化信封（规格 §3.1）：外层固定 format + format_version，payload 才是 Scope。
//! 解码方先识别信封再进 payload——不支持版本明确报错，不静默猜。

use crate::scope::PersistedScopeV1;
use serde::{Deserialize, Serialize};

/// `format` 固定值：composer 作用域持久化的自描述标记。
pub const PERSISTED_FORMAT: &str = "composer-scope";

/// V1：首个独立持久化格式（此前 ScopeState 直接 serde 的形状是 legacy，迁移见 Phase B）。
pub const PERSISTED_FORMAT_VERSION_V1: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedEnvelopeV1 {
    pub format: String,
    pub format_version: u32,
    pub payload: PersistedScopeV1,
}

impl PersistedEnvelopeV1 {
    pub fn new(payload: PersistedScopeV1) -> Self {
        PersistedEnvelopeV1 {
            format: PERSISTED_FORMAT.to_string(),
            format_version: PERSISTED_FORMAT_VERSION_V1,
            payload,
        }
    }

    /// 识别 + 版本闸门：非本格式或不支持的版本明确报错（规格 §3.5）。
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let envelope: PersistedEnvelopeV1 = serde_json::from_slice(bytes)
            .map_err(|error| format!("持久化信封解析失败: {error}"))?;
        if envelope.format != PERSISTED_FORMAT {
            return Err(format!(
                "未知持久化格式 '{}'（期望 '{PERSISTED_FORMAT}'）",
                envelope.format
            ));
        }
        if envelope.format_version != PERSISTED_FORMAT_VERSION_V1 {
            return Err(format!(
                "不支持的持久化格式版本 {}（本实现支持 {PERSISTED_FORMAT_VERSION_V1}）",
                envelope.format_version
            ));
        }
        Ok(envelope)
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|error| format!("持久化信封编码失败: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::PersistedComposerContentV1;
    use crate::scope::{PersistedDraftV1, PersistedScopeV1};

    fn sample_scope() -> PersistedScopeV1 {
        PersistedScopeV1 {
            scope_id: "scope".into(),
            store_revision: 7,
            draft: PersistedDraftV1 {
                generation_id: "gen-1".into(),
                draft_revision: 3,
                content: PersistedComposerContentV1::plain_text("草稿"),
            },
            submissions: Vec::new(),
            active_turn_id: None,
            finished_turn_ids: vec!["turn-1".into()],
            pending_receipts: Vec::new(),
            created_at_ms: 1,
            updated_at_ms: 2,
            extensions: serde_json::Map::new(),
        }
    }

    #[test]
    fn envelope_roundtrip_and_format_gate() {
        let envelope = PersistedEnvelopeV1::new(sample_scope());
        let bytes = envelope.encode().unwrap();
        let back = PersistedEnvelopeV1::decode(&bytes).unwrap();
        assert_eq!(back, envelope);
        assert_eq!(back.format, "composer-scope");
        assert_eq!(back.format_version, 1);
    }

    #[test]
    fn unknown_format_is_rejected_explicitly() {
        let mut envelope = PersistedEnvelopeV1::new(sample_scope());
        envelope.format = "something-else".into();
        let bytes = envelope.encode().unwrap();
        let error = PersistedEnvelopeV1::decode(&bytes).unwrap_err();
        assert!(error.contains("未知持久化格式"), "{error}");
    }

    #[test]
    fn unsupported_version_is_rejected_explicitly() {
        let mut envelope = PersistedEnvelopeV1::new(sample_scope());
        envelope.format_version = 99;
        let bytes = envelope.encode().unwrap();
        let error = PersistedEnvelopeV1::decode(&bytes).unwrap_err();
        assert!(error.contains("不支持的持久化格式版本"), "{error}");
    }

    #[test]
    fn corrupt_json_is_rejected_and_does_not_suggest_overwrite() {
        let error = PersistedEnvelopeV1::decode(b"{ not json").unwrap_err();
        assert!(error.contains("解析失败"), "{error}");
    }
}
