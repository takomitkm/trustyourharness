# Run 结果报告：Bridge V2 Phase A（四层模型类型定义 + ack 修复）

## 做了什么（规格 §1 审计 + §11 Phase A + §8）

1. **§1 审计** → `docs/architecture/composer-model-boundaries-audit.md`：规格五断言逐一
   源码核实**全部属实**（SnapshotDto.state / Persist{ScopeState} / Outcome 用 DeliveryResult /
   Edit 只有 text / WASM from_str::<ScopeState> L45 / ack 引用早删 L235-240），补两条发现
   （runtime Event/Effect 本体携带 core 类型；ScopeState.schema 是旧内嵌版本标记）。
2. **Phase A 类型定义（零运行链改动，V1/现有 92 判据不动）**：
   - composer-core：ids.rs 增 AssetId/ReferenceId/DocumentNodeId（GenerationId 复用裁定=
     不新建同义类型，规格 §2.2）；新 content.rs——ComposerContent/ComposerDocument/
     BlockNode/InlineNode/TextMark/AssetDescriptor/ReferenceDescriptor + validate 九条
     + plain_text 投影 + from_plain_text（15 测试）
   - composer-session-format（新 crate，**零 core 依赖**）：PersistedEnvelopeV1（格式/
     版本闸门，坏版本明确报错）/ PersistedScopeV1（Submissions+PendingReceipts 完整
     数据，禁 count）/ PersistedAssetV1 带 blob_hash 引用（字节永不进 JSON）/ validate
     （9 测试）
   - composer-bridge-protocol/src/v2：PROTOCOL_VERSION=2 + 请求/回执信封 + 全套 DTO
     （8 测试，含架构约束静态断言：v2 六文件不得出现 composer_core/ScopeState 字样）
   - 模型定义文档：docs/architecture/composer-four-layer-model.md（四层 ER/职责表/
     Phase A 测试覆盖/未做清单）
3. **§8 ack 修复**：get→map_outcome→成功才 remove（错误回执不再吞 ref_id）；
   persist_revision 递增时机绑到验证成功后；新增 BM03b–g 六测试。
4. **本地修正落地**：L4 首个 adapter 定为 taino（主力编辑器），Lexical 次之。

## 测试结果（校机 D:\yzd\bh-sync@82d45e2→d349b47 内容，cargo test --workspace）

- **177 passed / 0 failed**（原 133 基线 + 新增 44：content 18/session-format 9/v2 8/
  ack 修复 6 + 架构约束等）；wasm 载体 6/6（guest 构建后）。
- 过程抓出并修复 3 个真 bug：session-format 缺导入、or-pattern 少 `level`、
  **validate 两处逻辑错**（BTreeSet 先去重致重复 NodeId 检测恒过；check_block 共用
  ok 旗标致悬空 Mention 报成 DanglingAsset）——测试先行抓现行后修。

## 已知问题 / 边界

- E2E（浏览器七套）未跑——Phase A 未接运行链，无新 UI 行为；Phase C 接入时全跑。
- V2 Mapper/Golden JSON/LegacyScopeStateV1 迁移/编辑器接入/AssetStore = Phase B/C（待续）。
- 里程碑口径（规格 §13）：类型=已定义+单元测试过；**未接入运行链**。
