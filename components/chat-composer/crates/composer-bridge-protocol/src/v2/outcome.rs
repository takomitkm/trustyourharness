//! 回执/投递方向 DTO（规格 §4.5–§4.7）：协议枚举独立定义，禁止直接用 core 的
//! Delivery/DeliveryResult/Receipt——映射校验（回执身份一致性）在 Mapper（Phase B）。

use serde::{Deserialize, Serialize};

/// 投递模式（core Delivery 的协议镜像）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryDto {
    Start,
    Steer,
}

/// 回执 DTO：字符串身份（§4.6——不得直接传内部 Receipt）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptDto {
    pub message_id: String,
    pub scope_id: String,
    pub turn_id: String,
}

/// 送达结果（§4.7）。刻意没有 Failed 变体：传输错误不是未送达的证明，
/// 缺回执必须是 Uncertain——与 core 的 DeliveryResult 语义一致。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DeliveryResultDto {
    Accepted { receipt: ReceiptDto },
    Rejected { reason: String },
    Uncertain { reason: String },
}

/// Effect 执行结果的回执方向枚举（Host → Core，经 AckRequestDto）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EffectOutcomeDtoV2 {
    Persisted {
        ok: bool,
    },
    Delivered {
        result: DeliveryResultDto,
    },
    TimedOut,
    LookupReturned {
        result: DeliveryResultDto,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncertain_has_no_failed_lookalike() {
        // 规格纪律：没有 Failed 变体——传输失败不能伪装成确定性拒绝。
        let value = serde_json::to_value(DeliveryResultDto::Uncertain {
            reason: "lost".into(),
        })
        .unwrap();
        assert_eq!(value["status"], "uncertain");
        assert!(value.get("failed").is_none());
    }

    #[test]
    fn outcome_roundtrip() {
        let outcome = EffectOutcomeDtoV2::TimedOut;
        let value = serde_json::to_value(&outcome).unwrap();
        assert_eq!(value["type"], "timed_out");
        let back: EffectOutcomeDtoV2 = serde_json::from_value(value).unwrap();
        assert_eq!(back, outcome);
    }
}
