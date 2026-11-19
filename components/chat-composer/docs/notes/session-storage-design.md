# 设计：Harness 会话存储（调研结论，未排期）

- **日期**：2026-11-19
- **状态**：设计结论，**未实现、未排期**——是将来 composer 会话存储的蓝图，不是当前
  任何阶段的承诺；排期待拍板后才进 PRODUCT-LINE-PLAN
- **输入**：
  - [agent-session-formats.md](agent-session-formats.md)（dsh-chat-import 22 家解析器调研，
    证据级别=源码观察）
  - [im-storage-patterns.md](im-storage-patterns.md)（IM 存储机制调研，Matrix 官方文档已核实，
    其余公开资料级）
- **结论一句话**：OpenCode V2 的类型化序列 + Codex/DSH 的追加式归档 + Signal 的载荷出列 +
  Telegram 的轻量预览 + Matrix 的状态快照思想 + XMPP 的明确分页终点。
- **具体化（2026-11-19）**：外部 HSF 规格蒸馏成两页——表结构与关系见
  [hsf-data-model.md](hsf-data-model.md)，实现方必须遵守的契约见
  [hsf-protocol.md](hsf-protocol.md)；两页均为 Draft、未排期。规格原文全文逐字节存档于
  [hsf-spec-1.0-draft-full.md](hsf-spec-1.0-draft-full.md)（含完整 DDL、P0–P4 实施顺序与压测矩阵），
  蒸馏稿若有偏差以原文为准。

## 六层结构

### 1. 热数据：追加式事件主干

```sql
session_event(session_id, seq, event_id, type, schema_version, payload)
```

- `seq` 单调递增、session 内唯一——排序不靠时间戳、不靠 mtime
- 保留完整事件历史，支持恢复和重放
- 来源：opencode V2 的 `(session_id, seq)` + codex/DSH 的追加式思想

### 2. 查询投影：类型化消息与 Parts

```sql
message(...); part(...)
```

- 显式 user / assistant / tool / system 等类型
- Parts 独立记录；有状态的工具调用（status/input/error）
- 消息与事件流保持可追溯关系（投影可从 session_event 重建）
- 来源：opencode V2 的类型化记录 + part 表

### 3. Blob：外置大载荷

```sql
blob(hash, bytes, mime, storage, preview)
```

- SHA-256 寻址；工具输出、截图、文件等不直接膨胀数据库
- 预览信息保留在热数据（列表渲染永不碰 blob 本体）
- 来源：Signal/WhatsApp 附件出列 + Telegram stripped thumbnail 的"行内留迷你预览"

### 4. Context Checkpoint：模型上下文压缩

```sql
checkpoint(session_id, covered_through_seq, keep_from_seq, summary, status)
```

- 完整历史不删除；记录模型已经看到的历史范围
- `summary` 与 `recent` 分离（对应 opencode V2 的 compaction 两段语义）
- 只有 `completed` 的 checkpoint 才能成为有效上下文边界
- 来源：opencode V2 compaction 边界（全场唯一把上下文窗口语义存进数据结构的设计）

### 5. Cold Storage：冷数据归档

```
session.vN.jsonl.zst + manifest
```

- 按 seq 范围分块；自描述 schema；校验和、代次、压缩格式写进 manifest
- DB 保存索引，查询不必解压全史
- 来源：codex `.jsonl.zst` 身份键归一 + DSH `session.vN` 代次命名（实测 52 会话 48 个 v3，
  无代次命名就读不到当前代次）+ Slack 导出形状

### 6. Session Projection：列表快速读取

- 首条用户消息摘要 + 最近一条助手消息摘要 + 消息数量 + 最近活动时间 + 项目/工作目录/状态
- 会话卡片渲染只碰这一层，永不碰冷档或 blob 本体
- 来源：Telegram stripped thumbnail（内嵌迷你预览，列表先渲染）

## 三条设计约束

1. **压缩不能等于删除。** 模型上下文可以缩短，原始事件仍然完整保留
   （checkpoint 只是"模型看到了哪"的记录，不是 `DELETE`）。
2. **只有真正可重建的数据才能逐出。** Blob、冷档、数据库中的规范事件记录要明确区分
   生命周期，不能把持久化数据统称为"缓存"。哪些层可逐出：冷档分块可以重解压验证后
   再释放、投影可从事件主干重建；事件主干本身不可逐出（它是真源）。
3. **不机械照搬 Matrix"每 100 次状态变化落全量"。** 那是针对高度重复的房间状态；
   Harness 的 checkpoint 时机应按**上下文状态的变化成本、快照大小和恢复成本**自行确定。

## "过长卡顿"三问题 → 对应层

| 问题 | 由哪层解决 |
|---|---|
| 磁盘占用 | Blob 出列（3）+ 冷档压缩（5）+ 保留策略（另配，参考 Matrix `m.room.retention` 做成显式配置，不做隐式无限增长） |
| 加载卡顿 | 有界窗口 + `seq` 锚点翻页（读 session_event 时永不 `SELECT *` 全史，参考 Element/Discord/XMPP RSM；分页响应带终点标志） |
| 内存卡顿 | 虚拟化列表只实例化窗口内对象；冷档分块按需解压 |

## 明确不取的部分

- Matrix 的 DAG（线性对话用不上，纯负担）——只取它的 delta+周期快照**思想**用于会话状态
  （环境快照、大段重复注入），不用于消息本体
- Telegram 的服务器权威（composer 是本地优先，方向相反）——只取"可逐出/预览内嵌"两个姿态
- IRC / ICQ（无结构）
- dsh-chat-import 调研里的反面教材：豆包活体二进制、cline 三头分裂、crush 无 cwd、
  goose 孤儿数据、cursor/trae 借壳

## 开放项

- 排期未拍板；若立项，命名建议为 composer 的新阶段线（如 E6 / S 线），与 E4 乐观确认无依赖
- `session_event` 与 E5 插件 `FiberSpec.config` 的关系（插件产生的会话事件走同一主干）需在
  立项时定义
- 保留策略参数（N 条 / cooling N 天转冷档）待定
