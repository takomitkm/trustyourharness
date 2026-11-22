//! Bridge DTO V2（四层模型的 L3，规格 §4）。
//!
//! 与 V1 的关系：D002/BM05 冻结了 V1 JSON 形状——V1 不得悄悄改字段，V2 独立定义。
//! 两个版本线并存直至全部 Host 迁移完（规格 §9；本 crate 的 `lib.rs` 仍是 V1）。
//!
//! V2 的三条纪律：
//! 1. **不依赖 composer-core**——本模块所有类型只用 serde 自描述（架构约束测试钉死，
//!    `composer_core`/`ScopeState` 字样不得出现在本目录）；
//! 2. **DTO 是能力边界**——UI 只能发 Edit/Submit/Retry/Remove 等意图
//!    （[`ComposerEventDtoV2`]），DeliverResult/Persisted/LookupResult 这类回执只能由
//!    Host 经 [`AckRequestDto`] 进 Core；
//! 3. **Snapshot 是只读 UI 视图**——只暴露 UI 需要的内容（§4.4 的禁暴露清单），
//!    内部加字段不自动要求 UI 升级协议。
//!
//! Mapper（domain↔DTO，规格 §4.9）在 Phase B 于 Host 侧实现；Phase A 只定义形状。

pub mod common;
pub mod content;
pub mod effect;
pub mod event;
pub mod outcome;
pub mod snapshot;

use serde::{Deserialize, Serialize};

pub use common::{BusyEnterDtoV2, DraftVersionDto, SubmissionStatusDtoV2};
pub use content::{
    AssetAvailabilityDto, AssetDto, BlockNodeDto, ComposerContentDto, ComposerDocumentDto,
    InlineNodeDto, ReferenceDto, TextMarkDto,
};
pub use effect::EffectDtoV2;
pub use event::ComposerEventDtoV2;
pub use outcome::{DeliveryDto, DeliveryResultDto, EffectOutcomeDtoV2, ReceiptDto};
pub use snapshot::{DraftViewDto, RuntimeViewDto, ScopeSnapshotDto, SnapshotDtoV2, SubmissionViewDto};

/// Bridge 协议版本（规格 §4.1）。V1=1（lib.rs 的既有形状），V2=本模块。
pub const PROTOCOL_VERSION: u32 = 2;

/// 新式请求信封：所有入口先查版本，不支持时明确报 ProtocolVersionMismatch。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeRequestDto {
    pub protocol_version: u32,
    pub request_id: String,
    pub payload: ComposerEventDtoV2,
}

/// 回执请求信封：Host 执行 Effect 后经此进入 Core（UI 不得伪造）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AckRequestDto {
    pub protocol_version: u32,
    pub ref_id: u64,
    pub outcome: EffectOutcomeDtoV2,
}

/// 版本不匹配错误（§4.1：明确报错，不静默降级）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolVersionMismatch {
    pub received: u32,
    pub supported: u32,
}

impl BridgeRequestDto {
    pub fn validate_version(&self) -> Result<(), ProtocolVersionMismatch> {
        if self.protocol_version == PROTOCOL_VERSION {
            Ok(())
        } else {
            Err(ProtocolVersionMismatch {
                received: self.protocol_version,
                supported: PROTOCOL_VERSION,
            })
        }
    }
}

impl AckRequestDto {
    pub fn validate_version(&self) -> Result<(), ProtocolVersionMismatch> {
        if self.protocol_version == PROTOCOL_VERSION {
            Ok(())
        } else {
            Err(ProtocolVersionMismatch {
                received: self.protocol_version,
                supported: PROTOCOL_VERSION,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 架构约束测试（规格 §10.3）：V2 DTO 文件不得引用 composer-core 类型——
    /// L3 独立性由"根本不 import"保证，而不是靠 review。
    /// （mod.rs 自身含本测试的断言字样，故只检查六个纯 DTO 文件。）
    #[test]
    fn v2_module_never_imports_core_types() {
        const SOURCES: &[&str] = &[
            include_str!("common.rs"),
            include_str!("content.rs"),
            include_str!("effect.rs"),
            include_str!("event.rs"),
            include_str!("outcome.rs"),
            include_str!("snapshot.rs"),
        ];
        for source in SOURCES {
            assert!(
                !source.contains("composer_core"),
                "V2 DTO 不得依赖 composer-core"
            );
            assert!(!source.contains("ScopeState"), "V2 DTO 不得出现 ScopeState");
        }
    }

    #[test]
    fn version_gate_rejects_mismatch() {
        let request = BridgeRequestDto {
            protocol_version: 1,
            request_id: "r1".into(),
            payload: ComposerEventDtoV2::Flush,
        };
        assert_eq!(
            request.validate_version(),
            Err(ProtocolVersionMismatch { received: 1, supported: 2 })
        );
        let ok = BridgeRequestDto { protocol_version: 2, ..request };
        assert!(ok.validate_version().is_ok());
    }

    #[test]
    fn ack_envelope_roundtrip() {
        let ack = AckRequestDto {
            protocol_version: 2,
            ref_id: 7,
            outcome: EffectOutcomeDtoV2::Delivered {
                result: DeliveryResultDto::Accepted {
                    receipt: ReceiptDto {
                        message_id: "m1".into(),
                        scope_id: "scope".into(),
                        turn_id: "turn-1".into(),
                    },
                },
            },
        };
        let value = serde_json::to_value(&ack).unwrap();
        assert_eq!(value["outcome"]["result"]["status"], "accepted");
        assert_eq!(value["outcome"]["result"]["receipt"]["message_id"], "m1");
        let back: AckRequestDto = serde_json::from_value(value).unwrap();
        assert_eq!(back, ack);
    }
}
