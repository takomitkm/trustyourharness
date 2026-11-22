//! Effect DTO V2（规格 §4.8）：Host 要执行的动作。Persist 的 payload 是持久化
//! Codec 的产物（带 payload_format 自描述），Host 只做 CAS 存取——不需要懂 Domain。

use crate::v2::common::DraftVersionDto;
use crate::v2::content::ComposerContentDto;
use crate::v2::outcome::DeliveryDto;
use serde::{Deserialize, Serialize};

/// Persist payload 的自描述格式标记（§4.8；版本属于持久化层，不是协议版本）。
pub const PERSIST_PAYLOAD_FORMAT_V1: &str =
    "application/vnd.harness.composer-scope+json;version=1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EffectDtoV2 {
    Persist {
        expected_revision: u64,
        next_revision: u64,
        payload_format: String,
        payload: String,
    },
    Deliver {
        message_id: String,
        attempt_id: String,
        delivery: DeliveryDto,
        turn_id: Option<String>,
        content: ComposerContentDto,
    },
    ScheduleTimeout {
        message_id: String,
        attempt_id: String,
        deadline_ms: u64,
    },
    CancelTimeout {
        message_id: String,
        attempt_id: String,
    },
    Lookup {
        message_id: String,
    },
    CancelAllLookups,
}

/// Effect 引用配对用的基准版本提示（Persist 之外的宿主内部用途，规格未单列——
/// 保留 DraftVersionDto 的协议层复用点）。
pub type EffectBaseDraftDto = DraftVersionDto;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persist_payload_is_opaque_with_format_tag() {
        let effect = EffectDtoV2::Persist {
            expected_revision: 41,
            next_revision: 42,
            payload_format: PERSIST_PAYLOAD_FORMAT_V1.into(),
            payload: "{\"format\":\"composer-scope\"}".into(),
        };
        let value = serde_json::to_value(&effect).unwrap();
        assert_eq!(value["type"], "persist");
        assert_eq!(
            value["payload_format"],
            "application/vnd.harness.composer-scope+json;version=1"
        );
        // payload 是不透明字符串：Host 不解构、不校验其内部形状。
        assert!(value["payload"].is_string());
    }

    #[test]
    fn deliver_carries_content_and_identity() {
        let effect = EffectDtoV2::Deliver {
            message_id: "m1".into(),
            attempt_id: "a1".into(),
            delivery: DeliveryDto::Start,
            turn_id: None,
            content: ComposerContentDto {
                document: crate::v2::ComposerDocumentDto {
                    version: 1,
                    blocks: Vec::new(),
                },
                assets: Vec::new(),
                references: Vec::new(),
            },
        };
        let value = serde_json::to_value(&effect).unwrap();
        assert_eq!(value["type"], "deliver");
        assert_eq!(value["delivery"], "start");
    }
}
