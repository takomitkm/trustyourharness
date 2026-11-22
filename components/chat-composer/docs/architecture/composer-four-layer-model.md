# Composer 四层模型定义（Phase A 交付）

- **日期**：2026-11-22 · **状态**：类型已定义 + 单元测试；**未接入运行链**（Phase A 纪律）
- **规格**：[外部规格原文 §2–§5](../notes/composer-bridge-v2-spec-original.md)；审计见
  [composer-model-boundaries-audit.md](composer-model-boundaries-audit.md)

## 四层总览（同一附件在四层的四种形状）

```
L4 Editor Document（编辑器无关契约；taino adapter 优先、Lexical adapter 次之）
   └─ EditorAdapter（显式双向转换）
L1 Domain（composer-core）
   └─ Persistence Codec（Phase B；上层 Adapter 完成，core 不引用持久化 Schema）
L2 Persistence（composer-session-format，不依赖 composer-core）
   └─ Protocol Mapper（Phase B，集中实现）
L3 Bridge DTO V2（composer-bridge-protocol/src/v2，不依赖 composer-core）
```

同一附件：L4=编辑器附件节点 → L1=`AssetDescriptor`（AssetId 逻辑身份）→
L2=`PersistedAssetV1`（含 blob_hash 引用，字节在 BlobStore）→ L3=`AssetDto`（传输描述）。
四层引用同一逻辑资产，**不是同一种数据结构**；任何一层不得直接借用另一层的完整结构体。

## 类型关系（Phase A 已定义）

### L1 composer-core（Domain）

```
ids.rs 新增：AssetId / ReferenceId / DocumentNodeId（id_type! 宏，禁裸 String）
GenerationId 复用：ids.rs:17 已是 draft generation（规格 §2.2 不新建同义类型）
content.rs：
  ComposerContent { document: ComposerDocument, assets: Vec<AssetDescriptor>, references: Vec<ReferenceDescriptor> }
  ComposerDocument { version: u32, blocks: Vec<BlockNode> }
  BlockNode  = Paragraph | Heading | Quote | Custom{namespace, node_type, version, data}
  InlineNode = Text{marks} | HardBreak | Mention{reference_id, label} | Attachment{asset_id} | Custom
  TextMark   = Bold | Italic | Underline | Strikethrough | Code
  AssetDescriptor { id, name, media_type, size_bytes, alt, send_as_file, availability }
  AssetAvailability = Available | Missing（上传状态是另一维度，不混字段）
  ReferenceDescriptor { id, kind, label, value: JsonValue }（引用不简化为文本）
  方法：validate()（§2.6 九条）/ plain_text()（Document 唯一正文事实）/ has_content()
        / from_plain_text()（纯文本编辑器入口，单 Paragraph+Text）
  现行 Snapshot{text, attachment_ids, reference_ids} 原样保留（D009 运行链不变），
  draft.snapshot → draft.content 的迁移在 Phase B 接线。
```

### L2 composer-session-format（Persistence，新 crate）

```
envelope.rs：PersistedEnvelopeV1 { format: "composer-scope", format_version: 1, payload }
             decode 先识别信封再进 payload；未知格式/版本明确报错；坏 JSON 不覆盖旧数据
scope.rs：   PersistedScopeV1 { scope_id, store_revision, draft, submissions, active_turn_id,
             finished_turn_ids, pending_receipts(完整回执数据，非 count), created_at_ms,
             updated_at_ms, extensions } + validate()（身份唯一/正向引用/扩展卫生）
content.rs： PersistedComposerContentV1 / PersistedDocumentV1 / PersistedBlockV1 / ...
             PersistedAssetV1 { ..., blob_hash: Option<String>, version }（字节永不进 JSON）
             节点/资产/引用各带自己的 version（§3.3）
不依赖 composer-core；LegacyScopeStateV1 迁移与 codec 在 Phase B。
```

### L3 composer-bridge-protocol/src/v2（Bridge DTO）

```
PROTOCOL_VERSION = 2；BridgeRequestDto / AckRequestDto 信封带版本闸门（ProtocolVersionMismatch）
common.rs：  DraftVersionDto / BusyEnterDtoV2 / SubmissionStatusDtoV2
content.rs： ComposerContentDto / ComposerDocumentDto / BlockNodeDto / InlineNodeDto /
             TextMarkDto / AssetDto / AssetAvailabilityDto / ReferenceDto
event.rs：   ComposerEventDtoV2 = Edit{base_draft, content} | Submit{base_draft, content} |
             SetBusyMode | BeginTurn | EndTurn | Restore | Retry | Remove | Flush | Resume | Close
             （Submit 必带完整内容——不依赖此前 document-changed 是否已送达）
effect.rs：  EffectDtoV2 = Persist{expected/next_revision, payload_format, payload(opaque)} |
             Deliver{..., content} | ScheduleTimeout | CancelTimeout | Lookup | CancelAllLookups
             PERSIST_PAYLOAD_FORMAT_V1 = "application/vnd.harness.composer-scope+json;version=1"
outcome.rs： DeliveryDto / ReceiptDto / DeliveryResultDto（无 Failed 变体——缺回执=Uncertain）/
             EffectOutcomeDtoV2 = Persisted{ok} | Delivered | TimedOut | LookupReturned
snapshot.rs：SnapshotDtoV2 { protocol_version, revision, runtime: RuntimeViewDto,
             scope: ScopeSnapshotDto }——pending 只给计数不给队列；禁暴露内部 Ledger/Schema
架构约束测试：v2 六个 DTO 文件不得出现 composer_core / ScopeState 字样（include_str 静态断言）
Mapper（domain↔DTO 六函数）在 Phase B 集中实现（规格 §4.9），Phase A 只定义形状。
```

### L4 Editor Document（契约先行，Phase C 接入）

编辑器无关契约：`getDocument()/setDocument()/getPlainText()/clear()/focus()/setDisabled()/
destroy()` + `instanceId`/`editorRevision`；`document-changed` 为变更通知（适配层拉取完整
文档），submit-request 同步捕获当时文档。选区/IME/撤销栈是编辑器临时状态，**不进** ComposerContent。
Phase C 接入顺序：**taino adapter 优先**（主力编辑器），Lexical adapter 次之（island fallback）。

## 职责与依赖表（规格 §四 裁定，本地落位）

| 模块 | 权威对象 | 允许依赖 |
|---|---|---|
| composer-core | Domain Model | 业务类型 |
| composer-session-format | Persistence Model | 独立 Schema（零 core 依赖） |
| composer-bridge-protocol（v1 保留 + v2 新增） | Bridge DTO | v2 零 core 依赖；Mapper 在 Host 侧 |
| taino-edit / lexical-island | Editor Document | 各自编辑器 |
| composer-editor-adapter（Phase C 新建） | 编辑器与业务映射 | Editor 契约 + Bridge DTO |
| AssetStore（Phase C） | Blob 字节 | 平台存储 API |

## Phase A 测试覆盖（§10.1/§10.2/§10.3 对应项）

- core content 15 测试：纯文本往返/多段顺序/rail-only 附件/悬空 Asset 与 Reference 拒绝/
  重复与空 NodeId 拒绝/未知 Custom 保留/object URL 拒绝/深度超限拒绝/serde tagged 形状
- session-format 9 测试：信封往返+格式闸门（未知格式/坏版本/坏 JSON）/资产 blob_hash 引用
  非 bytes/全回执数据往返/悬空引用与回执身份校验/扩展卫生
- bridge v2 8 测试：V2 独立性静态断言/版本闸门/Ack 信封往返/Submit 带完整内容/UI 事件
  无回执动词/Persist payload 不透明+格式标记/Deliver 身份/Snapshot 只读视图隐藏队列
- ack 修复 6 新测试（BM03b–BM03g）：错形不消费引用→正确 ACK 成功/错形最小用例/成功后
  重复 ACK 不重放/未知 ref 无副作用/错配回执→Uncertain 非 Accepted/正确回执→Accepted

## 明确未做（Phase B/C 起，规格 §11）

- Domain↔Persisted codec 与 LegacyScopeStateV1 迁移（原数据可恢复证明）
- V2 Mapper 六函数 + Golden JSON Fixtures + BM05 协议测试更新
- 编辑器接入（taino 优先）与 AssetStore；V1 退役
- 运行链零改动：现有 92 判据与 D009 纯文本主线不受本阶段影响
