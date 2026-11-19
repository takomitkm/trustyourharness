# 会话数据结构设计（HSF 1.0 Draft · 一页）

- **日期**：2026-11-19 · **状态**：设计稿，未排期
- **来源**：外部 HSF 规格全文（用户 2026-11-19 转贴）蒸馏；是 [session-storage-design.md](session-storage-design.md) 六层蓝图的具体化
- **原文存档**：规格全文逐字节存档于 [hsf-spec-1.0-draft-full.md](hsf-spec-1.0-draft-full.md)——本页是蒸馏摘要，**若有偏差以原文为准**，修改本页前先对拍原文
- **架构一句话**：SQLite 热库（规范数据权威）+ SHA-256 内容寻址 Blob（大载荷）+ JSONL/zstd 冷档（迁移与归档）——混合模型，**不是**纯 event sourcing

## 表清单（V1）

| 表 | 关键字段 / 约束 | 一句话 |
|---|---|---|
| `schema_migrations` | version 严格递增、checksum | 结构变更必走 Migration，禁止静默改旧迁移 |
| `workspaces` | `UNIQUE(canonical_root)` | 项目身份；不让会话靠 cwd 字符串反查归属 |
| `sessions` | `next_message_seq`/`next_event_seq` 计数器；`first_user_preview`/`last_assistant_preview`；status CHECK | 会话列表**只查这张表**，不为卡片读历史 |
| `session_runs` | `UNIQUE(session_id, turn_seq)`；status 含 queued/running/completed/failed/interrupted/cancelled | 一次完整 Agent 执行轮次 |
| `model_calls` | `UNIQUE(run_id, call_seq)`；provider/model、first_token_at_ms、usage | 每次真实模型请求独立记账 |
| `messages` | `UNIQUE(session_id, seq)`；role CHECK(system/developer/user/assistant/tool)；status；visibility；idempotency_key；source_* 三字段保留导入原时间 | 会话历史核心；完成的消息原则上不原地改写 |
| `message_parts` | `UNIQUE(message_id, ordinal)`；**三态 CHECK**：inline⇔blob⇔archive 互斥（payload_storage 决定哪两列为 NULL） | 多模态与工具交互的内容单元 |
| `message_part_chunks` | `PRIMARY KEY(part_id, chunk_seq)` | 流式增量缓冲（≈4KiB/250ms 落一块），崩溃后留作 interrupted 证据 |
| `blobs` | hash 主键、bytes、mime、`UNIQUE(relative_path)` | 大载荷元数据；路径=`blobs/sha256/ab/cd/<64hex>`，不用原始文件名 |
| `part_attachments` | `PRIMARY KEY(part_id, ordinal)` + blob_hash 引用 | 附件关系；GC 依据 |
| `tool_invocations` | 独立状态机（queued→running→succeeded/failed/cancelled/timed_out）；`UNIQUE(session_id, tool_call_id)`；retry_of 保留旧记录 | 工具调用全生命周期，失败不覆盖 |
| `session_events` | `UNIQUE(session_id, seq)`（**event_seq 与 message_seq 不同命名空间**）；occurred/recorded 分离 | 业务状态变化事件；不为每个 token 建事件 |
| `context_checkpoints` | covered_from/through + raw_keep_from + summary + **source_digest**；status 含 stale | 上下文压缩快照；不可变历史产物 |
| `model_call_context_items` | `PRIMARY KEY(model_call_id, ordinal)`；content_hash | **Run Context Manifest**：该次请求实际看到了哪些内容 |
| `session_snapshots` | state_kind + encoding(json/blob/delta) + content_hash | Agent 运行状态快照；V1 只做完整快照，delta 留字段不实现 |
| `import_mappings` | `UNIQUE(source_system, source_session_id, source_message_id)` | 防重复导入 |

## 核心关系（别混淆的两个概念）

```
Workspace → Session ─┬─ SessionRun ─┬─ ModelCall ×N（含重试/模型切换）
                     │              └─ ToolInvocation ×N
                     ├─ Message → MessagePart → BlobRef
                     ├─ SessionEvent（独立 seq 流）
                     ├─ ContextCheckpoint（对话上下文压缩）
                     └─ SessionSnapshot（运行状态，≠Checkpoint）
```

**Run ≠ ModelCall**：一次 Run = 模型请求→工具→再请求→…→最终响应；Run 状态必须覆盖工具执行期，
不能在第一个模型响应结束时就算完成。**Checkpoint ≠ Snapshot**：前者压缩对话历史，后者保存
"改了哪几个文件、哪个没测"这类运行状态——聊天摘要不是可靠的运行状态库。

## ID / Seq / 时间

- **ID 全 UUIDv7**（RFC 9562）——有时间特征但**不**作为排序依据
- **排序一律 seq**：message_seq / event_seq 各自从 1 起、永不复用；禁 UUID 排序、mtime、创建时间、导入原始时间戳
- **时间 = UTC Unix 毫秒**，字段统一 `_at_ms` 后缀；导入数据分 `source_created_at_ms`（原始）与 `created_at_ms`（本系统写入），不混用

## Part 类型（V1 ≥14）

text / reasoning / tool_call / tool_result / image / audio / video / file / json / citation /
refusal / artifact / status / custom——每类型自带 schema_version；**未知类型以原始 JSON 保留，
禁止导入时静默丢弃**；禁止把多模态先转 Markdown/Base64 再塞 text_content。

## 五个架构决策（改成本最高的地方）

1. **Message / Run / ModelCall 必须分开**——多 Agent、模型切换、工具链、重试不被一张 message 表锁死
2. **混合模型，不过早上纯 event sourcing**——内容进 Message/Part，事件只记状态变化，避免大对象重复入事件流
3. **Snapshot 与 Compaction 是两件事**
4. **解决膨胀靠生命周期，不只靠压缩**——小文本留 SQLite，大文本按阈值转 Blob，历史完整保留
5. **V1 不实现 Matrix 式 delta**——等压测出重复率/差异大小/重建耗时再决定，避免父快照缺失与增量链损坏问题
