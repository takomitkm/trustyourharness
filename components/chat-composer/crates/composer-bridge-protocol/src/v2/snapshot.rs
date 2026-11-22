//! Snapshot DTO V2（规格 §4.4）：只读 UI 视图——**不是** Core 内存的完整镜像。
//! 禁止暴露：PendingReceipt 内部队列、内部 Ledger、Domain Schema、持久化版本字段。

use crate::v2::common::{BusyEnterDtoV2, SubmissionStatusDtoV2};
use crate::v2::content::ComposerContentDto;
use crate::v2::outcome::{DeliveryDto, ReceiptDto};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotDtoV2 {
    pub protocol_version: u32,
    pub revision: u64,
    pub runtime: RuntimeViewDto,
    pub scope: ScopeSnapshotDto,
}

/// 运行时视图：UI 需要的门控/忙碌信息（pending 只给计数，不给队列）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeViewDto {
    pub admitting: bool,
    pub paused: bool,
    pub closed: bool,
    pub busy_mode: BusyEnterDtoV2,
    pub pending_receipt_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScopeSnapshotDto {
    pub scope_id: String,
    pub draft: DraftViewDto,
    pub submissions: Vec<SubmissionViewDto>,
    pub active_turn_id: Option<String>,
    pub finished_turn_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DraftViewDto {
    pub generation_id: String,
    pub revision: u64,
    pub content: ComposerContentDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubmissionViewDto {
    pub message_id: String,
    pub content: ComposerContentDto,
    pub status: SubmissionStatusDtoV2,
    pub delivery: DeliveryDto,
    pub attempt_id: Option<String>,
    pub attempts: u32,
    pub created_at_ms: u64,
    pub reason: Option<String>,
    pub receipt: Option<ReceiptDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SnapshotDtoV2 {
        SnapshotDtoV2 {
            protocol_version: 2,
            revision: 9,
            runtime: RuntimeViewDto {
                admitting: true,
                paused: false,
                closed: false,
                busy_mode: BusyEnterDtoV2::Queue,
                pending_receipt_count: 0,
            },
            scope: ScopeSnapshotDto {
                scope_id: "scope".into(),
                draft: DraftViewDto {
                    generation_id: "gen-1".into(),
                    revision: 2,
                    content: crate::v2::ComposerContentDto {
                        document: crate::v2::ComposerDocumentDto {
                            version: 1,
                            blocks: Vec::new(),
                        },
                        assets: Vec::new(),
                        references: Vec::new(),
                    },
                },
                submissions: vec![SubmissionViewDto {
                    message_id: "m1".into(),
                    content: crate::v2::ComposerContentDto {
                        document: crate::v2::ComposerDocumentDto {
                            version: 1,
                            blocks: Vec::new(),
                        },
                        assets: Vec::new(),
                        references: Vec::new(),
                    },
                    status: SubmissionStatusDtoV2::Queued,
                    delivery: DeliveryDto::Start,
                    attempt_id: None,
                    attempts: 0,
                    created_at_ms: 1,
                    reason: None,
                    receipt: None,
                }],
                active_turn_id: None,
                finished_turn_ids: Vec::new(),
            },
        }
    }

    #[test]
    fn snapshot_view_roundtrip_and_hides_internal_queues() {
        let snapshot = sample();
        let value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(value["protocol_version"], 2);
        // 只读视图纪律：只暴露 UI 需要的，pending_receipt 是计数不是队列。
        assert!(value["runtime"]["pending_receipt_count"].is_u64());
        assert!(value["runtime"].get("pending_receipts").is_none());
        assert!(value["scope"]["draft"]["content"].is_object());
        let back: SnapshotDtoV2 = serde_json::from_value(value).unwrap();
        assert_eq!(back, snapshot);
    }
}
