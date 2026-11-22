//! V2 公共小类型（规格 §4.1/§4.3/§4.4 引用）。

use serde::{Deserialize, Serialize};

/// 草稿版本（§4.3）：Edit/Submit 必须携带基准版本，Core 据此做过期检查。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftVersionDto {
    pub generation_id: String,
    pub revision: u64,
}

/// 忙碌进入模式（V2 独立定义；形状与 V1 的 BusyEnterDto 同构）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusyEnterDtoV2 {
    Queue,
    Steer,
}

/// 提交记录状态的协议投影（core Status 的独立镜像——不是 type alias）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatusDtoV2 {
    Queued,
    Sending,
    Accepted,
    Rejected,
    Uncertain,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_shape_is_snake_case() {
        assert_eq!(
            serde_json::to_value(SubmissionStatusDtoV2::Uncertain).unwrap(),
            serde_json::json!("uncertain")
        );
    }
}
