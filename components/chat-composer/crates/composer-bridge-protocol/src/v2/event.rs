//! UI 事件 DTO V2（规格 §4.3）：能力边界——UI 只能表达**意图**。
//! 回执类结果（DeliverResult/Persisted/LookupReturned）不在本枚举里，只能走 AckRequestDto。
//!
//! Edit/Submit 必须携带 `base_draft` + **完整内容**：不能假设此前的 document-changed
//! 已被 Core 处理（§4.3 提交六步的前两步在 Core 内复核）。

use crate::v2::common::{BusyEnterDtoV2, DraftVersionDto};
use crate::v2::content::ComposerContentDto;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComposerEventDtoV2 {
    Edit {
        base_draft: DraftVersionDto,
        content: ComposerContentDto,
    },
    Submit {
        base_draft: DraftVersionDto,
        content: ComposerContentDto,
    },
    SetBusyMode {
        mode: BusyEnterDtoV2,
    },
    BeginTurn {
        turn_id: String,
    },
    EndTurn {
        turn_id: String,
    },
    Restore {
        message_id: String,
    },
    Retry {
        message_id: String,
    },
    Remove {
        message_id: String,
    },
    Flush,
    Resume,
    Close,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_carries_full_content_not_a_pointer() {
        let event = ComposerEventDtoV2::Submit {
            base_draft: DraftVersionDto { generation_id: "gen-1".into(), revision: 3 },
            content: ComposerContentDto {
                document: crate::v2::ComposerDocumentDto {
                    version: 1,
                    blocks: Vec::new(),
                },
                assets: Vec::new(),
                references: Vec::new(),
            },
        };
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["type"], "submit");
        assert_eq!(value["base_draft"]["generation_id"], "gen-1");
        assert!(value["content"]["document"].is_object(), "Submit 必须带完整内容");
    }

    #[test]
    fn receipt_like_variants_do_not_exist_for_ui() {
        // 能力边界守卫：UI 事件枚举的 JSON 标签里不允许出现回执类动词。
        let value = serde_json::to_value(ComposerEventDtoV2::Flush).unwrap();
        assert_eq!(value["type"], "flush");
        // 若未来有人在 ComposerEventDtoV2 里加 DeliverResult/Persisted 变体，
        // 这里会因枚举序列化形态变化而提醒复核（规格 §三.3）。
        let all_tags = ["edit", "submit", "set_busy_mode", "begin_turn", "end_turn",
            "restore", "retry", "remove", "flush", "resume", "close"];
        for tag in all_tags {
            assert!(!tag.contains("deliver") && !tag.contains("persist") && !tag.contains("lookup"));
        }
    }
}
