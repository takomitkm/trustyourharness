//! Scope 级持久化形状（规格 §3.2）：Submission/PendingReceipt **完整**保留——
//! 回执不能只存 count（规格明令），draft/submissions/turn 账本全部带进新格式。

use crate::content::{PersistedBlockV1, PersistedComposerContentV1, PersistedInlineV1};
use crate::ValidationError;
use serde::{Deserialize, Serialize};
use serde_json::Map as JsonMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedScopeV1 {
    pub scope_id: String,
    pub store_revision: u64,
    pub draft: PersistedDraftV1,
    pub submissions: Vec<PersistedSubmissionV1>,
    pub active_turn_id: Option<String>,
    pub finished_turn_ids: Vec<String>,
    /// 未落账的宿主回执：完整保留数据（不能只存 count），flush 重试时逐条结算。
    pub pending_receipts: Vec<PersistedPendingReceiptV1>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    /// 非核心扩展（未知字段保留通道，禁静默删除）。
    pub extensions: JsonMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedDraftV1 {
    pub generation_id: String,
    pub draft_revision: u64,
    pub content: PersistedComposerContentV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedSubmissionV1 {
    pub message_id: String,
    pub scope_id: String,
    pub generation_id: String,
    pub content: PersistedComposerContentV1,
    pub delivery: PersistedDeliveryV1,
    pub status: PersistedStatusV1,
    pub queued_for_turn_id: Option<String>,
    pub attempt_id: Option<String>,
    pub attempts: u32,
    pub created_at_ms: u64,
    pub reason: Option<String>,
    /// 已收到的最终回执（Accepted 才有；Rejected/Uncertain 的原因在 `reason`）。
    pub receipt: Option<PersistedReceiptV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedPendingReceiptV1 {
    pub message_id: String,
    pub attempt_id: Option<String>,
    pub result: PersistedDeliveryResultV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedReceiptV1 {
    pub message_id: String,
    pub scope_id: String,
    pub turn_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedDeliveryV1 {
    Start,
    Steer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedStatusV1 {
    Queued,
    Sending,
    Accepted,
    Rejected,
    Uncertain,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PersistedDeliveryResultV1 {
    Accepted { receipt: PersistedReceiptV1 },
    Rejected { reason: String },
    Uncertain { reason: String },
}

impl PersistedScopeV1 {
    /// 结构校验（Phase A 自身的守卫；codec 接线时在 decode 后强制调用）：
    /// 身份唯一性、正向引用完整性、扩展数据卫生。
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut problems: Vec<String> = Vec::new();
        if self.scope_id.trim().is_empty() {
            problems.push("scope_id 为空".into());
        }
        // 资产唯一 + 引用清单键集。
        let mut asset_ids = std::collections::BTreeSet::new();
        for asset in &self.draft.content.assets {
            if asset.id.trim().is_empty() || !asset_ids.insert(asset.id.as_str()) {
                problems.push(format!("草稿资产 ID 重复或为空: {}", asset.id));
            }
        }
        let draft_references: std::collections::BTreeSet<&str> = self
            .draft
            .content
            .references
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        if draft_references.len() != self.draft.content.references.len() {
            problems.push("草稿引用 ID 重复".into());
        }
        check_content_refs(&self.draft.content, &mut problems, "draft");
        crate::check_extension(
            &serde_json::Value::Object(self.extensions.clone()),
            &mut problems,
            "extensions",
        );
        // Submission：消息 ID 唯一；内容引用成立；receipt 身份与消息一致。
        let mut message_ids = std::collections::BTreeSet::new();
        for submission in &self.submissions {
            if submission.message_id.trim().is_empty()
                || !message_ids.insert(submission.message_id.as_str())
            {
                problems.push(format!("消息 ID 重复或为空: {}", submission.message_id));
            }
            check_content_refs(&submission.content, &mut problems, &submission.message_id);
            if let Some(receipt) = &submission.receipt {
                if receipt.message_id != submission.message_id {
                    problems.push(format!(
                        "回执身份与消息不一致: {} / {}",
                        receipt.message_id, submission.message_id
                    ));
                }
            }
        }
        // PendingReceipt：完整数据 + 身份一致。
        for pending in &self.pending_receipts {
            match &pending.result {
                PersistedDeliveryResultV1::Accepted { receipt } => {
                    if receipt.message_id != pending.message_id {
                        problems.push(format!(
                            "pending 回执身份不一致: {} / {}",
                            receipt.message_id, pending.message_id
                        ));
                    }
                }
                PersistedDeliveryResultV1::Rejected { .. }
                | PersistedDeliveryResultV1::Uncertain { .. } => {}
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(ValidationError { problems })
        }
    }
}

fn check_content_refs(
    content: &PersistedComposerContentV1,
    problems: &mut Vec<String>,
    where_: &str,
) {
    let assets: std::collections::BTreeSet<&str> =
        content.assets.iter().map(|a| a.id.as_str()).collect();
    let references: std::collections::BTreeSet<&str> =
        content.references.iter().map(|r| r.id.as_str()).collect();
    check_blocks(&content.document.blocks, &assets, &references, problems, where_);
    for asset in &content.assets {
        crate::check_extension(
            &serde_json::json!({
                "name": asset.name, "alt": asset.alt,
            }),
            problems,
            &format!("{where_}/asset/{}", asset.id),
        );
    }
    for reference in &content.references {
        crate::check_extension(&reference.value, problems, &format!("{where_}/ref/{}", reference.id));
    }
}

fn check_blocks(
    blocks: &[PersistedBlockV1],
    assets: &std::collections::BTreeSet<&str>,
    references: &std::collections::BTreeSet<&str>,
    problems: &mut Vec<String>,
    where_: &str,
) {
    for block in blocks {
        match block {
            PersistedBlockV1::Paragraph { node_id, children }
            | PersistedBlockV1::Heading { node_id, children, .. } => {
                if node_id.trim().is_empty() {
                    problems.push(format!("{where_}: 节点 ID 为空"));
                }
                for inline in children {
                    match inline {
                        PersistedInlineV1::Attachment { node_id, asset_id } => {
                            if !assets.contains(asset_id.as_str()) {
                                problems.push(format!(
                                    "{where_}/{node_id}: 附件节点引用不存在的资产 {asset_id}"
                                ));
                            }
                        }
                        PersistedInlineV1::Mention { node_id, reference_id, .. } => {
                            if !references.contains(reference_id.as_str()) {
                                problems.push(format!(
                                    "{where_}/{node_id}: Mention 引用不存在的引用 {reference_id}"
                                ));
                            }
                        }
                        PersistedInlineV1::Custom { data, .. } => {
                            crate::check_extension(data, problems, &format!("{where_}/{node_id}"))
                        }
                        _ => {}
                    }
                }
            }
            PersistedBlockV1::Quote { node_id, children } => {
                if node_id.trim().is_empty() {
                    problems.push(format!("{where_}: 节点 ID 为空"));
                }
                check_blocks(children, assets, references, problems, where_);
            }
            PersistedBlockV1::Custom { node_id, data, .. } => {
                crate::check_extension(data, problems, &format!("{where_}/{node_id}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{PersistedAssetAvailabilityV1, PersistedAssetV1};

    fn sample() -> PersistedScopeV1 {
        PersistedScopeV1 {
            scope_id: "scope".into(),
            store_revision: 1,
            draft: PersistedDraftV1 {
                generation_id: "gen-0".into(),
                draft_revision: 0,
                content: PersistedComposerContentV1::plain_text(""),
            },
            submissions: Vec::new(),
            active_turn_id: None,
            finished_turn_ids: Vec::new(),
            pending_receipts: Vec::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
            extensions: JsonMap::new(),
        }
    }

    #[test]
    fn sample_scope_validates() {
        assert!(sample().validate().is_ok());
    }

    #[test]
    fn dangling_attachment_in_submission_is_reported() {
        let mut scope = sample();
        scope.draft.content.assets = Vec::new();
        scope.draft.content.document.blocks =
            vec![PersistedBlockV1::Paragraph {
                node_id: "n0".into(),
                children: vec![PersistedInlineV1::Attachment {
                    node_id: "n0:a".into(),
                    asset_id: "ghost".into(),
                }],
            }];
        let error = scope.validate().unwrap_err();
        assert!(error
            .problems
            .iter()
            .any(|p| p.contains("引用不存在的资产 ghost")), "{error:?}");
    }

    #[test]
    fn mismatched_receipt_identity_is_reported() {
        let mut scope = sample();
        scope.pending_receipts = vec![PersistedPendingReceiptV1 {
            message_id: "m1".into(),
            attempt_id: None,
            result: PersistedDeliveryResultV1::Accepted {
                receipt: PersistedReceiptV1 {
                    message_id: "m2".into(),
                    scope_id: "scope".into(),
                    turn_id: "turn-1".into(),
                },
            },
        }];
        let error = scope.validate().unwrap_err();
        assert!(error.problems.iter().any(|p| p.contains("身份不一致")));
    }

    #[test]
    fn submission_roundtrip_keeps_full_receipt_data() {
        let mut scope = sample();
        scope.submissions = vec![PersistedSubmissionV1 {
            message_id: "m1".into(),
            scope_id: "scope".into(),
            generation_id: "gen-1".into(),
            content: PersistedComposerContentV1::plain_text("A"),
            delivery: PersistedDeliveryV1::Start,
            status: PersistedStatusV1::Accepted,
            queued_for_turn_id: None,
            attempt_id: Some("a1".into()),
            attempts: 1,
            created_at_ms: 5,
            reason: None,
            receipt: Some(PersistedReceiptV1 {
                message_id: "m1".into(),
                scope_id: "scope".into(),
                turn_id: "turn-1".into(),
            }),
        }];
        let bytes = serde_json::to_vec(&scope).unwrap();
        let back: PersistedScopeV1 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.submissions[0].receipt.as_ref().unwrap().turn_id, "turn-1");
        assert!(back.validate().is_ok());
    }

    #[test]
    fn blob_object_url_in_asset_name_is_reported() {
        let mut scope = sample();
        scope.draft.content.assets = vec![PersistedAssetV1 {
            id: "a1".into(),
            blob_hash: None,
            name: "blob:https://x/y".into(),
            media_type: "image/png".into(),
            size_bytes: 0,
            alt: None,
            send_as_file: false,
            availability: PersistedAssetAvailabilityV1::Available,
            version: 1,
        }];
        let error = scope.validate().unwrap_err();
        assert!(error
            .problems
            .iter()
            .any(|p| p.contains("object/blob URL")), "{error:?}");
    }
}
