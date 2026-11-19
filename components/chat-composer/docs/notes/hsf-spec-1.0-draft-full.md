喵。我的建议是不要再寻找一家 IM 或一个 Agent 项目直接照抄，而是设计一套**面向长期运行、可恢复、可查询、可迁移、多模态的 Harness 会话存储格式**。

我建议采用：

# **Harness Session Format（HSF）**

推荐架构

**SQLite + Typed Records + Append-only Events + Content-addressed Blobs + JSONL/Zstandard Archive**

**OpenCode V2**

类型化消息、序号、Parts

**Codex Rollout**

追加式记录、自描述归档

**CloudEvents**

事件信封、版本化类型

**OpenTelemetry**

执行链、Trace、Span

**Git Object Model**

内容寻址、Blob 去重

**Jupyter nbformat**

多类型输出、MIME Parts

这不是纯粹的 Event Sourcing，也不是把所有内容塞进 JSONL。它是一个混合模型：关系数据库保存可查询的规范数据，追加式事件记录操作历史，独立 Blob 保存大型载荷，JSONL 负责迁移和冷归档。

OpenCode 当前代码确实提供了值得直接研究的 `session_v2`、`session_message`、`session_part`、`session_inbox`、`blob`、`blob_ref` 组合；CloudEvents、OpenTelemetry 和 Jupyter 则分别补足事件身份、执行关联和多模态输出模型。

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+3



下面是一份可以直接交给开发 Agent 的设计规格。

# Harness Session Format（HSF）设计规格

**版本：HSF 1.0 Draft**

**目标：** 为 Harness 建立长期可维护的本地会话存储结构，支持普通对话、Agent 执行、工具调用、多模态载荷、上下文压缩、历史搜索、会话导入导出、崩溃恢复和长期归档。

**实施原则：先保证数据模型正确，再实现性能优化。**

---

# 1. 总体架构

## 1.1 设计目标

存储系统必须满足：

1. 会话数量和消息数量持续增长时，打开会话不能退化为全历史加载。
2. 消息、工具调用、Agent 执行和模型调用具有明确的类型和关联关系。
3. 上下文压缩不得删除原始对话。
4. 图片、音频、视频、文件、超长工具输出不得无限膨胀数据库。
5. 崩溃后能够恢复已提交的消息和工具执行状态。
6. 支持导入其他 Agent 的历史记录，不依赖某个上游项目的内部数据库格式。
7. 格式具有明确版本，未来能够迁移。
8. 可以独立搜索消息、查看执行过程、提取附件和恢复会话。
9. 一个会话可以包含多个模型调用、工具调用及子 Agent 执行。
10. 不允许不同文件或数据库表之间出现多个无法确定优先级的权威数据来源。

## 1.2 三层存储

### 第一层：热数据库

文件：

`data/harness.db`

使用 SQLite。

作为当前会话规范数据的权威存储，保存：

- workspace
- session
- run
- model_call
- message
- message_part
- tool_invocation
- session_event
- context_checkpoint
- session_snapshot
- blob 元数据
- archive 索引
- 搜索索引

### 第二层：Blob 对象存储

目录：

`data/blobs/`

保存：

- 图片原始文件
- 音频文件
- 视频
- 文件附件
- 截图
- 大型工具输出
- 大型代码或数据文件
- 超长文本载荷

采用 SHA-256 内容寻址，允许相同文件跨消息、跨会话去重。

消息和 Parts 保存引用，不直接嵌入文件内容。

### 第三层：冷归档

目录：

`data/archives/`

使用：

- JSON Lines
- Zstandard
- 独立 Manifest
- 分代、分段命名

归档用于备份、迁移以及未来冷数据存储。

**V1 不允许归档完成后立即删除数据库原始数据。** 先建立完整的导出、验证、恢复流程，再单独实现冷热迁移。

## 1.3 核心数据关系

```
Workspace
   |
   +---- Session
            |
            +---- SessionRun
            |        |
            |        +---- ModelCall
            |        |       |
            |        |       +---- ContextItems
            |        |
            |        +---- ToolInvocation
            |
            +---- Message
            |        |
            |        +---- MessagePart
            |                  |
            |                  +---- BlobRef
            |
            +---- SessionEvent
            |
            +---- ContextCheckpoint
            |
            +---- SessionSnapshot
            |
            +---- SessionArchive
```

核心概念：

- Session：一次持续存在的会话。
- Run：一次由 Harness 管理的完整执行轮次。
- ModelCall：一次实际发给模型的请求。
- Message：一条对话消息。
- Part：消息内部独立的内容单元。
- ToolInvocation：一次工具调用的完整生命周期。
- SessionEvent：会话中发生的结构化事件。
- ContextCheckpoint：模型上下文压缩快照。
- SessionSnapshot：Agent 运行状态快照。
- Blob：外部载荷对象。

不要混淆 Run 和 ModelCall。

一个 Run 可以包含：

1. 一次模型请求。
2. 多次工具调用。
3. 第二次模型请求。
4. 更多工具调用。
5. 最终模型响应。

因此，一次 Agent 执行不应该被简单等同于一条 assistant message。

---

# 2. 经典格式及借鉴关系

实现时参考以下格式，但不能直接照搬它们的全部数据结构。

| 格式                  | 借鉴内容                                  | 不照搬的部分               |
| ------------------- | ------------------------------------- | -------------------- |
| OpenCode V2         | Typed Message、Seq、Parts、Compaction    | 上游具体字段和业务耦合          |
| Codex Rollout JSONL | 追加式记录、自描述、分页归档                        | 将整个数据库都做成 JSONL      |
| CloudEvents 1.0     | Event Envelope、ID、Source、Type、Version | 全部业务数据都采用事件信封包装      |
| OpenTelemetry       | Timestamp、TraceID、SpanID、Attributes   | 不将所有聊天内容当普通日志        |
| Matrix              | 事件类型化、状态快照思想                          | 房间 DAG 和分布式同步协议      |
| Jupyter nbformat    | Cell、Source、Outputs、MIME Bundle       | 不把每个对话转换为 Notebook   |
| Git Object Model    | SHA-256 内容寻址、不可变 Blob、引用              | 不需要直接采用 Git Packfile |
| JSON Lines          | 流式序列化、逐行处理                            | 不作为唯一在线查询数据库         |
| SQLite FTS5         | 可重建全文索引                               | 不将搜索索引作为权威内容         |

参考资料：

- OpenCode V2：[session/sql.ts](https://github.com/anomalyco/opencode/blob/dev/packages/core/src/session/sql.ts)、[session-message.ts](https://github.com/anomalyco/opencode/blob/dev/packages/schema/src/session-message.ts)
- Codex：[Rollout JSONL 格式](https://developers.openai.com/codex)
- [CloudEvents Specification](https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md)
- [OpenTelemetry Log Data Model](https://opentelemetry.io/docs/specs/otel/logs/data-model/)
- [Matrix Client-Server API](https://spec.matrix.org/v1.19/client-server-api/)
- [Jupyter Notebook Format](https://nbformat.readthedocs.io/en/latest/format_description.html)
- [Git Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects.html)
- [JSON Lines](https://jsonlines.org/)
- [SQLite WAL](https://www.sqlite.org/wal.html)
- [SQLite FTS5](https://www.sqlite.org/fts5.html)

---

# 3. ID、时间与排序规则

这是整个设计的基础。

## 3.1 ID

所有业务对象使用 UUIDv7。

适用对象：

- workspace_id
- session_id
- run_id
- model_call_id
- message_id
- part_id
- tool_invocation_id
- event_id
- checkpoint_id
- snapshot_id
- archive_id

示例：

`019a1234-5678-7abc-8def-0123456789ab`

UUIDv7 提供基于时间的排序特征，但不能把它当作会话内唯一顺序的保证。

参考 RFC 9562。

## 3.2 Seq

每个 Session 独立使用两个递增序列：

- message_seq
- event_seq

从 1 开始，永不复用。

Message 使用：

`UNIQUE(session_id, seq)`

Event 使用：

`UNIQUE(session_id, seq)`

这里两个 seq 位于不同的命名空间，不可混用。

排序和分页一律以 seq 为准。

禁止依赖：

- UUID 排序
- 文件修改时间
- 消息创建时间
- 导入来源的原始时间戳

## 3.3 时间戳

数据库统一使用 UTC Unix Epoch 毫秒：

`INTEGER`

字段统一使用 `_at_ms` 后缀。

例如：

- created_at_ms
- updated_at_ms
- started_at_ms
- completed_at_ms
- observed_at_ms

导入外部消息时，分别保留：

- source_created_at_ms：原始数据中的时间
- created_at_ms：当前系统实际写入时间

两者不可混为一谈。

---

# 4. SQLite 数据库结构

以下为建议的 V1 DDL。Agent 应根据项目现有的 ORM、数据库封装和 SQLite 版本调整语法，但不得改变字段语义或取消关键约束。

## 4.1 Schema Migration

```
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    checksum TEXT NOT NULL,
    applied_at_ms INTEGER NOT NULL
);
```

要求：

- 每次数据库结构修改都必须通过 Migration。
- Migration 有严格递增版本。
- 已应用 Migration 不允许静默修改。
- 启动时检查 DB 版本。
- 版本不兼容时明确报错，不允许直接覆盖旧数据。

## 4.2 Workspace

```
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    canonical_root TEXT NOT NULL,
    display_name TEXT,
    created_at_ms INTEGER NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    UNIQUE(canonical_root)
);
```

Workspace 代表工作目录或项目身份。

不要让会话只依靠 `cwd` 字符串反查所属项目。

路径规范化由应用层处理，并考虑 Windows 大小写、符号链接和不同平台路径语义。

## 4.3 Session

```
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,

    workspace_id TEXT REFERENCES workspaces(id),
    parent_session_id TEXT REFERENCES sessions(id),
    forked_from_seq INTEGER,

    title TEXT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK(status IN (
            'active',
            'idle',
            'archived',
            'deleted'
        )),

    next_message_seq INTEGER NOT NULL DEFAULT 1,
    next_event_seq INTEGER NOT NULL DEFAULT 1,

    message_count INTEGER NOT NULL DEFAULT 0,
    last_message_seq INTEGER,
    last_message_id TEXT,

    first_user_preview TEXT,
    last_assistant_preview TEXT,

    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    last_activity_at_ms INTEGER,

    retention_policy_json TEXT NOT NULL DEFAULT '{}',
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_sessions_activity
ON sessions(last_activity_at_ms DESC, id);

CREATE INDEX idx_sessions_workspace
ON sessions(workspace_id, last_activity_at_ms DESC);
```

Session 是会话列表的基本读取单位。

打开会话列表时，只查询这个表。

不得为了生成卡片而读取完整历史。

`parent_session_id` 用于记录来源关系，不代表自动继承父会话消息。V1 不实现隐式继承历史的分支机制。

## 4.4 SessionRun

一次完整的 Agent 执行轮次。

```
CREATE TABLE session_runs (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),

    turn_seq INTEGER NOT NULL,

    status TEXT NOT NULL
        CHECK(status IN (
            'queued',
            'running',
            'completed',
            'failed',
            'interrupted',
            'cancelled'
        )),

    agent_id TEXT,
    agent_name TEXT,

    trigger_message_id TEXT,

    started_at_ms INTEGER,
    completed_at_ms INTEGER,

    result_json TEXT,
    error_json TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',

    UNIQUE(session_id, id),
    UNIQUE(session_id, turn_seq)
);

CREATE INDEX idx_runs_session_turn
ON session_runs(session_id, turn_seq DESC);
```

一次 Run 可以有多个 ModelCall。

Run 的状态必须覆盖工具执行期间，不能在第一个模型响应结束时就认为整个执行完成。

## 4.5 ModelCall

每次实际模型调用独立记录。

```
CREATE TABLE model_calls (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    run_id TEXT NOT NULL,

    call_seq INTEGER NOT NULL,

    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,

    status TEXT NOT NULL
        CHECK(status IN (
            'pending',
            'streaming',
            'completed',
            'failed',
            'interrupted',
            'cancelled'
        )),

    provider_request_id TEXT,

    started_at_ms INTEGER,
    first_token_at_ms INTEGER,
    completed_at_ms INTEGER,

    usage_json TEXT,
    error_json TEXT,
    request_metadata_json TEXT NOT NULL DEFAULT '{}',

    UNIQUE(session_id, id),
    UNIQUE(run_id, call_seq),

    FOREIGN KEY(session_id, run_id)
        REFERENCES session_runs(session_id, id)
);

CREATE INDEX idx_model_calls_run
ON model_calls(run_id, call_seq);
```

这样可以保留：

- 不同模型之间的切换。
- 同一 Run 的多轮模型调用。
- 重试记录。
- 每次调用的输入输出 token。
- 首 token 时间。
- 请求失败原因。
- Provider 原生响应 ID。

不要把一次 Run 的模型信息硬编码进 Session。

## 4.6 Message

这是会话历史的核心表。

```
CREATE TABLE messages (
    id TEXT PRIMARY KEY,

    session_id TEXT NOT NULL REFERENCES sessions(id),
    seq INTEGER NOT NULL,

    run_id TEXT,
    model_call_id TEXT,

    role TEXT NOT NULL
        CHECK(role IN (
            'system',
            'developer',
            'user',
            'assistant',
            'tool'
        )),

    speaker_name TEXT,
    tool_call_id TEXT,

    status TEXT NOT NULL
        CHECK(status IN (
            'pending',
            'streaming',
            'completed',
            'interrupted',
            'failed',
            'cancelled'
        )),

    visibility TEXT NOT NULL DEFAULT 'public'
        CHECK(visibility IN (
            'public',
            'internal',
            'hidden'
        )),

    finish_reason TEXT,

    source_system TEXT,
    source_message_id TEXT,
    source_created_at_ms INTEGER,

    idempotency_key TEXT,

    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    completed_at_ms INTEGER,

    metadata_json TEXT NOT NULL DEFAULT '{}',

    UNIQUE(session_id, seq),
    UNIQUE(session_id, id),

    UNIQUE(session_id, idempotency_key),

    FOREIGN KEY(session_id, run_id)
        REFERENCES session_runs(session_id, id),

    FOREIGN KEY(session_id, model_call_id)
        REFERENCES model_calls(session_id, id)
);

CREATE INDEX idx_messages_session_seq
ON messages(session_id, seq DESC);

CREATE INDEX idx_messages_session_role
ON messages(session_id, role, seq DESC);

CREATE INDEX idx_messages_status
ON messages(status)
WHERE status IN ('pending', 'streaming');
```

角色表示谁产生了消息。

不要将消息的实际内容和角色混在一个自由 JSON 中。

`metadata_json` 用来保留非核心扩展信息，不允许把本应有明确类型的核心字段全部塞进去。

已完成消息原则上不可原地改写。重新生成、重试和用户编辑都应保留可追溯记录。

## 4.7 MessagePart

一条 Message 可以拥有多个 Part。

这是多模态和工具交互的关键。

```
CREATE TABLE message_parts (
    id TEXT PRIMARY KEY,

    session_id TEXT NOT NULL,
    message_id TEXT NOT NULL,

    ordinal INTEGER NOT NULL,

    type TEXT NOT NULL,

    schema_version INTEGER NOT NULL DEFAULT 1,

    status TEXT NOT NULL DEFAULT 'completed'
        CHECK(status IN (
            'pending',
            'streaming',
            'completed',
            'failed',
            'interrupted'
        )),

    visibility TEXT NOT NULL DEFAULT 'public',

    text_content TEXT,
    data_json TEXT,

    payload_storage TEXT NOT NULL DEFAULT 'inline'
        CHECK(payload_storage IN (
            'inline',
            'blob',
            'archive'
        )),

    payload_blob_hash TEXT REFERENCES blobs(hash),
    archive_id TEXT REFERENCES session_archives(id),
    archive_record_key TEXT,

    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,

    UNIQUE(message_id, ordinal),
    UNIQUE(session_id, id),

    FOREIGN KEY(session_id, message_id)
        REFERENCES messages(session_id, id)
        ON DELETE CASCADE,

    CHECK (
        (payload_storage = 'inline'
            AND payload_blob_hash IS NULL
            AND archive_id IS NULL)
        OR
        (payload_storage = 'blob'
            AND payload_blob_hash IS NOT NULL
            AND archive_id IS NULL)
        OR
        (payload_storage = 'archive'
            AND payload_blob_hash IS NULL
            AND archive_id IS NOT NULL
            AND archive_record_key IS NOT NULL)
    )
);

CREATE INDEX idx_parts_message_order
ON message_parts(message_id, ordinal);

CREATE INDEX idx_parts_session_message
ON message_parts(session_id, message_id, ordinal);
```

注意：上面依赖先创建 `blobs` 和 `session_archives` 表。

### Part 类型

V1 至少支持：

| Type        | 作用                    |
| ----------- | --------------------- |
| text        | 普通文本                  |
| reasoning   | 模型提供的内部推理类内容，独立控制访问权限 |
| tool_call   | 工具调用请求                |
| tool_result | 工具执行结果                |
| image       | 图片引用                  |
| audio       | 音频引用                  |
| video       | 视频引用                  |
| file        | 文件引用                  |
| json        | 结构化 JSON              |
| citation    | 引用或来源                 |
| refusal     | 拒绝内容                  |
| artifact    | 生成的文件或产物              |
| status      | 进度和状态信息               |
| custom      | 上游扩展类型                |

每个类型有自己的数据 Schema。

例如：

```
{
  "type": "text",
  "schema_version": 1,
  "text": "Hello"
}
```

工具调用：

```
{
  "type": "tool_call",
  "schema_version": 1,
  "tool_call_id": "019a...",
  "name": "read_file",
  "arguments": {
    "path": "src/main.rs"
  }
}
```

图片：

```
{
  "type": "image",
  "schema_version": 1,
  "attachments": [
    {
      "hash": "sha256:...",
      "mime": "image/png",
      "bytes": 123456
    }
  ],
  "width": 1920,
  "height": 1080
}
```

未知 Part 类型必须允许以原始 JSON 保留，禁止导入时静默丢弃。

特别注意：不要把所有多模态内容先转换为 Markdown 或 Base64 文本，再存储到 text_content。

## 4.8 Streaming Chunk

不能每收到一个 token 就执行一次对完整 TEXT 字段的重写。

建议提供临时增量表：

```
CREATE TABLE message_part_chunks (
    part_id TEXT NOT NULL
        REFERENCES message_parts(id)
        ON DELETE CASCADE,

    chunk_seq INTEGER NOT NULL,
    text_content TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,

    PRIMARY KEY(part_id, chunk_seq)
);
```

处理方式：

1. 创建 streaming Message 和 Part。
2. 模型输出先在内存缓冲。
3. 每累计约 4 KiB 或 250 ms，写入一个 chunk。
4. 收到完成事件后，合并成最终 Part。
5. 小于内联阈值的存入 SQLite，大于阈值的转 Blob。
6. 完成后删除临时 chunk。
7. 如果进程崩溃，保留 chunk，恢复成 interrupted 状态。

这些阈值做成配置，并通过压测调整。

## 4.9 Blob

先创建：

```
CREATE TABLE blobs (
    hash TEXT PRIMARY KEY,

    algorithm TEXT NOT NULL DEFAULT 'sha256',

    bytes INTEGER NOT NULL,
    mime TEXT NOT NULL,

    relative_path TEXT NOT NULL UNIQUE,

    created_at_ms INTEGER NOT NULL,
    verified_at_ms INTEGER,

    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_blobs_created
ON blobs(created_at_ms);
```

附件引用关系：

```
CREATE TABLE part_attachments (
    part_id TEXT NOT NULL
        REFERENCES message_parts(id)
        ON DELETE CASCADE,

    ordinal INTEGER NOT NULL,

    blob_hash TEXT NOT NULL
        REFERENCES blobs(hash),

    purpose TEXT NOT NULL DEFAULT 'attachment',

    metadata_json TEXT NOT NULL DEFAULT '{}',

    PRIMARY KEY(part_id, ordinal)
);

CREATE INDEX idx_part_attachments_hash
ON part_attachments(blob_hash);
```

Blob 路径：

```
data/
  blobs/
    sha256/
      ab/
        cd/
          abcdef...64hex
```

不要使用原始文件名作为物理存储路径。

处理 Blob 时：

1. 读取原始字节。
2. 计算 SHA-256。
3. 写入临时文件。
4. 完整写入并验证。
5. 原子重命名至最终哈希路径。
6. 在数据库事务中插入元数据和引用。

相同 SHA-256 的内容只存储一次。

使用引用检查和垃圾回收，不要因为一个 Message 被删除就直接删除共享 Blob。

## 4.10 ToolInvocation

工具调用必须有独立生命周期。

```
CREATE TABLE tool_invocations (
    id TEXT PRIMARY KEY,

    session_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    model_call_id TEXT,

    tool_call_id TEXT NOT NULL,

    assistant_message_id TEXT NOT NULL,
    assistant_part_id TEXT NOT NULL,

    result_message_id TEXT,
    result_part_id TEXT,

    tool_name TEXT NOT NULL,

    status TEXT NOT NULL
        CHECK(status IN (
            'queued',
            'running',
            'succeeded',
            'failed',
            'cancelled',
            'timed_out'
        )),

    input_json TEXT NOT NULL,

    error_json TEXT,

    started_at_ms INTEGER,
    completed_at_ms INTEGER,

    retry_of_id TEXT REFERENCES tool_invocations(id),

    trace_id TEXT,
    span_id TEXT,

    idempotency_key TEXT,

    metadata_json TEXT NOT NULL DEFAULT '{}',

    UNIQUE(session_id, id),
    UNIQUE(session_id, tool_call_id),

    FOREIGN KEY(session_id, run_id)
        REFERENCES session_runs(session_id, id),

    FOREIGN KEY(model_call_id)
        REFERENCES model_calls(id),

    FOREIGN KEY(assistant_message_id)
        REFERENCES messages(id),

    FOREIGN KEY(assistant_part_id)
        REFERENCES message_parts(id),

    FOREIGN KEY(result_message_id)
        REFERENCES messages(id),

    FOREIGN KEY(result_part_id)
        REFERENCES message_parts(id)
);

CREATE INDEX idx_tool_invocations_run
ON tool_invocations(run_id, started_at_ms);

CREATE INDEX idx_tool_invocations_status
ON tool_invocations(status);
```

工具执行过程：

```
Assistant ToolCall
       |
       v
ToolInvocation queued
       |
       v
ToolInvocation running
       |
       +---- succeeded
       |
       +---- failed
       |
       +---- cancelled
       |
       +---- timed_out
```

重要规定：

- 工具开始执行前先提交 running 状态。
- 执行工具时不能一直持有数据库写事务。
- 工具结束后另开短事务提交结果。
- 失败和重试必须保留旧记录。
- 不要覆盖上一次失败的输入和错误。
- 超长输出转 Blob。
- 需要恢复的工具必须具有独立的恢复策略，不能把数据库中的 running 直接当作工具仍在运行。

---

# 5. SessionEvent：事件模型

## 5.1 事件表

```
CREATE TABLE session_events (
    id TEXT PRIMARY KEY,

    session_id TEXT NOT NULL REFERENCES sessions(id),

    seq INTEGER NOT NULL,

    event_type TEXT NOT NULL,
    event_version INTEGER NOT NULL DEFAULT 1,

    run_id TEXT,
    message_id TEXT,

    actor_type TEXT,
    actor_id TEXT,

    occurred_at_ms INTEGER NOT NULL,
    recorded_at_ms INTEGER NOT NULL,

    trace_id TEXT,
    span_id TEXT,
    correlation_id TEXT,

    idempotency_key TEXT,

    payload_json TEXT NOT NULL DEFAULT '{}',

    UNIQUE(session_id, seq),
    UNIQUE(session_id, idempotency_key)
);

CREATE INDEX idx_events_session_seq
ON session_events(session_id, seq DESC);

CREATE INDEX idx_events_type
ON session_events(session_id, event_type, seq);
```

## 5.2 事件类型

至少支持：

```
session.created
session.updated
session.archived

run.started
run.completed
run.failed
run.interrupted

model_call.started
model_call.first_token
model_call.completed
model_call.failed

message.created
message.completed
message.interrupted

tool.started
tool.completed
tool.failed
tool.cancelled

checkpoint.created
checkpoint.completed
checkpoint.failed

snapshot.created

archive.created
archive.verified
archive.restored
```

不要为每个 token 创建一个 Event。

事件粒度应该是有意义的业务状态变化。

Message 和 Part 保存具体内容；SessionEvent 保存发生过什么、何时发生、由谁产生，以及涉及哪些对象。

这是一种混合模型，不是要求所有数据都通过重放事件重建。

## 5.3 CloudEvents 信封

导出或跨进程传输时，建议使用 CloudEvents 风格。

示例：

```
{
  "specversion": "1.0",
  "id": "019a1234-5678-7abc-8def-0123456789ab",
  "source": "urn:harness:session:session-001",
  "type": "harness.tool.completed.v1",
  "subject": "tool/019a1234-...",
  "time": "2026-11-19T00:00:00.000Z",
  "datacontenttype": "application/json",
  "session_seq": 28,
  "data": {
    "tool_call_id": "tool-001",
    "status": "succeeded",
    "duration_ms": 1200
  }
}
```

数据库内继续使用整数毫秒和独立 event_seq。

不要为了符合 CloudEvents 而在数据库中重复保存大量字段。

---

# 6. Model Context 与 Compaction

这是必须独立设计的一层。

绝对不要把上下文压缩等同于历史删除。

## 6.1 ContextCheckpoint

```
CREATE TABLE context_checkpoints (
    id TEXT PRIMARY KEY,

    session_id TEXT NOT NULL REFERENCES sessions(id),

    status TEXT NOT NULL
        CHECK(status IN (
            'pending',
            'running',
            'completed',
            'failed',
            'stale'
        )),

    strategy TEXT NOT NULL,

    covered_from_seq INTEGER NOT NULL,
    covered_through_seq INTEGER NOT NULL,

    raw_keep_from_seq INTEGER NOT NULL,

    summary_text TEXT,

    included_ranges_json TEXT NOT NULL DEFAULT '[]',
    pinned_message_ids_json TEXT NOT NULL DEFAULT '[]',

    source_digest TEXT NOT NULL,

    model_provider_id TEXT,
    model_id TEXT,

    prompt_template_version TEXT,

    token_estimate INTEGER,

    created_at_ms INTEGER NOT NULL,
    completed_at_ms INTEGER,

    error_json TEXT,

    UNIQUE(session_id, id)
);

CREATE INDEX idx_checkpoints_session
ON context_checkpoints(session_id, status, created_at_ms DESC);
```

## 6.2 压缩语义

假设原始消息：

```
1  用户
2  助手
3  用户
4  助手
...
80 用户
81 助手
...
100 用户
```

生成一个 completed checkpoint：

```
{
  "strategy": "summary_recent",
  "covered_from_seq": 1,
  "covered_through_seq": 80,
  "raw_keep_from_seq": 81,
  "summary_text": "此前对话总结……",
  "pinned_message_ids": [
    "important-message-id"
  ]
}
```

模型实际看到：

```
System Instructions
+
Checkpoint Summary
+
Pinned Messages
+
Messages 81..100
+
Current User Input
```

数据库仍然保留 1..100 全部原始消息。

## 6.3 有效性规则

1. 只有 completed checkpoint 可以生效。
2. running、failed、stale 不得改变当前模型上下文。
3. 不得删除 checkpoint 覆盖的原始消息。
4. 保存被总结的完整范围。
5. 保存摘要生成时的模型和模板版本。
6. 保存源消息 digest，确保摘要能够追溯。
7. 如果历史消息被修改，必须检查 digest 并使相关 checkpoint 失效。
8. 最近消息和 pinned 消息必须保留独立的上下文入口。
9. checkpoint 是一个不可变的历史产物，新压缩生成新记录。

不要直接使用 Matrix 的每 100 次状态变化规则。它的压缩对象是房间状态，而不是线性聊天历史。

## 6.4 Run Context Manifest

还需要记录每次 ModelCall 实际向模型发送了哪些内容。

```
CREATE TABLE model_call_context_items (
    model_call_id TEXT NOT NULL
        REFERENCES model_calls(id),

    ordinal INTEGER NOT NULL,

    item_type TEXT NOT NULL,

    message_id TEXT,
    checkpoint_id TEXT,
    snapshot_id TEXT,

    source_ref TEXT,

    content_hash TEXT NOT NULL,

    token_estimate INTEGER,

    included_as TEXT,

    PRIMARY KEY(model_call_id, ordinal)
);
```

`item_type`：

```
system
developer
message
checkpoint_summary
pinned_message
runtime_state
tool_result
other
```

这样可以回答：

- 这个模型请求实际看到了哪些历史？
- 某条旧消息是否被压缩？
- 哪个 checkpoint 提供了摘要？
- 为什么某条信息没有进入上下文？
- 两次请求的上下文为什么不同？

不强制把每次完整 Prompt 的副本永久存储，先保存内容来源、顺序和哈希。需要完整重放时，再按安全策略选择是否保存原文。

---

# 7. Agent Runtime Snapshot

ContextCheckpoint 与 SessionSnapshot 是不同概念。

Checkpoint 是对话历史的上下文压缩。

Snapshot 是 Agent 的运行状态快照。

例如：

- 当前工作目录和项目状态。
- Agent 的任务状态。
- 已完成的子任务。
- 工具恢复信息。
- 当前执行环境描述。
- 工作区文件版本。
- 必须跨重启保留的运行状态。

建议建立：

```
CREATE TABLE session_snapshots (
    id TEXT PRIMARY KEY,

    session_id TEXT NOT NULL REFERENCES sessions(id),

    seq_at INTEGER NOT NULL,

    state_kind TEXT NOT NULL,
    state_schema_version INTEGER NOT NULL,

    encoding TEXT NOT NULL DEFAULT 'json'
        CHECK(encoding IN ('json', 'blob', 'delta')),

    payload_json TEXT,
    payload_blob_hash TEXT REFERENCES blobs(hash),

    base_snapshot_id TEXT REFERENCES session_snapshots(id),

    content_hash TEXT NOT NULL,

    created_at_ms INTEGER NOT NULL,

    metadata_json TEXT NOT NULL DEFAULT '{}'
);
```

V1 只实现完整快照：

- JSON 或 Blob。
- 明确版本。
- 明确 hash。
- 能独立恢复。

`delta` 先保留格式扩展能力，不要立即实现。

将来通过基准测试确认状态重复率高、增量回放确实节省资源后，再实现 Matrix 风格的 delta + 周期全量快照。

---

# 8. Blob 生命周期

## 8.1 文件结构

```
data/
  harness.db

  blobs/
    sha256/
      ab/
        cd/
          abcdef...

  archives/
    <session_id>/
      g000001-000001-000500.jsonl.zst
      g000002-000501-001000.jsonl.zst
      manifest.json

  tmp/
```

## 8.2 Blob 规则

- SHA-256 原始字节寻址。
- 同内容只有一份。
- 通过引用表管理生命周期。
- MIME、尺寸、原始文件名为元数据。
- 数据库不保存大型 Base64。
- Blob 读取先校验其是否被引用。
- GC 只能回收无引用对象。
- 文件写入成功且校验通过后才能建立正式引用。

建议设置一个可配置的内联阈值。

例如：

```
inline_text_max_bytes = 65536
```

大于阈值的长工具输出、文本和二进制内容可以进入 Blob。

不要对所有文本无条件外置。小型文本仍然适合直接在 SQLite 中查询。

---

# 9. JSONL 冷归档

## 9.1 文件格式

文件：

```
g000001-000001-000500.jsonl.zst
```

使用 UTF-8，每行一个有效 JSON 对象。

第一行：

```
{
  "record_type": "archive_header",
  "format": "harness-session",
  "format_version": 1,
  "session_id": "session-001",
  "generation": 1,
  "seq_start": 1,
  "seq_end": 500,
  "created_at_ms": 1795046400000
}
```

普通记录：

```
{
  "record_type": "message",
  "record_version": 1,
  "seq": 15,
  "data": {
    "id": "message-001",
    "role": "assistant",
    "status": "completed"
  }
}
```

Part 记录：

```
{
  "record_type": "message_part",
  "record_version": 1,
  "message_id": "message-001",
  "ordinal": 0,
  "type": "text",
  "data": {
    "text": "Hello"
  }
}
```

工具记录：

```
{
  "record_type": "tool_invocation",
  "record_version": 1,
  "data": {}
}
```

还需要导出：

- Workspace
- Session
- Run
- ModelCall
- SessionEvent
- ContextCheckpoint
- SessionSnapshot
- Blob 引用清单

不要在归档中遗漏工具结果和附件关系。

## 9.2 Manifest

使用单独的 Manifest，不要把整个文件的 SHA-256 放在被哈希的同一个文件尾部。

```
{
  "format": "harness-session",
  "format_version": 1,
  "session_id": "session-001",
  "generation": 1,
  "file": "g000001-000001-000500.jsonl.zst",
  "compression": "zstd",

  "seq_start": 1,
  "seq_end": 500,

  "record_count": 1200,

  "compressed_bytes": 123456,
  "uncompressed_bytes": 987654,

  "compressed_sha256": "...",
  "uncompressed_sha256": "..."
}
```

## 9.3 归档规则

1. 写临时文件。
2. 导出结构化记录。
3. 压缩。
4. 验证 JSONL 每一行。
5. 验证记录数量和序号范围。
6. 计算 SHA-256。
7. 写 Manifest。
8. 原子重命名。
9. 将 archive 元数据提交到数据库。

归档未完成或 Manifest 未通过验证，不得被认为是有效归档。

建议一个 Chunk 控制在约 8 MiB 未压缩大小，达到目标大小或记录数量阈值时生成新 Chunk，实际参数通过压测调整。

普通 Zstandard 压缩流不应假定支持任意位置随机访问。先使用小型独立 Chunk；需要随机访问时，再评估 Zstandard Seekable Frames。

V1 归档是完整副本，不执行删除操作。

未来实现冷热迁移时，必须先确保归档完整、附件齐全、可恢复，然后才允许将数据库中的旧载荷替换为 archive 指针。

---

# 10. SQLite 运行配置

建议：

```
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

要求：

1. 每个数据库连接都启用 foreign_keys。
2. 采用短事务。
3. 采用单写入队列或统一的写入协调器。
4. 读取和工具执行不能长期持有写事务。
5. 定期检查 WAL 大小和 checkpoint 状态。
6. 不允许在网络文件系统上直接使用 WAL 数据库。
7. 不能在数据库处于运行状态时只复制 `.db` 文件而忽略 WAL。
8. 正式备份使用 SQLite Backup API 或受控的一致性快照。

WAL 能让多个读取者与写入者并行工作，但 SQLite 仍然只有一个同时执行的写事务。长时间读取也可能阻碍 checkpoint。

`FULL` 是优先的数据持久性设置；如将来允许 `NORMAL`，必须明确其断电恢复语义和用户接受的风险。

SQLite 官方资料：

[https://www.sqlite.org/wal.html](https://www.sqlite.org/wal.html)

[https://www.sqlite.org/foreignkeys.html](https://www.sqlite.org/foreignkeys.html)

---

# 11. 分页与读取 API

永远不要全量加载会话。

## 11.1 消息分页

首次加载：

```
SELECT *
FROM messages
WHERE session_id = ?
ORDER BY seq DESC
LIMIT 51;
```

取出后在 UI 中反转顺序，并使用额外的一条判断是否还有历史。

读取更旧的消息：

```
SELECT *
FROM messages
WHERE session_id = ?
  AND seq < ?
ORDER BY seq DESC
LIMIT 51;
```

读取更新的消息：

```
SELECT *
FROM messages
WHERE session_id = ?
  AND seq > ?
ORDER BY seq ASC
LIMIT 51;
```

不要使用：

```
OFFSET 50000
```

不要每次滚动都执行：

```
SELECT COUNT(*)
```

使用 Keyset Pagination。

## 11.2 Cursor

外部 API 返回：

```
{
  "items": [],
  "next_cursor": "opaque-token",
  "has_more": true
}
```

Cursor 至少包含：

```
{
  "version": 1,
  "session_id": "session-001",
  "direction": "backward",
  "seq": 100
}
```

最后一页：

```
{
  "items": [],
  "next_cursor": null,
  "has_more": false
}
```

分页终点必须明确。

## 11.3 UI 约束

建议初始窗口 50 条左右。

滚动时按 50～100 条追加。

采用虚拟化列表，不让 DOM 或 UI 组件实例化完整会话。

附件只在进入可视区域或用户主动打开时加载。

Session 卡片只依赖 sessions 表中的摘要字段。

---

# 12. 全文搜索

建立独立的 FTS 索引。

建议：

```
CREATE TABLE message_search_docs (
    rowid INTEGER PRIMARY KEY,
    message_id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL,
    search_text TEXT NOT NULL,
    tokenizer_version TEXT NOT NULL
);

CREATE VIRTUAL TABLE message_fts
USING fts5(
    search_text,
    content='message_search_docs',
    content_rowid='rowid'
);
```

索引内容：

- User 文本
- Assistant 公开文本
- 必要的工具输出
- 可搜索的 Artifact 描述

默认不索引敏感内部内容。

索引通过数据库事务、触发器或可靠的重建任务维护。

FTS 是派生索引，不是权威数据。

特别注意中文：

SQLite FTS5 的默认 unicode61 tokenizer 并不会自动替代中文分词器。

建议：

- 使用应用层中文分词预处理。
- 保存独立的 search_text。
- 记录 tokenizer_version。
- 评估 FTS5 trigram 子串索引。
- 短于三个 Unicode 字符的查询设置备用策略。

不要因为创建了 FTS5 表，就认为中文搜索已经做好。

参考：

[https://www.sqlite.org/fts5.html](https://www.sqlite.org/fts5.html)

---

# 13. 导入与迁移

## 13.1 来源适配器

每个外部 Agent 独立提供 Adapter：

```
Importer
  |
  +-- OpenCodeAdapter
  +-- CodexAdapter
  +-- ClineAdapter
  +-- CursorAdapter
  +-- TraeAdapter
  +-- DSHAdapter
  +-- CustomJSONLAdapter
```

Adapter 职责：

1. 读取来源。
2. 确定会话身份。
3. 提取时间和原始排序。
4. 转换为统一 Message。
5. 转换为统一 Part。
6. 提取 Blob。
7. 保留 Provider Native Metadata。
8. 记录导入映射。
9. 验证数量和内容。

不要让每个 Adapter 直接操作数据库底层表。

统一经过一个 Import Service。

## 13.2 来源身份

建议增加：

```
CREATE TABLE import_mappings (
    id TEXT PRIMARY KEY,

    source_system TEXT NOT NULL,
    source_session_id TEXT NOT NULL,
    source_message_id TEXT NOT NULL,

    target_session_id TEXT NOT NULL,
    target_message_id TEXT NOT NULL,

    source_record_hash TEXT,

    imported_at_ms INTEGER NOT NULL,

    UNIQUE(
        source_system,
        source_session_id,
        source_message_id
    )
);
```

防止重复导入。

同一个来源文件反复扫描不得生成重复会话。

## 13.3 保留原生数据

例如某模型 Provider 返回：

```
{
  "provider_specific_field": "value",
  "native_response_id": "..."
}
```

不要丢弃无法映射的字段。

在相应 Message 或 Part 的 `metadata_json` / `data_json` 中保留，使用来源命名空间隔离。

---

# 14. 崩溃恢复与一致性

系统启动时执行恢复检查。

## 14.1 Streaming Message

如果发现：

```
status = streaming
```

但所属进程已经消失：

- 保留已有 Part。
- 合并已持久化的 Chunk。
- 将 Message 改为 interrupted。
- 将对应 Run 改为 interrupted 或进入恢复流程。
- 不得伪造 completed。

## 14.2 ToolInvocation

如果发现：

```
status = running
```

不能直接假定工具已完成。

必须根据工具类型恢复：

| 工具类型  | 恢复策略         |
| ----- | ------------ |
| 纯查询工具 | 可按安全策略重新执行   |
| 文件写入  | 检查目标结果、操作 ID |
| 网络请求  | 使用幂等键或检查远端状态 |
| Shell | 必须检查进程及执行环境  |
| 不可逆操作 | 不允许未经确认自动重放  |

所有工具调用状态都要有明确终态。

## 14.3 一致性检查

提供命令：

```
harness db check
harness session check <session_id>
harness blob verify
harness archive verify
harness archive restore-test <archive_id>
```

至少检查：

- foreign key integrity
- seq 唯一性
- Part 是否存在所属 Message
- ToolInvocation 是否有对应调用 Part
- Blob hash 是否一致
- Blob 引用是否存在
- Archive 校验和
- Checkpoint 范围是否合法
- Session 计数和预览是否与实际数据一致

发现问题时先报告，自动修复必须具有明确规则。

---

# 15. Retention 与冷热数据策略

不要默认删除历史数据。

建议策略：

| 数据               | 默认策略     |
| ---------------- | -------- |
| 最近消息             | 热数据      |
| 历史 Message 元数据   | 长期保留     |
| 历史文本内容           | 长期保留，可归档 |
| 图片音频等 Blob       | 独立管理     |
| 大型 Tool Output   | 超阈值外置    |
| FTS              | 可重建      |
| Checkpoint       | 长期保留     |
| Runtime Snapshot | 依策略保留    |
| Session Event    | 长期保留     |
| 临时 Chunk         | 完成后清理    |

Retention 分成三个独立操作：

1. Archive：归档。
2. Evict：将冷载荷从热数据库迁移到归档。
3. Delete：真正删除。

不能将三者混为一个操作。

默认只做 Archive。

Evict 和 Delete 必须是独立的、可审计的策略。

---

# 16. API / Repository 层

Agent 不得让 UI 或其他业务模块直接拼接 SQL。

至少实现：

```
createWorkspace()
getWorkspace()

createSession()
getSession()
updateSession()
listSessions()

beginRun()
completeRun()
interruptRun()

beginModelCall()
completeModelCall()
failModelCall()

appendMessage()
appendPart()
appendPartChunk()
finalizeMessage()

beginToolInvocation()
completeToolInvocation()
failToolInvocation()
cancelToolInvocation()

createContextCheckpoint()
getLatestCompletedCheckpoint()

createSessionSnapshot()
restoreSessionSnapshot()

listMessagesBefore()
listMessagesAfter()
getMessage()
getMessageParts()

searchMessages()

storeBlob()
getBlob()
verifyBlob()
collectOrphanBlobs()

exportSessionArchive()
verifySessionArchive()
restoreSessionArchive()

checkDatabaseIntegrity()
```

写入操作尽可能通过服务层的单一事务入口。

UI 只接触稳定的业务 API。

---

# 17. 事务边界

每次消息写入必须原子化。

例如创建 User Message：

```
BEGIN IMMEDIATE

1. 分配 message_seq
2. 插入 Message
3. 插入 Parts
4. 插入 message.created Event
5. 更新 Session 计数
6. 更新 Session 预览
7. 更新最后活动时间

COMMIT
```

模型请求：

```
BEGIN

1. 创建 Run
2. 创建 ModelCall
3. 创建对应 Event
4. 写入 Context Manifest

COMMIT

调用模型（数据库事务之外）

BEGIN

5. 保存 Assistant Message
6. 保存 Parts
7. 更新 ModelCall 状态
8. 更新 Run 状态
9. 更新 Session 预览和统计
10. 写入完成 Event

COMMIT
```

工具调用采用同样的开始、执行、完成两阶段。

不能在外部 API 或 Shell 执行期间一直占有数据库事务。

Seq 的分配需要在短写事务中完成，使用 `BEGIN IMMEDIATE` 和原子更新，不能采用两个进程同时 `SELECT MAX(seq)+1`。

---

# 18. 版本兼容

分开管理四种版本：

1. SQLite Schema Version
2. Archive Format Version
3. Event Version
4. Message / Part Schema Version

例如：

```
{
  "type": "tool_call",
  "schema_version": 2
}
```

升级时：

- 已有数据可以逐步迁移。
- 新旧格式允许存在过渡期。
- 未知类型保留原样。
- 迁移失败回滚。
- 不允许通过重新导入旧文件覆盖已有的新数据。
- 保留导入来源信息。

V1 不要尝试一次定义所有未来会出现的 Part 类型。

使用稳定核心类型 + 可扩展类型空间。

---

# 19. 实施顺序

不要一次性实现所有可能的高级优化。

按以下阶段推进，每阶段交付代码和测试。

## P0：规范数据模型

必须完成：

- Migration 系统
- Workspace
- Session
- Run
- ModelCall
- Message
- MessagePart
- ToolInvocation
- SessionEvent
- 外键、唯一性约束
- Seq 分配器

验收：重启后数据完整；多进程写入不会产生重复序号。

## P1：性能与多模态

实现：

- BlobStore
- SHA-256 去重
- Part Attachments
- Streaming Chunk
- Keyset Pagination
- Session 列表投影
- FTS5
- 中文检索策略

验收：大型附件不会进入 Message JSON；加载历史不会全量查询。

## P2：上下文与恢复

实现：

- ContextCheckpoint
- Run Context Manifest
- SessionSnapshot
- 崩溃恢复
- ToolInvocation 恢复策略
- 运行状态检查

验收：压缩失败不影响历史；恢复后能准确解释一次模型请求使用了哪些历史内容。

## P3：归档和导入

实现：

- JSONL Export
- Zstandard
- Manifest
- SHA-256 校验
- Archive Restore
- Codex / OpenCode / DSH 导入适配器
- Import Mapping

验收：导出后清空测试数据库，再从 Archive 恢复，校验消息、Parts、事件和 Blob 全部一致。

## P4：冷热分层

仅在压测证明有必要时实现：

- Archive Payload Locator
- 冷载荷按需读取
- 热数据迁移
- 归档 GC
- 备份和恢复策略

不得因为预先设计了 Archive 就立即删除 SQLite 历史。

---

# 20. 必须建立的压力测试

不能只用几十条消息验证。

建立测试数据集：

| 测试      | 数据规模              |
| ------- | ----------------- |
| 普通会话    | 100 条消息           |
| 长会话     | 100,000 条消息       |
| 超长会话    | 1,000,000 条消息     |
| 多 Parts | 单条消息 100 个 Parts  |
| 超长工具输出  | 10 MiB            |
| 多模态附件   | 1,000 个附件         |
| 多会话     | 10,000 个 Session  |
| 并发      | 多个读进程 + 多个写入请求    |
| 归档      | 多代 JSONL.zst      |
| 恢复      | 在写入、执行工具、归档中途模拟崩溃 |

测试项目：

1. 打开 Session 的查询耗时。
2. 加载第一屏消息的耗时。
3. 前后分页耗时。
4. 内存占用。
5. UI 虚拟列表实例数量。
6. Blob 去重率。
7. FTS 搜索耗时。
8. 写入吞吐。
9. WAL 增长速度。
10. 崩溃恢复正确性。
11. Archive 导出、恢复速度。
12. 迁移前后数据一致性。

必须通过查询计划检查：

```
EXPLAIN QUERY PLAN
SELECT *
FROM messages
WHERE session_id = ?
  AND seq < ?
ORDER BY seq DESC
LIMIT 51;
```

确认使用适当索引，而不是扫描整个消息表。

不要预先承诺特定硬件上的毫秒级指标，以真实测试数据确定性能基线。

---

# 21. 安全与数据完整性

1. 导入内容和工具输出全部视为不可信数据。
2. 不允许将 JSON 中的字段解释为可执行指令。
3. 不在普通日志中保存 API Key、密码、认证 Token。
4. 敏感字段按策略脱敏，必须保留原文时采用独立加密存储。
5. Blob 路径由哈希生成，不允许目录穿越。
6. MIME、文件大小、实际内容需要验证。
7. 不允许通过用户提供的文件名直接覆盖现有 Blob。
8. 文件恢复必须进行哈希校验。
9. 搜索索引和摘要不能代替原始数据。
10. 数据删除必须能处理附件共享引用和归档副本。

---

# 22. 不允许的实现方式

明确禁止：

- 一个会话一个无限增长的 JSON 文件作为唯一数据库。
- 直接使用会话 ID + 时间戳作为唯一排序方式。
- 打开 Session 时 SELECT 全部 Message。
- 每条 Message 把所有多模态数据 Base64 塞进 JSON。
- 把所有工具执行结果拼进 Assistant 的一个 Text 字段。
- 把每次模型调用等同于一轮 Agent 执行。
- 用摘要覆盖原始聊天历史。
- 将 FTS 索引作为权威记录。
- 将归档文件和 SQLite 同时视为可以随意修改的权威数据。
- 使用 OFFSET 进行长期历史分页。
- 忽略部分成功的导入。
- 把失败的 Compaction 当成有效上下文边界。
- 在工具执行期间长时间锁定数据库。
- 未验证 Blob 就直接建立数据库引用。
- 未验证归档就删除原始数据。

---

# 23. 最终交付物

Agent 必须交付：

1. 完整的数据结构设计文档。
2. SQL Migration。
3. 数据库连接和事务管理模块。
4. Session Repository。
5. Message / Part Repository。
6. Run / ModelCall / ToolInvocation Repository。
7. BlobStore。
8. ContextCheckpoint Service。
9. SessionSnapshot Service。
10. Keyset Pagination。
11. FTS 搜索模块。
12. Archive Export / Import。
13. 数据完整性检查工具。
14. 压力测试和恢复测试。
15. 每个字段的含义、约束、生命周期文档。

实施完成后提供：

- 实际创建的表清单。
- ER 图。
- Migration 版本。
- 关键索引及查询计划。
- 压力测试结果。
- 崩溃恢复测试结果。
- 未实现的 P3/P4 功能和原因。

**不要只生成 SQL 后就宣布完成。必须将数据模型接入实际的 Harness 消息写入、模型调用、工具调用和读取链路。**

---

# 24. 最终架构决策

V1 的权威数据：

- SQLite 中的 Sessions、Messages、Parts、Runs、ToolInvocations 和 Events。
- BlobStore 中经过哈希验证的附件原始字节。

V1 的派生数据：

- FTS。
- Session 卡片摘要。
- Checkpoint。
- Runtime Snapshot。
- Archive。

Archive 是经过验证的不可变副本；第一阶段不承担唯一数据来源的职责。

未来可以逐步把冷数据从 SQLite 移到 Archive，但必须保留数据库内的元数据、顺序和可定位引用。

**最终原则：在线查询采用关系模型，历史记录具有追加式特征，内容采用类型化结构，大载荷采用内容寻址，上下文采用独立快照，冷数据采用自描述归档。**

这六种设计组合起来，才是适合长期运行的 Harness Session Format。

## **我认为这套方案里最值得坚持的几个设计**

上面的规格已经包含完整的实施任务。这里再强调五个架构决策，它们会直接决定以后修改成本。

### **1. Message、Run、ModelCall 必须分开**

**User Message**

**Run 001**

一次 Agent 执行

**ModelCall 001**

模型请求

**ToolInvocation 001**

工具执行

**ModelCall 002**

工具结果后的模型请求

**ToolInvocation 002**

第二次工具执行

**Assistant Message**

以后无论是多 Agent、模型切换、工具链、重试还是失败恢复，都不会被一个简单的 message 表限制住。

### **2. 不要过早采用完全 Event Sourcing**

这里我特意选择了混合模型。

原因是聊天内容、工具输出和附件是非常大的数据对象。假如每次修改 Part 都产生一条完整事件，数据重复量会很快增加。

让 Message 和 Part 保存规范内容，让 SessionEvent 保存重要状态变更，已经能满足大部分追溯需求。

将来确实需要完全重放时，可以为特定子系统引入更严格的事件溯源，而不必让整个聊天数据库承担这个复杂度。

### **3. Snapshot 和 Compaction 是两件事**

Agent 的运行状态不能只依赖聊天摘要恢复。

例如，聊天中说过：

> 已经修改了三个文件，第二个文件尚未测试。

这不是一个可靠的运行状态数据库。

需要单独保留文件版本、任务状态、执行进度等 Snapshot，并通过 Run Context Manifest 记录其是否真的交给了模型。

### **4. 真正解决膨胀的是数据生命周期，不只是压缩**

我建议采用：

- 小文本留 SQLite。
- 大文本按需转 Blob。
- 二进制载荷独立 Blob。
- 历史完整保留。
- 旧数据可以归档。
- 搜索索引随时可以重建。

尤其不要因为 Zstandard 压缩率很好，就把所有在线查询都变成解压 JSONL。

### **5. 暂时不要实现 Matrix 式 delta 压缩**

这个方案已经为未来 Snapshot delta 留出了字段。

但 V1 先用完整快照。要等实际测出：

- 状态重复比例；
- 完整快照体积；
- 差异大小；
- 重建耗时；
- 故障恢复成本；

再决定是否引入增量链。

否则会提前引入父快照缺失、增量链损坏、周期性重基等问题。

## **给 Agent 的实施要求**

把上面的整份规格直接交给 Agent，并特别强调：**先检查现有 Harness 的代码和数据流，再按 P0 → P4 实施。不要在不了解现有结构时另起一个与现有运行链路脱节的数据库。**

完成后最重要的不是表有多少，而是三个端到端测试：

1. 一个包含几万条消息、多个工具调用和多模态附件的会话，能够快速分页和搜索。
2. 在模型生成和工具执行过程中强制杀进程，重启后能正确恢复状态。
3. 对会话完成压缩、归档、数据库重建，仍然能够还原原始历史和附件。

这三项通过，才说明 HSF 真正建立起来了。