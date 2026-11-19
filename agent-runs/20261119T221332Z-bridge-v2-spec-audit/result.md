# Run 结果报告：Bridge V2 四层模型规格核对 + 存档

## 做了什么

1. 规格原文逐字节存档（44,172B，cmp 验证一致）→ `docs/notes/composer-bridge-v2-spec-original.md`。
2. 对规格声称的耦合点逐条核对源码（`crates/composer-bridge-protocol/src/lib.rs`）：

| 规格断言 | 核对 | 证据 |
|---|---|---|
| `SnapshotDto.state` 直接用 `ScopeState` | ✅ 属实 | lib.rs L52/L61 |
| `EffectDto::Persist` 直接传 `ScopeState` | ✅ 属实 | L100-101（且 L161 注释自认"Effect 含 ScopeState"） |
| `EffectOutcomeDto` 直接用 Core 的 `Delivery/DeliveryResult` | ✅ 属实 | L13 use 导入 + L131/L135 |
| `ComposerEventDto::Edit` 只有 `text` | ✅ 属实 | L69-70（L301-302 `Snapshot::text(text)` 实锤 D009 纯文本主线） |
| `ack()` 先 `pending.remove` 再验证 outcome，形状错误即吞引用 | ✅ 属实 | L235 remove 先于 L240 map_outcome |

五条全部属实——外部评审读的是真源码。

## 评估（未实施）

- 规格与既有纪律兼容：明确保留 BM05 V1 冻结（另立 V2）、CAS、可靠交接六步、
  "UI 不得伪造 DeliverResult"——与我们 D002/FM 系列同源。
- **本地修正点 1（编辑器层）**：规格 L4 以 Lexical 视角书写，但本仓库主力编辑器已是
  taino-edit（Rust）、island 为 fallback。实施时 L4 应定义为"编辑器无关的 EditorDocument
  契约"，**首个 adapter 应是 taino**，Lexical adapter 次之——否则会把 fallback 编辑器
  当成数据标准，恰是规格自己警告的事。
- **本地修正点 2（范围）**：规格 §5 的 Lexical 自定义节点/AssetStore/浏览器 E2E 是
  island 线的工作量；taino 线已有 Attachment/Node 模型，Phase C 应按编辑器分两支排期。
- **可先行的独立小修**：§8 的 ack 修复（get→验证→再 remove，七条新测试）独立于四层
  重构，可单独先做。

## 待拍板

1. 直接开工 Phase A（四层类型定义，不动运行链），还是先落计划文档
   （e6-bridge-v2 计划 + PRODUCT-LINE-PLAN 排期，像 E4/E5 那样）？
2. ack 小修是否先独立提交？
3. L4 首个 adapter 按 taino 优先是否认可？

## 测试结果

本次仅存档+核对，无构建/测试。校验：cmp 逐字节一致 ✓；五断言 grep 实锤 ✓。
