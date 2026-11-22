//! 领域内容模型（四层模型的 L1 内容半区，四层规格 §2）。
//!
//! 与 [`crate::state::Snapshot`] 的关系：Snapshot 是现行纯文本运行形状（D009 主线），
//! 本模块是规格 §2.3 的结构化升级，Phase A 只新增不接线（不改运行链）。
//! 正文语义的唯一权威从"text+两个字符串数组"改为：**Document 是唯一正文事实，
//! plain_text 是派生投影**（§2.7）。
//!
//! 身份纪律（§2.2）：MessageId 不可代替 AssetId；AssetId 不可代替 DocumentNodeId；
//! ReferenceId 不可代替 DocumentNodeId；各 ID 走 [`crate::ids`] 的新类型，禁止裸 String。

use crate::ids::{AssetId, DocumentNodeId, ReferenceId};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 一份草稿/一条消息的完整业务内容：结构化文档 + 资产清单 + 引用清单。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComposerContent {
    pub document: ComposerDocument,
    pub assets: Vec<AssetDescriptor>,
    pub references: Vec<ReferenceDescriptor>,
}

/// 编辑器无关的文档模型（§2.3）。`version` 是 Domain 文档版本（§9：与持久化/
/// 协议/编辑器文档四个版本线分开管理）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComposerDocument {
    pub version: u32,
    pub blocks: Vec<BlockNode>,
}

/// 块级节点。`Custom` 为上游扩展的保留通道（未知类型以原 JSON 保留，禁静默删除）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockNode {
    Paragraph {
        id: DocumentNodeId,
        children: Vec<InlineNode>,
    },
    Heading {
        id: DocumentNodeId,
        level: u8,
        children: Vec<InlineNode>,
    },
    Quote {
        id: DocumentNodeId,
        children: Vec<BlockNode>,
    },
    Custom {
        id: DocumentNodeId,
        namespace: String,
        node_type: String,
        version: u32,
        data: JsonValue,
    },
}

/// 行内节点。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InlineNode {
    Text {
        id: DocumentNodeId,
        text: String,
        marks: Vec<TextMark>,
    },
    HardBreak {
        id: DocumentNodeId,
    },
    Mention {
        id: DocumentNodeId,
        reference_id: ReferenceId,
        label: String,
    },
    Attachment {
        id: DocumentNodeId,
        asset_id: AssetId,
    },
    Custom {
        id: DocumentNodeId,
        namespace: String,
        node_type: String,
        version: u32,
        data: JsonValue,
    },
}

/// 文本标记（§2.3：预留集合，实际支持范围以现有编辑器功能为准）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextMark {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
}

/// 附件的逻辑身份与元数据（§2.4）——不含 Blob 字节、不含本地路径、不含 object URL。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDescriptor {
    pub id: AssetId,
    pub name: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub alt: Option<String>,
    pub send_as_file: bool,
    pub availability: AssetAvailability,
}

/// 本地可用性。上传状态是另一维度，不与本地可用性混一字段（§2.4）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetAvailability {
    Available,
    Missing,
}

/// 结构化外部引用（§2.5）——不简化为文本字符串，`value` 保留结构化数据。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferenceDescriptor {
    pub id: ReferenceId,
    pub kind: String,
    pub label: Option<String>,
    pub value: JsonValue,
}

/// 校验失败原因（§2.6）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentError {
    /// DocumentNodeId 在文档中必须唯一且非空。
    DuplicateOrEmptyNodeId,
    /// AssetId 在资产清单中必须唯一。
    DuplicateAssetId,
    /// 文档中的附件节点引用了不存在的 AssetId（正向引用必须成立）。
    DanglingAsset,
    /// Mention 引用了不存在的 ReferenceId。
    DanglingReference,
    /// Custom 扩展 JSON 超出深度上限（防循环/防失控嵌套）。
    ExtensionTooDeep,
    /// 扩展数据里出现了 object URL / blob URL —— 禁止持久化（§2.6 第 8 条）。
    ObjectUrlInExtension,
}

impl ContentError {
    pub fn reason(&self) -> &'static str {
        match self {
            ContentError::DuplicateOrEmptyNodeId => "文档节点 ID 重复或为空",
            ContentError::DuplicateAssetId => "资产 ID 重复",
            ContentError::DanglingAsset => "附件节点引用了不存在的资产",
            ContentError::DanglingReference => "Mention 引用了不存在的引用",
            ContentError::ExtensionTooDeep => "扩展 JSON 嵌套过深",
            ContentError::ObjectUrlInExtension => "扩展数据含 object/blob URL",
        }
    }
}

/// Custom 扩展 JSON 的最大嵌套深度（防御性上限；规格 §2.6 第 5 条）。
const MAX_EXTENSION_DEPTH: usize = 32;

impl ComposerContent {
    /// 纯文本入口（§2.7）：纯文本编辑器路径——单 Paragraph + 单 Text。
    pub fn from_plain_text(text: impl Into<String>) -> Self {
        ComposerContent {
            document: ComposerDocument {
                version: 1,
                blocks: vec![BlockNode::Paragraph {
                    id: DocumentNodeId::new("n0"),
                    children: vec![InlineNode::Text {
                        id: DocumentNodeId::new("n0:t0"),
                        text: text.into(),
                        marks: Vec::new(),
                    }],
                }],
            },
            assets: Vec::new(),
            references: Vec::new(),
        }
    }

    /// **Document 是唯一正文事实**；plain_text 是派生投影（§2.7）。
    ///
    /// 段落间以 `\n` 连接；Mention 投影为 label；附件/HardBreak 不产出字符
    /// （HardBreak 语义由块边界承担）。
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for block in &self.document.blocks {
            if !out.is_empty() {
                out.push('\n');
            }
            write_block_text(block, &mut out);
        }
        out
    }

    /// 是否值得提交：正文有非空白文本，或存在资产/引用（rail-only 附件合法）。
    pub fn has_content(&self) -> bool {
        let mut saw_text = false;
        for block in &self.document.blocks {
            collect_block_text(block, &mut |s| {
                if !s.trim().is_empty() {
                    saw_text = true;
                }
            });
        }
        saw_text || !self.assets.is_empty() || !self.references.is_empty()
    }

    /// 结构校验（§2.6）：正向引用必须成立；反向引用（资产未入正文）不强制。
    pub fn validate(&self) -> Result<(), ContentError> {
        // 收集用 Vec：BTreeSet 会先去重，重复 NodeId 检测必须在去重前做。
        let node_ids = collect_node_ids(&self.document.blocks);
        if node_ids.iter().any(|id| id.is_empty()) {
            return Err(ContentError::DuplicateOrEmptyNodeId);
        }
        let mut seen = std::collections::BTreeSet::new();
        for id in &node_ids {
            if !seen.insert(id.as_str()) {
                return Err(ContentError::DuplicateOrEmptyNodeId);
            }
        }
        let mut asset_ids = std::collections::BTreeSet::new();
        for asset in &self.assets {
            if asset.id.is_empty() || !asset_ids.insert(asset.id.as_str()) {
                return Err(ContentError::DuplicateAssetId);
            }
        }
        let reference_ids: std::collections::BTreeSet<&str> = self
            .references
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        if reference_ids.len() != self.references.len() {
            return Err(ContentError::DanglingReference);
        }
        // 正向引用 + 扩展数据检查（深度/URL），报错精确到种类。
        check_block(&self.document.blocks, &asset_ids, &reference_ids)?;
        Ok(())
    }
}

fn write_block_text(block: &BlockNode, out: &mut String) {
    match block {
        BlockNode::Paragraph { children, .. } | BlockNode::Heading { children, .. } => {
            for inline in children {
                write_inline_text(inline, out);
            }
        }
        BlockNode::Quote { children, .. } => {
            for inner in children {
                write_block_text(inner, out);
            }
        }
        BlockNode::Custom { .. } => {}
    }
}

fn write_inline_text(inline: &InlineNode, out: &mut String) {
    match inline {
        InlineNode::Text { text, .. } => out.push_str(text),
        InlineNode::HardBreak { .. } => {}
        InlineNode::Mention { label, .. } => out.push_str(label),
        InlineNode::Attachment { .. } => {}
        InlineNode::Custom { .. } => {}
    }
}

fn collect_block_text(block: &BlockNode, sink: &mut impl FnMut(&str)) {
    match block {
        BlockNode::Paragraph { children, .. } | BlockNode::Heading { children, .. } => {
            for inline in children {
                if let InlineNode::Text { text, .. } = inline {
                    sink(text);
                }
            }
        }
        BlockNode::Quote { children, .. } => {
            for inner in children {
                collect_block_text(inner, sink);
            }
        }
        BlockNode::Custom { .. } => {}
    }
}

fn collect_node_ids(blocks: &[BlockNode]) -> Vec<DocumentNodeId> {
    let mut out = Vec::new();
    fn walk(blocks: &[BlockNode], out: &mut Vec<DocumentNodeId>) {
        for block in blocks {
            match block {
                BlockNode::Paragraph { id, children } | BlockNode::Heading { id, children, .. } => {
                    out.push(id.clone());
                    for inline in children {
                        out.push(inline_id(inline));
                    }
                }
                BlockNode::Quote { id, children } => {
                    out.push(id.clone());
                    walk(children, out);
                }
                BlockNode::Custom { id, .. } => {
                    out.push(id.clone());
                }
            }
        }
    }
    walk(blocks, &mut out);
    out
}

fn inline_id(inline: &InlineNode) -> DocumentNodeId {
    match inline {
        InlineNode::Text { id, .. }
        | InlineNode::HardBreak { id }
        | InlineNode::Mention { id, .. }
        | InlineNode::Attachment { id, .. }
        | InlineNode::Custom { id, .. } => id.clone(),
    }
}

fn check_block(
    blocks: &[BlockNode],
    assets: &std::collections::BTreeSet<&str>,
    references: &std::collections::BTreeSet<&str>,
) -> Result<(), ContentError> {
    for block in blocks {
        match block {
            BlockNode::Paragraph { children, .. } | BlockNode::Heading { children, .. } => {
                for inline in children {
                    match inline {
                        InlineNode::Attachment { asset_id, .. } => {
                            if !assets.contains(asset_id.as_str()) {
                                return Err(ContentError::DanglingAsset);
                            }
                        }
                        InlineNode::Mention { reference_id, .. } => {
                            if !references.contains(reference_id.as_str()) {
                                return Err(ContentError::DanglingReference);
                            }
                        }
                        InlineNode::Custom { data, .. } => check_extension(data)?,
                        _ => {}
                    }
                }
            }
            BlockNode::Quote { children, .. } => check_block(children, assets, references)?,
            BlockNode::Custom { data, .. } => check_extension(data)?,
        }
    }
    Ok(())
}

fn check_extension(data: &JsonValue) -> Result<(), ContentError> {
    check_extension_depth(data, 0)?;
    if contains_object_url(data) {
        return Err(ContentError::ObjectUrlInExtension);
    }
    Ok(())
}

fn check_extension_depth(value: &JsonValue, depth: usize) -> Result<(), ContentError> {
    if depth > MAX_EXTENSION_DEPTH {
        return Err(ContentError::ExtensionTooDeep);
    }
    match value {
        JsonValue::Array(items) => {
            for item in items {
                check_extension_depth(item, depth + 1)?;
            }
        }
        JsonValue::Object(map) => {
            for item in map.values() {
                check_extension_depth(item, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn contains_object_url(value: &JsonValue) -> bool {
    match value {
        JsonValue::String(s) => {
            let s = s.trim();
            s.starts_with("blob:") || s.starts_with("object-url:")
        }
        JsonValue::Array(items) => items.iter().any(contains_object_url),
        JsonValue::Object(map) => map.values().any(contains_object_url),
        _ => false,
    }
}

impl ComposerDocument {
    /// 空文档非法（taino `doc content="block+"` 同款教训）：至少一个空段落。
    pub fn empty() -> Self {
        ComposerContent::from_plain_text("").document
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(id: &str) -> AssetDescriptor {
        AssetDescriptor {
            id: AssetId::new(id),
            name: format!("{id}.png"),
            media_type: "image/png".into(),
            size_bytes: 10,
            alt: None,
            send_as_file: false,
            availability: AssetAvailability::Available,
        }
    }

    fn content_with(blocks: Vec<BlockNode>, assets: Vec<AssetDescriptor>) -> ComposerContent {
        ComposerContent {
            document: ComposerDocument { version: 1, blocks },
            assets,
            references: Vec::new(),
        }
    }

    #[test]
    fn from_plain_text_roundtrips_through_plain_text() {
        let content = ComposerContent::from_plain_text("你好世界");
        assert_eq!(content.plain_text(), "你好世界");
        assert!(content.validate().is_ok());
        assert!(content.has_content());
    }

    #[test]
    fn multi_paragraph_order_is_preserved() {
        let mut content = ComposerContent::from_plain_text("一");
        content.document.blocks.push(BlockNode::Paragraph {
            id: DocumentNodeId::new("n1"),
            children: vec![InlineNode::Text {
                id: DocumentNodeId::new("n1:t0"),
                text: "二".into(),
                marks: vec![TextMark::Bold],
            }],
        });
        assert_eq!(content.plain_text(), "一\n二");
    }

    #[test]
    fn rail_only_asset_is_valid_and_submittable() {
        // 附件只挂 rail、未入正文：合法（§2.6 第 7 条），且算有内容。
        let content = content_with(Vec::new(), vec![asset("a1")]);
        assert!(content.validate().is_ok());
        assert!(content.has_content());
    }

    #[test]
    fn dangling_asset_reference_is_rejected() {
        let content = content_with(
            vec![BlockNode::Paragraph {
                id: DocumentNodeId::new("n0"),
                children: vec![InlineNode::Attachment {
                    id: DocumentNodeId::new("n0:a"),
                    asset_id: AssetId::new("ghost"),
                }],
            }],
            Vec::new(),
        );
        assert_eq!(content.validate(), Err(ContentError::DanglingAsset));
    }

    #[test]
    fn dangling_mention_reference_is_rejected() {
        let content = content_with(
            vec![BlockNode::Paragraph {
                id: DocumentNodeId::new("n0"),
                children: vec![InlineNode::Mention {
                    id: DocumentNodeId::new("n0:m"),
                    reference_id: ReferenceId::new("ghost"),
                    label: "main.rs:L20".into(),
                }],
            }],
            Vec::new(),
        );
        assert_eq!(content.validate(), Err(ContentError::DanglingReference));
    }

    #[test]
    fn mention_requires_structured_reference_not_string() {
        // Reference 不许简化为文本：value 保留结构化数据（§2.5）。
        let content = ComposerContent {
            document: ComposerDocument::empty(),
            assets: Vec::new(),
            references: vec![ReferenceDescriptor {
                id: ReferenceId::new("ref-001"),
                kind: "code-comment".into(),
                label: Some("main.rs:L20".into()),
                value: serde_json::json!({ "path": "src/main.rs", "line": 20 }),
            }],
        };
        assert!(content.validate().is_ok());
        assert!(content.has_content());
    }

    #[test]
    fn duplicate_node_id_is_rejected() {
        let content = content_with(
            vec![
                BlockNode::Paragraph {
                    id: DocumentNodeId::new("n0"),
                    children: vec![],
                },
                BlockNode::Paragraph {
                    id: DocumentNodeId::new("n0"),
                    children: vec![],
                },
            ],
            Vec::new(),
        );
        assert_eq!(
            content.validate(),
            Err(ContentError::DuplicateOrEmptyNodeId)
        );
    }

    #[test]
    fn empty_node_id_is_rejected() {
        let content = content_with(
            vec![BlockNode::Paragraph {
                id: DocumentNodeId::new("   "),
                children: vec![],
            }],
            Vec::new(),
        );
        assert_eq!(
            content.validate(),
            Err(ContentError::DuplicateOrEmptyNodeId)
        );
    }

    #[test]
    fn duplicate_asset_id_is_rejected() {
        let content = content_with(Vec::new(), vec![asset("a1"), asset("a1")]);
        assert_eq!(content.validate(), Err(ContentError::DuplicateAssetId));
    }

    #[test]
    fn unknown_custom_node_is_preserved_not_dropped() {
        // 未知扩展以原 JSON 保留（§2.6 第 6 条）——plain_text 不受影响但结构完整。
        let content = content_with(
            vec![BlockNode::Custom {
                id: DocumentNodeId::new("n0"),
                namespace: "vendor".into(),
                node_type: "widget".into(),
                version: 3,
                data: serde_json::json!({ "rows": 2 }),
            }],
            Vec::new(),
        );
        assert!(content.validate().is_ok());
        assert_eq!(content.plain_text(), "");
        assert!(!content.has_content());
    }

    #[test]
    fn object_url_in_extension_is_rejected() {
        let content = content_with(
            vec![BlockNode::Custom {
                id: DocumentNodeId::new("n0"),
                namespace: "vendor".into(),
                node_type: "widget".into(),
                version: 1,
                data: serde_json::json!({ "src": "blob:https://example/uuid" }),
            }],
            Vec::new(),
        );
        assert_eq!(content.validate(), Err(ContentError::ObjectUrlInExtension));
    }

    #[test]
    fn too_deep_extension_is_rejected() {
        let mut value = serde_json::json!(1);
        for _ in 0..40 {
            value = serde_json::json!({ "n": value });
        }
        let content = content_with(
            vec![BlockNode::Custom {
                id: DocumentNodeId::new("n0"),
                namespace: "vendor".into(),
                node_type: "widget".into(),
                version: 1,
                data: value,
            }],
            Vec::new(),
        );
        assert_eq!(content.validate(), Err(ContentError::ExtensionTooDeep));
    }

    #[test]
    fn whitespace_only_text_is_not_content_but_assets_are() {
        let text_only = ComposerContent::from_plain_text("   ");
        assert!(!text_only.has_content());
        let with_asset = content_with(
            vec![BlockNode::Paragraph {
                id: DocumentNodeId::new("n0"),
                children: vec![InlineNode::Text {
                    id: DocumentNodeId::new("n0:t"),
                    text: "  ".into(),
                    marks: Vec::new(),
                }],
            }],
            vec![asset("a1")],
        );
        assert!(with_asset.has_content());
    }

    #[test]
    fn serde_shape_uses_tagged_enums() {
        let content = ComposerContent::from_plain_text("A");
        let json = serde_json::to_value(&content).unwrap();
        let block = &json["document"]["blocks"][0];
        assert_eq!(block["type"], "paragraph");
        assert_eq!(block["children"][0]["type"], "text");
        let back: ComposerContent = serde_json::from_value(json).unwrap();
        assert_eq!(back, content);
    }

    #[test]
    fn quote_nests_blocks_and_keeps_ids_unique() {
        let content = content_with(
            vec![BlockNode::Quote {
                id: DocumentNodeId::new("q0"),
                children: vec![BlockNode::Paragraph {
                    id: DocumentNodeId::new("q0:p0"),
                    children: vec![InlineNode::Text {
                        id: DocumentNodeId::new("q0:p0:t"),
                        text: "引文".into(),
                        marks: Vec::new(),
                    }],
                }],
            }],
            Vec::new(),
        );
        assert!(content.validate().is_ok());
        assert_eq!(content.plain_text(), "引文");
    }
}
