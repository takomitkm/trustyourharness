//! 内容半区的持久化形状（规格 §3.3）：独立结构、显式版本，禁止 `type X = ComposerContent`。
//! （本 crate 不依赖 composer-core，该禁令由不写那行代码保证。）

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 结构化文档的持久化形状；`version` 随节点保存（规格 §3.3：节点/资产/引用都带版本）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedDocumentV1 {
    pub version: u32,
    pub blocks: Vec<PersistedBlockV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PersistedBlockV1 {
    Paragraph {
        node_id: String,
        children: Vec<PersistedInlineV1>,
    },
    Heading {
        node_id: String,
        level: u8,
        children: Vec<PersistedInlineV1>,
    },
    Quote {
        node_id: String,
        children: Vec<PersistedBlockV1>,
    },
    Custom {
        node_id: String,
        namespace: String,
        node_type: String,
        version: u32,
        data: JsonValue,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PersistedInlineV1 {
    Text {
        node_id: String,
        text: String,
        marks: Vec<PersistedTextMarkV1>,
    },
    HardBreak {
        node_id: String,
    },
    Mention {
        node_id: String,
        reference_id: String,
        label: String,
    },
    Attachment {
        node_id: String,
        asset_id: String,
    },
    Custom {
        node_id: String,
        namespace: String,
        node_type: String,
        version: u32,
        data: JsonValue,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedTextMarkV1 {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
}

/// 附件记录：逻辑身份 + 元数据 + **blob_hash 引用**（§3.4，字节在 BlobStore）。
/// `version` 是资产记录自己的版本线。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedAssetV1 {
    pub id: String,
    pub blob_hash: Option<String>,
    pub name: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub alt: Option<String>,
    pub send_as_file: bool,
    pub availability: PersistedAssetAvailabilityV1,
    pub version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedAssetAvailabilityV1 {
    Available,
    Missing,
}

/// 引用记录：保留结构化 `value` 与自身 `version`。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedReferenceV1 {
    pub id: String,
    pub kind: String,
    pub label: Option<String>,
    pub value: JsonValue,
    pub version: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedComposerContentV1 {
    pub document: PersistedDocumentV1,
    pub assets: Vec<PersistedAssetV1>,
    pub references: Vec<PersistedReferenceV1>,
}

impl PersistedComposerContentV1 {
    /// 纯文本入口（与 Domain 的 from_plain_text 同形状，映射在 Phase B）。
    pub fn plain_text(text: impl Into<String>) -> Self {
        PersistedComposerContentV1 {
            document: PersistedDocumentV1 {
                version: 1,
                blocks: vec![PersistedBlockV1::Paragraph {
                    node_id: "n0".into(),
                    children: vec![PersistedInlineV1::Text {
                        node_id: "n0:t0".into(),
                        text: text.into(),
                        marks: Vec::new(),
                    }],
                }],
            },
            assets: Vec::new(),
            references: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_record_keeps_blob_reference_not_bytes() {
        let asset = PersistedAssetV1 {
            id: "asset-001".into(),
            blob_hash: Some("sha256:abcdef".into()),
            name: "diagram.png".into(),
            media_type: "image/png".into(),
            size_bytes: 12345,
            alt: None,
            send_as_file: false,
            availability: PersistedAssetAvailabilityV1::Available,
            version: 1,
        };
        let json = serde_json::to_value(&asset).unwrap();
        assert_eq!(json["blob_hash"], "sha256:abcdef");
        assert!(json.get("bytes").is_none(), "字节永不进 Scope JSON");
        let back: PersistedAssetV1 = serde_json::from_value(json).unwrap();
        assert_eq!(back, asset);
    }

    #[test]
    fn tagged_document_roundtrip() {
        let content = PersistedComposerContentV1::plain_text("A");
        let value = serde_json::to_value(&content).unwrap();
        assert_eq!(value["document"]["blocks"][0]["type"], "paragraph");
        let back: PersistedComposerContentV1 = serde_json::from_value(value).unwrap();
        assert_eq!(back, content);
    }
}
