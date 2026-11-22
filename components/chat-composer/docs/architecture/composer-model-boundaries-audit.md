# Composer 模型边界审计（composer-model-boundaries-audit）

- **日期**：2026-11-22 · **驱动**：Bridge V2 四层规格（[外部规格原文](../notes/composer-bridge-v2-spec-original.md)）§1
- **方法**：逐文件 grep + 读源码核对；每行给出类型与调用方
- **结论**：规格声称的四处跨层耦合 + WASM 持久化泄漏 + ack() 引用早删，**全部属实**，另有补充发现两条

## 一、跨层数据传递位置清单（L1 core 类型直接进协议/持久化）

| # | 位置 | 现状类型 | 调用方 | 问题层 |
|---|---|---|---|---|
| 1 | `composer-bridge-protocol/src/lib.rs:52-61` | `SnapshotDto { state: ScopeState }` | bridge-wasm、Dioxus app-web（经 dispatch 返回）、app-react 参考实现 | L3→L1 泄漏 |
| 2 | `lib.rs:99-101`（映射自 `Effect::Persist(state)`，lib.rs:338） | `EffectDto::Persist { state: ScopeState }` | 各 Host（web/desktop/CI 测试）：CAS 存取 payload | L3→L1 泄漏；Effect 本体在 `composer-runtime/src/lib.rs:93` 就携带 `ScopeState` |
| 3 | `lib.rs:127-135` | `EffectOutcomeDto::Delivered/LookupReturned { result: DeliveryResult }` | Host ack 回执入口（`ack()`） | L3→L1 泄漏（`Delivery/DeliveryResult/Receipt` 自 `composer_core::ids` 导入，lib.rs:13） |
| 4 | `lib.rs:67-70` + 映射 L301-302 | `ComposerEventDto::Edit { text: String }` → `Event::Edit { snapshot: Snapshot::text(text) }` | island JS（`document-changed, text`）、taino web 线（app-web main.rs:1019 注释"JSON 形状沿用 D009"） | L3 语义缺失（富文本/附件/引用无桥接通道） |
| 5 | `composer-bridge-wasm/src/lib.rs:45` | `serde_json::from_str::<ScopeState>(&json)` | WASM 构造函数恢复路径（FM09） | L2 缺失：持久化格式=core 结构体 serde 布局（ScopeState JSON） |
| 6 | `composer-core/src/state.rs:73-85` | `ScopeState { schema: u32, ... }` 派生 `Serialize/Deserialize` | 全部持久化路径（IDB CAS payload） | L1 即 L2：`schema: 1` 字段是唯一版本标记，无独立持久化 schema |
| 7 | `composer-runtime` Event/Effect 枚举 | `Event::Edit { snapshot: Snapshot }`、`Effect::Persist(ScopeState)` | runtime 引擎、composer-host | L1 内部无误，但协议镜像（#2/#3）随之泄漏 |

## 二、调用点核对（规格 §1 的 8 问）

1. **直接序列化 ScopeState**：bridge-protocol（DTO #1/#2）、bridge-wasm（#5）、IDB 持久化链（app-web idb.rs 存 `ScopeState` JSON，E3c FM09/FM23）。
2. **直接使用 core 的 ID/Delivery/DeliveryResult**：bridge-protocol lib.rs:13-14 导入 `AttemptId, Delivery, DeliveryResult, MessageId, TurnId`；Receipt（core L117-121）经 `DeliveryResult::Accepted` 间接进协议。
3. **消费 SnapshotDto 的**：bridge-wasm（构造返回）、app-web（`host.snapshot`/eval 供 island 与 taino 读 draft/submissions，main.rs L525 `cleared.snapshot.state.draft.snapshot.text` 直读内部）、app-react（参考实现）。
4. **Persist Effect 保存状态**：runtime 排空泵→Host `Persist` executor→IDB CAS（expected/next revision，FM23）；payload 即 ScopeState JSON。
5. **app-web 编辑器事件实际形状**：`{type:"document-changed", text}` / `{type:"submit-request"}`（main.rs:1019，D009 口径）——纯文本；taino 线同形状。
6. **app-react 映射**：types.ts / snapshots.ts / generated/protocol.ts 存在（参考实现+测试资产），映射仍在 v1 文本形状上。
7. **WASM 构造/恢复读旧状态**：lib.rs:36-47——`state_json = Some(...)` 时 `from_str::<ScopeState>`；None 时 `ScopeState::new(scope, gen-0)`。
8. **TS 协议文件与 Rust JSON 一致性**：generated/protocol.ts 由 v1 DTO 手工镜像（未逐字段机检——Phase B 建 Golden JSON 后以 fixture 钉死）。

## 三、ack() 引用早删（规格 §8，已核实）

`lib.rs:230-240`：`ack()` 先 `pending.remove(&ref_id)`（L235）再 `map_outcome(...).ok_or(OutcomeMismatch)`（L240）——形状不匹配时引用已被消费，正确回执随后到达只能得到 UnknownRef。修复方向与七条新测试见规格 §8（独立于四层重构，可先行）。

## 四、Phase A 的两个本地裁定

1. **GenerationId 复用**：`ids.rs:17-19` 注释明确 GenerationId 即 draft generation（"A handoff always allocates a new one"）——按规格 §2.2 "不要机械重复创建同义类型"，**不新建 DraftGenerationId**。
2. **L4 编辑器层**：规格以 Lexical 视角书写；本仓库主力编辑器为 taino-edit（Rust），island 为 fallback。EditorDocument 契约按编辑器无关设计，Phase C 接入时 taino adapter 优先、Lexical adapter 次之。

## 五、其余发现

- `Snapshot::is_empty`/`has_content` 以 text+两数组判定——V2 内容模型下由 `ComposerContent::has_content` 接管语义（Phase B 接线）。
- `ScopeState.schema: u32` 与规格的 Persistence `format_version` 是两个概念：前者是旧内嵌标记（迁移时读），后者是信封版本——迁移（LegacyScopeStateV1）按 §3.6 单独建模，不混淆。
