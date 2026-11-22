//! 内容 DTO（规格 §4.2）：与 Domain/持久化各自独立定义，禁止 `type X = InternalType`。
//! 字段身份一律字符串——协议层不做 Rust 新类型（映射校验在 Mapper，Phase B）。

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComposerContentDto {
    pub document: ComposerDocumentDto,
    pub assets: Vec<AssetDto>,
    pub references: Vec<ReferenceDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComposerDocumentDto {
    pub version: u32,
    pub blocks: Vec<BlockNodeDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockNodeDto {
    Paragraph {
        node_id: String,
        children: Vec<InlineNodeDto>,
    },
    Heading {
        node_id: String,
        level: u8,
        children: Vec<InlineNodeDto>,
    },
    Quote {
        node_id: String,
        children: Vec<BlockNodeDto>,
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
pub enum InlineNodeDto {
    Text {
        node_id: String,
        text: String,
        marks: Vec<TextMarkDto>,
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
pub enum TextMarkDto {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
}

/// 附件 DTO：逻辑身份+元数据，只有 `blob_hash` 可选引用——**无字节、无 object URL**。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDto {
    pub id: String,
    pub name: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub alt: Option<String>,
    pub send_as_file: bool,
    pub availability: AssetAvailabilityDto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetAvailabilityDto {
    Available,
    Missing,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferenceDto {
    pub id: String,
    pub kind: String,
    pub label: Option<String>,
    pub value: JsonValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_dto_shape_is_tagged_and_editor_agnostic() {
        let content = ComposerContentDto {
            document: ComposerDocumentDto {
                version: 1,
                blocks: vec![BlockNodeDto::Paragraph {
                    node_id: "n0".into(),
                    children: vec![InlineNodeDto::Attachment {
                        node_id: "n0:a".into(),
                        asset_id: "asset-001".into(),
                    }],
                }],
            },
            assets: vec![AssetDto {
                id: "asset-001".into(),
                name: "diagram.png".into(),
                media_type: "image/png".into(),
                size_bytes: 1,
                alt: None,
                send_as_file: false,
                availability: AssetAvailabilityDto::Available,
            }],
            references: Vec::new(),
        };
        let value = serde_json::to_value(&content).unwrap();
        assert_eq!(value["document"]["blocks"][0]["type"], "paragraph");
        assert_eq!(value["document"]["blocks"][0]["children"][0]["type"], "attachment");
        assert!(value["assets"][0].get("bytes").is_none(), "字节永不进协议");
        let back: ComposerContentDto = serde_json::from_value(value).unwrap();
        assert_eq!(back, content);
    }
}
