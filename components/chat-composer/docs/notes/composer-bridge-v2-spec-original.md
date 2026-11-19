喵。

# **结论：应该做四层模型隔离，而且要一次把整个 Bridge 的泄漏点查完**

你的判断是正确的。不过我检查了仓库当前源码后，发现问题不止你指出的两个。

**目前至少有四处跨层耦合：**

| 位置                                        | 当前问题                                    | 应有的设计                  |
| ----------------------------------------- | --------------------------------------- | ---------------------- |
| `SnapshotDto.state`                       | 直接使用 `ScopeState`                       | 独立的 `ScopeSnapshotDto` |
| `EffectDto::Persist`                      | 直接传递 `ScopeState`                       | 独立的持久化序列化载荷            |
| `EffectDto::Deliver` / `EffectOutcomeDto` | 直接使用 Core 的 `Delivery`、`DeliveryResult` | 独立的协议枚举及回执 DTO         |
| `ComposerEventDto::Edit`                  | 只有 `text`                               | 完整的结构化内容 DTO           |

另外，WASM 构造函数目前直接通过 `serde_json::from_str::<ScopeState>` 恢复持久化数据。这同样让持久化格式绑定了 Core 内部结构。

源码位置：

- [composer-bridge-protocol/src/lib.rs](https://github.com/takomitkm/trustyourharness/blob/main/components/chat-composer/crates/composer-bridge-protocol/src/lib.rs?utm_source=chatgpt.com)
- [composer-core/src/state.rs](https://github.com/takomitkm/trustyourharness/blob/main/components/chat-composer/crates/composer-core/src/state.rs?utm_source=chatgpt.com)
- [composer-bridge-wasm/src/lib.rs](https://github.com/takomitkm/trustyourharness/blob/main/components/chat-composer/crates/composer-bridge-wasm/src/lib.rs?utm_source=chatgpt.com)
- [app-react/types.ts](https://github.com/takomitkm/trustyourharness/blob/main/components/chat-composer/app-react/types.ts?utm_source=chatgpt.com)
- [D002 协议设计决策](https://github.com/takomitkm/trustyourharness/blob/main/components/chat-composer/docs/goal-log/decisions/D002-bridge-protocol-design.md?utm_source=chatgpt.com)

还有一个值得顺手修复的细节：

当前 `ack()` 先执行 `pending.remove(&ref_id)`，然后才验证回执类型。如果某次回执类型错误，引用已经被删除，正确回执之后也无法再处理。

这不是 DTO 问题，但属于 Bridge 边界可靠性问题，应一并修复。

## **一、先确定最终架构**

我建议把四层模型定义成下面这样：

**L4 · Editor Model**

TypeScript / Lexical

`LexicalDocumentV1`

编辑器自己的节点树、格式、附件节点、引用节点、编辑器版本。管理编辑器实例、选区、撤销和 IME。

Editor Adapter：显式双向转换

**L1 · Domain Model**

Rust Core

`ComposerContent` / `ScopeState`

真正的业务语义：正文、顺序、附件身份、引用身份、提交记录和运行状态。

Persistence Codec

**L2 · Persistence Model**

Rust

`PersistedScopeV1`

数据库、磁盘、迁移、归档格式。

Protocol Mapper

**L3 · Bridge DTO**

Rust

`SnapshotDtoV2`

`EventDtoV2`

`EffectDtoV2`

注意：图中的四层不是要求每个模型都复制完整数据，而是要求每层拥有自己的语义、版本和生命周期。

例如，同一个附件在四层里可以分别表现为：

- Editor：一个 Lexical 附件节点。
- Domain：一个带稳定 AssetId 的附件引用。
- Persistence：一个附件元数据记录，以及一个 Blob 存储引用。
- Bridge：一个传输用的附件描述 DTO。

它们引用的是同一逻辑资产，但不是同一种数据结构。

**尤其不要让 Lexical JSON 直接成为 Rust Domain Model，也不要让 Rust Domain Model 直接成为 JSON 协议。**

## **二、交给 Agent 的完整实施规格**

下面这份可以直接作为任务交付。重点是让 Agent 改架构而不是简单地把 `ScopeState` 复制成另一个名字。

# Composer 四层数据模型隔离与 Bridge V2 重构

**目标仓库：** `takomitkm/trustyourharness`

**主要范围：** `components/chat-composer/`

**任务性质：** 架构重构，不是简单 DTO 改名。

**核心原则：**

内部领域模型、持久化模型、跨边界 DTO、编辑器文档模型分别定义。每层拥有独立类型、版本、校验规则和生命周期，通过显式 Mapper 转换。

不得直接传递内部 Rust 结构体作为公开协议，不得将 Lexical JSON 直接作为 Domain Model，不得让持久化格式依赖 Core 内部字段布局。

---

# 1. 开始前：审计当前代码

先完整检查：

```
crates/composer-core/src/state.rs
crates/composer-core/src/ids.rs

crates/composer-runtime/src/lib.rs

crates/composer-bridge-protocol/src/lib.rs
crates/composer-bridge-wasm/src/lib.rs

app-react/types.ts
app-react/snapshots.ts
app-react/src/generated/protocol.ts

lexical-island/src/create-controller.ts
app-web/src/main.rs

docs/goal-log/decisions/D002-bridge-protocol-design.md
docs/goal-log/decisions/D009-island-draft-sync.md
docs/architecture/composer-asset-inventory.md
docs/architecture/host-adapters.md
PRODUCT-LINE-PLAN.md
```

检查所有调用点，不要只改上述文件。

重点确认：

1. 哪些地方直接序列化 ScopeState。
2. 哪些地方直接使用 composer-core 的 ID、Delivery、DeliveryResult。
3. 哪些地方消费 SnapshotDto。
4. 哪些地方通过 Persist Effect 保存状态。
5. app-web 当前编辑器事件的实际数据形状。
6. app-react 的完整 Lexical 快照与附件数据如何映射。
7. WASM 构造和恢复路径如何读取旧状态。
8. TypeScript 协议文件是否与 Rust JSON 实际一致。

**重要：当前 Plan A+ 是 Dioxus-first。app-react 是参考实现和测试资产，不要将其误认为当前唯一产品 UI。**

目前 D009 的主线仍然是 `document-changed → text → Snapshot::text`。它尚未实现完整富文本、附件和引用的 Bridge 同步。

本任务应先完成模型设计及转换路径，再接入各个 UI。不要假设附件链路已经完整。

输出：

`docs/architecture/composer-model-boundaries-audit.md`

列出当前每一个跨层数据传递位置、类型和调用方。

---

# 2. L1：定义 Domain Model

位置：

```
crates/composer-core/src/
  ids.rs
  content.rs
  document.rs
  asset.rs
  reference.rs
  state.rs
```

可以依据实际 Cargo Workspace 合并部分模块，但领域模型必须保持清晰边界。

## 2.1 Domain Model 的职责

Domain Model 表达业务意义，不表达：

- SQLite 表结构。
- IndexedDB ObjectStore。
- JSON 协议字段。
- Lexical Node 类型。
- 浏览器 Blob。
- JavaScript object URL。
- 文件系统路径。
- 数据库 Schema Version。

Core 只关心：

- 这份草稿包含什么内容。
- 正文的顺序是什么。
- 哪些附件参与消息。
- 哪些外部引用参与消息。
- 哪条消息已被交接。
- 消息的投递和恢复状态是什么。

Core 不负责读取附件字节。

## 2.2 ID 必须独立类型化

增加：

```
pub struct AssetId(String);
pub struct ReferenceId(String);
pub struct DocumentNodeId(String);
pub struct DraftGenerationId(String);
```

保留既有：

```
ScopeId
MessageId
AttemptId
TurnId
GenerationId
```

如需兼容既有 `GenerationId`，先确认它代表 Draft Generation 还是其他身份，不要机械重复创建同义类型。

原则：

- MessageId 不可代替 AssetId。
- AssetId 不可代替 DocumentNodeId。
- ReferenceId 不可代替 DocumentNodeId。
- BlobHash 不可代替 AssetId。
- DraftRevision 不可代替 ScopeRevision。
- BridgeRevision 不可代替 StoreRevision。

所有 ID 应有唯一的构造、校验和显示方法。

不能通过任意 `String::new()` 绕过校验后直接进入业务状态。

## 2.3 定义 ComposerContent

将目前的：

```
pub struct Snapshot {
    pub text: String,
    pub attachment_ids: Vec<String>,
    pub reference_ids: Vec<String>,
}
```

升级为完整内容模型。

推荐：

```
pub struct ComposerContent {
    pub document: ComposerDocument,
    pub assets: Vec<AssetDescriptor>,
    pub references: Vec<ReferenceDescriptor>,
}
```

其中：

```
pub struct ComposerDocument {
    pub version: u32,
    pub blocks: Vec<BlockNode>,
}
```

BlockNode：

```
pub enum BlockNode {
    Paragraph {
        id: DocumentNodeId,
        children: Vec<InlineNode>,
    },

    Heading {
        id: DocumentNodeId,
        level: u8,
        children: Vec<InlineNode>,
    },

    Quote {
        id: DocumentNodeId,
        children: Vec<BlockNode>,
    },

    Custom {
        id: DocumentNodeId,
        namespace: String,
        node_type: String,
        version: u32,
        data: JsonValue,
    },
}
```

InlineNode：

```
pub enum InlineNode {
    Text {
        id: DocumentNodeId,
        text: String,
        marks: Vec<TextMark>,
    },

    HardBreak {
        id: DocumentNodeId,
    },

    Mention {
        id: DocumentNodeId,
        reference_id: ReferenceId,
        label: String,
    },

    Attachment {
        id: DocumentNodeId,
        asset_id: AssetId,
    },

    Custom {
        id: DocumentNodeId,
        namespace: String,
        node_type: String,
        version: u32,
        data: JsonValue,
    },
}
```

`TextMark` 至少预留：

```
pub enum TextMark {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
}
```

实际支持范围以现有编辑器功能为准。不要为了预留而强行实现完整富文本排版系统。

## 2.4 AssetDescriptor

```
pub struct AssetDescriptor {
    pub id: AssetId,

    pub name: String,
    pub media_type: String,
    pub size_bytes: u64,

    pub alt: Option<String>,
    pub send_as_file: bool,

    pub availability: AssetAvailability,
}
```

状态：

```
pub enum AssetAvailability {
    Available,
    Missing,
}
```

必要时扩展状态，但不要把上传状态与本地文件可用状态混为一个字段。

注意：

Domain 中的 AssetDescriptor 是逻辑附件身份与元数据，不包含实际 Blob 字节，也不包含本地路径。

AssetId 与 SHA-256 BlobHash 是不同概念：

- AssetId：本次会话内的逻辑附件身份。
- BlobHash：内容寻址的物理数据身份。

相同文件可以被多个 AssetId 引用，但底层字节允许去重。

## 2.5 ReferenceDescriptor

```
pub struct ReferenceDescriptor {
    pub id: ReferenceId,

    pub kind: String,
    pub label: Option<String>,

    pub value: JsonValue,
}
```

例如代码引用：

```
{
  "id": "ref-001",
  "kind": "code-comment",
  "label": "main.rs:L20",
  "value": {
    "path": "src/main.rs",
    "line": 20
  }
}
```

Reference 不得被简化为一个文本字符串。

保留其结构化身份和数据。

## 2.6 内容校验

提供：

```
impl ComposerContent {
    pub fn validate(&self) -> Result<(), ContentError>;

    pub fn plain_text(&self) -> String;

    pub fn has_content(&self) -> bool;
}
```

必须检查：

1. DocumentNodeId 在文档中唯一且非空。
2. AssetId 在资产列表中唯一。
3. 每一个文档附件节点都引用存在的 AssetId。
4. 每一个 Mention 都引用存在的 ReferenceId。
5. JSON 扩展数据合法、有限深度、无循环。
6. 未知扩展不得静默删除。
7. rail-only 附件合法，即资产可以不出现在正文中。
8. 不允许持久化 object URL 或浏览器 Blob URL。
9. Missing Asset 不得伪装成 Available。

正向引用必须成立，反向引用不强制成立。

不要因为附件未插入正文，就判定该附件无效。

## 2.7 文本语义

取消 `text` 与 `document` 同时作为正文权威来源的设计。

新的规范是：

**Document 是唯一正文事实，plain_text 是派生投影。**

例如：

```
ComposerDocument::plain_text()
```

对于纯文本编辑器：

```
ComposerDocument::from_plain_text(text)
```

生成一个 Paragraph + Text 结构。

不得依靠两个可能不同步的字段决定消息正文。

## 2.8 Draft 和 Submission

将：

```
draft.snapshot
submission.snapshot
```

逐步迁移为：

```
draft.content
submission.content
```

提交之后的 Submission Content 不可被后续编辑器变更修改。

保留 generation/revision 的草稿同步机制。

必要时定义：

```
pub struct DraftVersion {
    pub generation: DraftGenerationId,
    pub revision: u64,
}
```

重新生成、重试和取回必须遵循既有可靠交接语义：

- 重试不篡改已经保存的消息内容。
- 新草稿不引用旧草稿的可变状态。
- 迟到回执不得影响新一代草稿。
- 编辑器异步回写必须进行版本检查。

## 2.9 Domain 不直接充当持久化结构

完成新的 Persistence Codec 后，逐步移除 Domain 类型用于外部 JSON 序列化的直接路径。

领域类型不得为了方便桥接而直接派生协议所需的 Serialize/Deserialize 行为。

如有内部测试或其他合理需求，逐项判断后处理，不能简单删除现有所有 serde 使用造成连锁破坏。

---

# 3. L2：独立 Persistence Model

建议建立：

```
crates/composer-session-format/
  src/
    lib.rs
    envelope.rs
    v1/
      scope.rs
      content.rs
      document.rs
      asset.rs
      submission.rs
    codec.rs
    migration.rs
    validation.rs
```

该 crate 的持久化 Schema 定义不能依赖 composer-core 的内部结构体。

如果实现需要转换 Domain，由上层 Adapter 或 Store Service 完成，不能为了省事反过来让 Core 引用持久化 Schema。

## 3.1 持久化信封

```
pub struct PersistedEnvelopeV1 {
    pub format: String,
    pub format_version: u32,
    pub payload: PersistedScopeV1,
}
```

固定：

```
format = "composer-scope"
format_version = 1
```

这里的版本属于持久化格式，不是 Bridge 协议版本，也不是 Domain 的版本。

## 3.2 PersistedScopeV1

独立定义：

```
pub struct PersistedScopeV1 {
    pub scope_id: String,

    pub store_revision: u64,

    pub draft: PersistedDraftV1,

    pub submissions: Vec<PersistedSubmissionV1>,

    pub active_turn_id: Option<String>,

    pub finished_turn_ids: Vec<String>,

    pub pending_receipts: Vec<PersistedReceiptV1>,

    pub created_at_ms: u64,
    pub updated_at_ms: u64,

    pub extensions: JsonObject,
}
```

其中：

```
pub struct PersistedDraftV1 {
    pub generation_id: String,
    pub draft_revision: u64,
    pub content: PersistedComposerContentV1,
}
```

Submission 必须保留：

- 原始消息 ID。
- Scope ID。
- 原始 Generation。
- 完整内容。
- Delivery 模式。
- 轮次关系。
- 投递状态。
- 当前 Attempt。
- 重试次数。
- 收到的回执。
- 创建时间。
- 失败原因。

PendingReceipt 也必须保留完整的回执数据，不能只保存 count。

## 3.3 PersistedComposerContentV1

独立定义自己的持久化结构。

不要：

```
type PersistedComposerContentV1 = ComposerContent;
```

也不要：

```
pub struct PersistedScopeV1 {
    pub state: ScopeState,
}
```

可以采用相似字段，但必须有独立结构和显式映射。

例如：

```
pub struct PersistedComposerContentV1 {
    pub document: PersistedDocumentV1,
    pub assets: Vec<PersistedAssetV1>,
    pub references: Vec<PersistedReferenceV1>,
}
```

文档节点、资产元数据和引用都必须保留其版本。

## 3.4 Blob 不进入 Scope JSON

Persistence Model 中的附件记录只保存引用：

```
{
  "id": "asset-001",
  "blob_hash": "sha256:abcdef...",
  "name": "diagram.png",
  "media_type": "image/png",
  "size_bytes": 12345
}
```

真实字节放独立 BlobStore。

逻辑资产与 Blob 关系独立管理。

## 3.5 Codec

必须提供：

```
encode_scope(
    state: &ScopeState
) -> Result<Vec<u8>, PersistenceError>;

decode_scope(
    bytes: &[u8]
) -> Result<ScopeState, PersistenceError>;
```

以及可测试的：

```
domain_to_persisted(...)
persisted_to_domain(...)
```

要求：

- 序列化不依赖 Domain 字段布局。
- 解码先识别格式和版本。
- 不支持的版本明确报错。
- 校验所有身份和引用。
- 不静默删除未知扩展。
- 迁移失败不覆盖旧数据。

## 3.6 旧数据迁移

目前存在直接持久化 `ScopeState` 的旧格式。

实现专门的：

```
LegacyScopeStateV1
```

迁移规则：

| 旧字段              | 新字段                          |
| ---------------- | ---------------------------- |
| `snapshot.text`  | Paragraph/Text               |
| `attachment_ids` | AssetDescriptor 引用           |
| `reference_ids`  | ReferenceDescriptor 的旧身份保留记录 |
| draft generation | DraftGenerationId            |
| draft revision   | DraftVersion                 |
| submissions      | 新 PersistedSubmission        |
| pending_receipts | 新 PersistedReceipt           |
| active_turn      | active_turn_id               |
| finished_turns   | finished_turn_ids            |

旧记录中没有的附件元数据和引用详情不能伪造。

需要从原有 AssetStore 尝试恢复。

无法恢复的附件应标记 Missing，并保留原 AssetId。

无法恢复详情的引用必须保留原 ID 和未解析状态，不能静默删除。

迁移必须：

1. 保留原始数据。
2. 新格式写入独立位置。
3. 验证新旧消息数量。
4. 验证资产身份和引用关系。
5. 验证恢复结果。
6. 完成后才允许切换读取路径。

不要在首次读取时直接原地覆盖旧格式。

## 3.7 Migration 与 Archive 分离

Persistence Schema Version 和 Archive Format Version 独立管理。

归档可以包含某个版本的规范记录，但归档格式不应等于数据库内部表结构。

未来修改 SQLite 表时，不需要修改外部 Archive 的 JSONL 结构。

---

# 4. L3：Bridge DTO V2

位置：

```
crates/composer-bridge-protocol/src/
  v2/
    mod.rs
    common.rs
    event.rs
    snapshot.rs
    effect.rs
    outcome.rs
    content.rs
    mapper.rs
```

当前 v1 JSON 形状已由 D002/BM05 冻结。

不得在保留 `PROTOCOL_VERSION = 1` 的情况下悄悄改变 JSON 结构。

建立 V2。

## 4.1 协议版本

```
pub const PROTOCOL_VERSION: u32 = 2;
```

新请求统一使用：

```
pub struct BridgeRequestDto {
    pub protocol_version: u32,
    pub request_id: String,
    pub payload: ComposerEventDtoV2,
}
```

回执请求：

```
pub struct AckRequestDto {
    pub protocol_version: u32,
    pub ref_id: u64,
    pub outcome: EffectOutcomeDtoV2,
}
```

所有入口都检查版本。

不支持时返回明确的 ProtocolVersionMismatch。

## 4.2 完全独立的 ComposerContentDto

```
pub struct ComposerContentDto {
    pub document: ComposerDocumentDto,
    pub assets: Vec<AssetDto>,
    pub references: Vec<ReferenceDto>,
}
```

Document、Asset、Reference DTO 均在协议层独立定义。

Rust 内部类型只能经过 Mapper 转换后进入 DTO。

例如：

```
impl TryFrom<ComposerContentDto> for ComposerContent {
    type Error = MappingError;

    fn try_from(
        value: ComposerContentDto
    ) -> Result<Self, Self::Error> {
        // 字段映射、ID 校验、结构校验
    }
}
```

输出同样使用显式转换。

禁止使用：

```
pub type ComposerContentDto = ComposerContent;
```

禁止在 DTO 的字段中直接放 `ScopeId`、`MessageId` 或内部枚举。

DTO 中使用明确的字符串身份和协议枚举。

## 4.3 ComposerEventDtoV2

UI 可发送的事件：

```
pub enum ComposerEventDtoV2 {
    Edit {
        base_draft: DraftVersionDto,
        content: ComposerContentDto,
    },

    Submit {
        base_draft: DraftVersionDto,
        content: ComposerContentDto,
    },

    SetBusyMode {
        mode: BusyEnterDto,
    },

    BeginTurn {
        turn_id: String,
    },

    EndTurn {
        turn_id: String,
    },

    Restore {
        message_id: String,
    },

    Retry {
        message_id: String,
    },

    Remove {
        message_id: String,
    },

    Flush,
    Resume,
    Close,
}
```

DraftVersionDto：

```
pub struct DraftVersionDto {
    pub generation_id: String,
    pub revision: u64,
}
```

Submit 必须携带被提交的完整内容。

原因：不能依赖此前某个 `document-changed` 事件已经成功传达到 Core，否则可能出现编辑器已经更新、Rust Core 却仍持有旧文本的情况。

Core 在处理 Submit 时：

1. 验证 DraftVersion。
2. 验证内容。
3. 将最新内容应用到当前 Draft。
4. 原子交接到 Submission。
5. 创建新的空 Draft。
6. 发出 Persist Effect。

这六步必须维持既有可靠交接纪律。

如果出于兼容考虑保留内部 `Event::Submit`，可增加 `SubmitWithContent`，但不要让 Bridge V2 的正式编辑器入口继续依赖旧的文本状态。

UI 仍不得提交回执类型事件。

## 4.4 SnapshotDtoV2

禁止：

```
pub struct SnapshotDto {
    pub state: ScopeState,
}
```

改成：

```
pub struct SnapshotDtoV2 {
    pub protocol_version: u32,

    pub revision: u64,

    pub runtime: RuntimeViewDto,

    pub scope: ScopeSnapshotDto,
}
```

RuntimeViewDto：

```
pub struct RuntimeViewDto {
    pub admitting: bool,
    pub paused: bool,
    pub closed: bool,

    pub busy_mode: BusyEnterDto,

    pub pending_receipt_count: usize,
}
```

ScopeSnapshotDto：

```
pub struct ScopeSnapshotDto {
    pub scope_id: String,

    pub draft: DraftViewDto,

    pub submissions: Vec<SubmissionViewDto>,

    pub active_turn_id: Option<String>,

    pub finished_turn_ids: Vec<String>,
}
```

DraftViewDto：

```
pub struct DraftViewDto {
    pub generation_id: String,
    pub revision: u64,

    pub content: ComposerContentDto,
}
```

SubmissionViewDto：

```
pub struct SubmissionViewDto {
    pub message_id: String,

    pub content: ComposerContentDto,

    pub status: SubmissionStatusDto,

    pub delivery: DeliveryDto,

    pub attempt_id: Option<String>,

    pub attempts: u32,

    pub created_at_ms: u64,

    pub reason: Option<String>,

    pub receipt: Option<ReceiptDto>,
}
```

Snapshot 只暴露 UI 和 Host 确实需要的内容。

不要暴露：

- PendingReceipt 内部队列。
- 内部 Ledger 结构。
- Domain State Schema。
- 持久化版本字段。
- 内部容器结构。
- 不必要的运行时实现细节。

## 4.5 DeliveryDto

禁止直接使用：

```
pub delivery: Delivery
```

定义：

```
pub enum DeliveryDto {
    Start,
    Steer,
}
```

显式映射内部 Delivery。

## 4.6 ReceiptDto

```
pub struct ReceiptDto {
    pub message_id: String,
    pub scope_id: String,
    pub turn_id: String,
}
```

不得直接传递内部 Receipt。

## 4.7 DeliveryResultDto

禁止：

```
pub enum EffectOutcomeDto {
    Delivered {
        result: DeliveryResult
    }
}
```

定义：

```
pub enum DeliveryResultDto {
    Accepted {
        receipt: ReceiptDto,
    },

    Rejected {
        reason: String,
    },

    Uncertain {
        reason: String,
    },
}
```

然后：

```
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
```

映射时验证：

- receipt.message_id 与本次 Effect 的 MessageId 一致。
- receipt.scope_id 与当前 Scope 一致。
- receipt.turn_id 合法。
- 内部 ID 转换成功。
- 状态变化满足 Core Ledger 规则。

不能因为 DTO JSON 合法，就认为回执在业务上合法。

## 4.8 EffectDtoV2

Effect 类型独立定义。

```
pub enum EffectDtoV2 {
    Persist {
        expected_revision: u64,
        next_revision: u64,
        payload_format: String,
        payload: String,
    },

    Deliver {
        message_id: String,
        attempt_id: String,
        delivery: DeliveryDto,
        turn_id: Option<String>,
        content: ComposerContentDto,
    },

    ScheduleTimeout {
        message_id: String,
        attempt_id: String,
        deadline_ms: u64,
    },

    CancelTimeout {
        message_id: String,
        attempt_id: String,
    },

    Lookup {
        message_id: String,
    },

    CancelAllLookups,
}
```

Persist 的 payload 是 Persistence Codec 产生的独立持久化序列化结果。

Bridge 只负责传递，不暴露 ScopeState。

Host 只负责 CAS 存储 payload，不需要知道 Domain 内部结构。

`payload_format` 应明确标记格式和版本，例如：

```
application/vnd.harness.composer-scope+json;version=1
```

原有 Effect 引用配对、顺序、Cancel\* 无回执原则必须保留。

## 4.9 DTO Mapping

集中在：

```
composer-bridge-protocol/src/v2/mapper.rs
```

提供：

```
domain_to_snapshot_dto()

domain_to_content_dto()

dto_to_domain_content()

domain_effect_to_dto()

dto_outcome_to_domain_event()

domain_to_persisted_payload()
```

所有有校验可能失败的转换使用 `Result`。

不要在各个 Host 分别手写映射。

---

# 5. L4：独立 Editor Document Model

这是与现有 Lexical Island 结合的部分。

位置：

```
lexical-island/src/
  model/
    editor-document-v1.ts
    editor-event-v2.ts
    editor-command-v2.ts

  adapter/
    lexical-to-composer.ts
    composer-to-lexical.ts
    validate-editor-document.ts

  nodes/
    composer-attachment-node.ts
    composer-mention-node.ts

  create-controller.ts
```

## 5.1 EditorDocumentV1

```
export interface EditorDocumentV1 {
  editor: "lexical";

  version: 1;

  state: SerializedEditorState;
}
```

它是 Lexical 的序列化模型。

不是 Rust Domain Model，也不是 Persistence DTO。

它可以使用 Lexical 原生 JSON 结构。

但必须遵循：

- 仅在 Lexical Island 内部理解。
- 外部不能依赖 Lexical 的内部节点实现。
- 不允许通过直接 cast 转换成 ComposerContent。
- 必须经过明确 Adapter。
- 每种自定义 Node 都必须有稳定序列化形式和迁移策略。

如果未来换成 ProseMirror、Tiptap 或纯 Rust 编辑器，不应要求修改 Composer Core。

## 5.2 自定义附件节点

文档中的附件只保留逻辑身份：

```
export interface AttachmentNodeJson {
  type: "composer-attachment";

  version: 1;

  nodeId: DocumentNodeId;

  assetId: AssetId;
}
```

节点不保存：

- Blob 字节。
- object URL。
- 数据库路径。
- 上传后的临时下载 URL。

附件本身的信息由 AssetStore 保存。

## 5.3 Mention 节点

```
export interface MentionNodeJson {
  type: "composer-mention";

  version: 1;

  nodeId: DocumentNodeId;

  referenceId: ReferenceId;

  label: string;
}
```

引用身份不能只保存在可见文本中。

编辑器删除显示文本、移动节点、Undo/Redo 后仍然必须保留正确的 ReferenceId。

## 5.4 选区与撤销历史

明确区分：

**文档内容：**

- 正文。
- 节点。
- 格式。
- 附件节点。
- Mention 节点。

**编辑器临时状态：**

- DOM Selection。
- 当前光标。
- IME composition。
- History Stack。
- 弹出菜单。
- Hover。
- 临时输入组合状态。

临时状态不应进入 ComposerContent。

一般不将浏览器 Selection 和 IME 状态作为长期持久化数据。

## 5.5 Editor Controller

现有 Controller 可以保留生命周期和原生 Lexical API，但应调整事件携带的信息。

建议：

```
interface EditorController {
  getDocument(): EditorDocumentV1;

  getPlainText(): string;

  setDocument(
    document: EditorDocumentV1,
  ): void;

  clear(): void;

  focus(): void;

  setDisabled(value: boolean): void;

  destroy(): void;
}
```

增加 `instanceId` 和 `editorRevision`。

EditorEvent V2：

```
type EditorEventV2 =
  | {
      type: "ready";
      instanceId: string;
    }
  | {
      type: "document-changed";
      instanceId: string;
      editorRevision: number;
    }
  | {
      type: "submit-request";
      instanceId: string;
      editorRevision: number;
    }
  | {
      type: "error";
      instanceId: string;
      message: string;
    };
```

避免每次编辑都把大块 Lexical JSON 直接放进 CustomEvent。

`document-changed` 是变更通知。Dioxus 适配层收到通知后，从当前 Editor Controller 获取完整文档并映射。

提交事件必须同步捕获当时的完整文档，不能依赖一个可能尚未处理的旧变更通知。

## 5.6 Editor ↔ Domain Adapter

建立：

```
lexicalToComposer(
  document: EditorDocumentV1,
  assetMetadata: AssetMetadata[],
  references: ReferenceDto[],
): ComposerContentDto;
```

以及反向恢复：

```
composerToLexical(
  content: ComposerContentDto,
): EditorDocumentV1;
```

实际生产代码根据 Rust Mapper 和 TS DTO 的分工调整语言调用方式，但转换规则必须一致。

映射规则：

| Lexical Node        | Composer Domain  |
| ------------------- | ---------------- |
| root                | ComposerDocument |
| paragraph           | Paragraph        |
| heading             | Heading          |
| text                | Text             |
| linebreak           | HardBreak        |
| composer-attachment | Attachment       |
| composer-mention    | Mention          |
| 支持的自定义节点            | Custom           |

未知节点：

- 能够无损保留时，以 namespaced Custom Node 形式保留。
- 无法安全映射时，明确返回 UnsupportedNode。
- 不得静默删除正文或附件节点。

必须验证转换的往返性质：

```
Lexical
  → ComposerContent
  → Lexical
```

在规范化后，用户可见文档内容、节点顺序、附件身份和引用身份必须一致。

不要求 Lexical 内部随机 key 或 UI 临时状态完全相等。

---

# 6. Attachment Store：把附件字节与四层数据分开

另外增加独立的资产存储接口。

TypeScript Host：

```
interface AssetStore {
  put(
    id: AssetId,
    metadata: AssetMetadata,
    bytes: Blob,
  ): Promise<void>;

  get(
    id: AssetId,
  ): Promise<Blob | null>;

  getMetadata(
    id: AssetId,
  ): Promise<AssetMetadata | null>;

  remove(
    id: AssetId,
  ): Promise<void>;
}
```

原来：

```
interface Attachment {
  id: AssetId;
  name: string;
  mediaType: string;
  data: Blob;
}
```

不再直接作为 Domain Content 或 Bridge DTO 使用。

拆成：

```
interface AssetMetadata {
  id: AssetId;
  name: string;
  mediaType: string;
  sizeBytes: number;
  alt?: string;
  sendAsFile?: boolean;
}
```

Blob 字节只由 AssetStore 保存。

## 6.1 添加附件

完整流程：

```
File Intake
    |
    v
Generate AssetId
    |
    v
Persist Blob in AssetStore
    |
    v
Create AssetMetadata
    |
    v
Insert Lexical Attachment Node
    |
    v
Editor Adapter
    |
    v
ComposerContent
    |
    v
Rust Core
```

AssetStore 写入失败时，不能把一个不可恢复的附件当作成功加入。

## 6.2 删除附件

一次内容更新必须同时：

1. 从编辑器文档中删除对应附件节点。
2. 从资产列表中删除对应 AssetId。
3. 保留其他附件节点和顺序。
4. 不得删除其他 Submission 仍引用的同一资产。

底层 Blob 采用引用计数或引用扫描 GC。

## 6.3 移动附件

移动时：

- NodeId 不变。
- AssetId 不变。
- 只改变文档结构顺序。
- 不重写 Blob。
- 不改变 AssetStore 映射。

保持现有已验证的 Lexical 移动语义。

## 6.4 提交附件

Submission 捕获完整 ComposerContent。

之后编辑器继续编辑新草稿，不得修改旧 Submission。

Deliver Effect 只传递 AssetId 和元数据。

实际 Host 根据 AssetId 找到 Blob 字节，完成上传或组装消息。

如果 Blob 丢失，必须明确失败或标记 Missing，不能伪造成功投递。

---

# 7. Bridge 与 Editor 的接线修改

## 7.1 编辑流程

当前：

```
Lexical document-changed
    |
    v
getPlainText()
    |
    v
Snapshot::text()
    |
    v
Event::Edit
```

新流程：

```
Lexical document-changed
    |
    v
EditorRevision
    |
    v
Dioxus ComposerEditorAdapter
    |
    v
getDocument()
    |
    v
LexicalDocumentV1
    |
    v
lexicalToComposer()
    |
    v
ComposerContent
    |
    v
ComposerContentDtoV2
    |
    v
Bridge dispatch
    |
    v
Event::Edit { content }
    |
    v
Rust Core
```

编辑事件可以根据性能数据进行合并，但不能把提交时的最新内容留在编辑器内部而不传到 Core。

## 7.2 提交流程

```
Submit Request
    |
    v
Capture current EditorDocument
    |
    v
Convert to ComposerContent
    |
    v
Validate Content + DraftVersion
    |
    v
ComposerEventDtoV2::Submit
    |
    v
Event::SubmitWithContent
    |
    v
ScopeState atomic handoff
    |
    v
Persist Effect
    |
    v
Persist ACK
    |
    v
Deliver Effect
```

保证用户点击提交时，交接的是当前可见的完整文档，而不是最后一次已同步的纯文本。

提交成功之后才清空编辑器。

如果交接失败，不得提前清空。

## 7.3 编辑器恢复

```
PersistedScope
    |
    v
Persistence Codec
    |
    v
ScopeState
    |
    v
SnapshotDtoV2
    |
    v
ComposerContentDto
    |
    v
composerToLexical
    |
    v
Editor Controller.setDocument
```

恢复必须使用已有的 generation/revision 握手。

异步恢复不能覆盖已经产生的新输入。

---

# 8. Bridge ACK 顺手修复

修改当前：

```
let effect = self.pending.remove(&ref_id)
    .ok_or(AckError::UnknownRef)?;

let internal = map_outcome(&effect, outcome)
    .ok_or(AckError::OutcomeMismatch)?;
```

这里存在问题：OutcomeMismatch 发生时，pending 已经被删除。

应调整为：

```
let effect = self.pending
    .get(&ref_id)
    .ok_or(AckError::UnknownRef)?;

let internal = map_outcome(effect, outcome)
    .ok_or(AckError::OutcomeMismatch)?;

// 只有验证和映射成功后才消费引用。
self.pending.remove(&ref_id);

Ok(self.run(internal))
```

实际实现注意 Rust 借用和所有权，必要时先复制内部 Effect。

同时检查 `persist_revision` 的递增时机，必须只在成功验证并接受 Persist ACK 后递增。

新增测试：

1. 错误类型的 ACK 返回 OutcomeMismatch。
2. 错误 ACK 不消费 ref_id。
3. 随后提交正确 ACK 可以成功。
4. 正确 ACK 不能重复执行状态转换。
5. 未知 ref_id 不改变状态。
6. 不匹配的 Receipt 不能导致 Accepted。
7. 回执必须匹配原始 Effect 身份。

另外，要明确当前重复 ACK 返回 UnknownRef 是 consume-once 防重，不等于完整的请求重放幂等。

如果要实现真正的 ACK 重放安全，应增加请求 ID 和已处理 ACK 的结果缓存；不应仅在文档中宣称幂等。

不要求本次顺便大改整个 Effect 系统，但必须把语义写清楚。

---

# 9. Bridge V1 → V2 迁移

因为 D002 明确冻结 V1 JSON 形状，所以：

1. 不在 V1 下偷偷更换字段。
2. 建立 V2 独立 DTO。
3. 建立 V2 Golden JSON Fixtures。
4. 更新 app-react generated/protocol.ts 参考类型。
5. 更新 WASM JSON 入口。
6. 更新 Dioxus Adapter。
7. 更新 BM05 的协议测试。
8. 检查所有旧消费者。
9. 所有内部 Host 完成迁移后再删除废弃 V1 路径。

如果 V1 没有外部兼容承诺，可以采用一次原子迁移，不必长期维护双协议。

不要让某个 Host 使用 V1，另一个 Host 使用 V2，却声称它们实现同一份协议。

协议版本、Persistence Version、Editor Document Version、Domain Document Version 分别管理。

---

# 10. 必须新增的测试

## 10.1 Domain 测试

- 纯文本转换为文档。
- 文档转纯文本。
- 多段落顺序。
- 文档附件节点身份。
- rail-only 附件。
- Mention 引用。
- 重复 NodeId 拒绝。
- 悬空 AssetId 拒绝。
- 悬空 ReferenceId 拒绝。
- 未知 Custom 节点保留。
- Submission 内容不可受后续 Draft 修改影响。

## 10.2 Persistence 测试

- Domain → Persisted → Domain Roundtrip。
- Persisted V1 → 新 Domain Migration。
- 旧附件引用保留。
- 旧引用身份保留。
- Missing Blob 恢复。
- 不支持的 Schema 明确拒绝。
- 损坏 JSON 不覆盖原文件。
- PendingReceipt 完整恢复。
- 所有历史 Submission 完整恢复。

## 10.3 Bridge 测试

- V2 JSON Golden Fixture。
- Domain → Snapshot DTO。
- DTO → Domain Event。
- DeliveryDto 映射。
- ReceiptDto 验证。
- DeliveryResultDto 验证。
- Persist Payload 不暴露 ScopeState。
- 错误 ACK 不消费引用。
- UI 无法伪造 DeliverResult。
- 版本不匹配明确失败。

增加架构约束测试：

Bridge 公共 DTO 中不得出现：

```
ScopeState
Draft
Submission
Delivery
DeliveryResult
Receipt
```

这些类型可以出现在 private Mapper 实现内部，但不得成为公开 JSON 契约的字段类型。

## 10.4 Editor 测试

- Lexical → Domain → Lexical。
- 文本与换行顺序。
- 粗体、斜体等已有格式。
- Mention 身份保持。
- Attachment NodeId 保持。
- AssetId 保持。
- 插入附件。
- 删除附件。
- 移动附件。
- 复制粘贴附件。
- Undo/Redo。
- 草稿恢复。
- 过期 editorRevision 拒绝。
- editorInstanceId 不匹配拒绝。
- Submit 时获取最新文档。
- 旧异步恢复不能覆盖新输入。

继续复用已有 B01–B32 浏览器测试资产，不要因新增模型把已有行为测试删掉。

## 10.5 跨层集成测试

必须有以下完整路径：

```
Lexical
 → ComposerContent
 → Bridge V2
 → Rust Core
 → Persistence Model
 → 保存
 → 加载
 → Domain
 → Snapshot DTO
 → Lexical
```

恢复后检查：

- 文本一致。
- 顺序一致。
- 附件身份一致。
- Reference 身份一致。
- Submission 状态一致。
- Blob 可读取。
- Draft Generation 正确。

再执行：

```
编辑
 → 提交
 → 编辑新草稿
 → 恢复旧 Submission
```

确认旧 Submission 内容没有变化。

---

# 11. 实施顺序

不要一次批量修改所有文件。

## Phase A：四层模型定义

先完成：

- Domain 类型。
- Editor 类型。
- Persistence 类型。
- Bridge V2 DTO。
- ER / 类型关系图。
- Mapper 设计。

此阶段先不删除 V1，不改现有运行链。

交付：模型定义文档 + 独立单元测试。

## Phase B：映射与迁移

实现：

- Domain Mapper。
- Persistence Codec。
- Legacy V1 Migration。
- Bridge V2 Mapper。
- Golden JSON。
- ACK 修复。

必须证明原数据可恢复。

## Phase C：接入编辑器

实现：

- LexicalDocumentV1。
- Lexical ↔ Composer Adapter。
- AssetStore。
- 完整 Edit 内容。
- 完整 Submit 内容。
- 恢复同步。

从纯文本路径渐进切换到完整文档路径。

## Phase D：清理旧耦合

全仓搜索：

```
ScopeState
DeliveryResult
Snapshot
attachment_ids
reference_ids
```

检查是否仍有跨界直接引用。

逐个处理旧调用者，不允许通过把 DTO 直接改名来掩盖耦合。

完成后删除确实不再需要的 V1 接线。

## Phase E：回归验收

运行：

- Rust Workspace 测试。
- BM05 及 V2 Golden Tests。
- WASM 编译。
- Dioxus Web 测试。
- Lexical Browser E2E。
- 历史迁移测试。
- 附件恢复测试。
- 提交和 ACK 故障测试。

最后生成完整差异报告。

---

# 12. 明确禁止

1. 禁止 `SnapshotDto { state: ScopeState }`。
2. 禁止 `EffectDto::Persist { state: ScopeState }`。
3. 禁止 DTO 直接使用内部 Delivery 和 DeliveryResult。
4. 禁止 `type XDto = InternalType`。
5. 禁止 Lexical JSON 直接成为 Domain Document。
6. 禁止把 Blob 字节塞入 Bridge 消息或文档 JSON。
7. 禁止把 object URL 当持久化引用。
8. 禁止仅传 text 就声称富文本已完成同步。
9. 禁止用字符串数组代替有身份的结构化文档。
10. 禁止删除未知节点、引用或附件数据。
11. 禁止改变 V1 JSON 却不升级协议版本。
12. 禁止持久化格式直接依赖 Core Serialize 布局。
13. 禁止编辑器异步恢复覆盖后续输入。
14. 禁止提交时使用过期编辑器快照。
15. 禁止因迁移失败清空原有持久化数据。

---

# 13. 交付报告

完成后提供：

1. 四层模型定义及职责。
2. 新增和修改的 Rust 类型。
3. 新增和修改的 TypeScript 类型。
4. 全部 Mapper 清单。
5. 旧持久化数据迁移方案及结果。
6. Bridge V1/V2 兼容情况。
7. AttachmentStore 接线情况。
8. Lexical 富文本同步情况。
9. ACK 修复情况。
10. 新增测试和通过数量。
11. 未完成项及其真实阻塞原因。

必须明确区分：

- 已定义。
- 已实现。
- 已接入运行链。
- 已通过单元测试。
- 已通过真实 Browser E2E。

仅新增类型定义，不代表完成了四层模型隔离。

**最终验收标准：四层分别演进时，不需要修改另外三层的内部类型；只需要更新对应的显式 Mapper、版本化契约或 Migration。**

## **三、我特别建议你不要忽略的三个设计细节**

上面是完整任务，以下三个问题尤其重要。

### **1. 不要让 Lexical 成为整个 Composer 的数据标准**

你现在的实际情况是：

- Dioxus 主产品用 Rust。
- Lexical 负责浏览器富文本编辑。
- 以后可能替换编辑器。
- 编辑器不应该决定消息在数据库里怎么保存。

所以必须有自己的 `ComposerDocument`。

例如：

**同一段用户内容**

Lexical Editor Document

`root → paragraph → text + composer-attachment`

Composer Domain Document

`Paragraph → Text + Attachment(asset_id)`

Persistence Document

`block_type + node_id + asset_id + version`

以后从 Lexical 换到 ProseMirror，甚至换成完全 Rust 的编辑器，只需重写 Editor Adapter。

核心消息历史、附件引用、归档格式不受影响。

### **2. AssetId 和 BlobHash 必须是两种身份**

这一点非常容易被忽略。

例如：

同一张 `diagram.png` 被拖进两个不同的会话。

**会话 A**

`AssetId: A001`

**会话 B**

`AssetId: B001`

相同的内容哈希

**BlobHash**

`SHA256: abcdef...`

只保存一份原始字节

如果直接把 AssetId 当文件哈希，就会把逻辑身份和物理存储绑死，后面做去重、复制、移动和版本管理都会麻烦。

### **3. Bridge DTO 不能只是 ScopeState 的瘦身版**

它还应该体现**权限边界**。

例如 UI 能发送：

`Edit / Submit / Retry / Remove`

但不能发送：

`DeliverResult / Persisted / LookupResult`

这些必须由 Host 实际执行 Effect 后，通过 ack 进入 Core。

因此，DTO 不只是序列化结构，它同时是跨边界的能力限制。

另外，`SnapshotDto` 应该是只读的 UI 视图，而不是 Core 内存的完整镜像。内部增加一个 pending receipt 字段，不应该自动要求 UI 升级协议。

## **四、最终架构裁定**

我建议最终形成这样的职责：

| 模块                         | 权威对象              | 允许依赖                      |
| -------------------------- | ----------------- | ------------------------- |
| `composer-core`            | Domain Model      | 业务类型                      |
| `composer-session-format`  | Persistence Model | 独立 Schema                 |
| `composer-bridge-protocol` | Bridge DTO        | Domain Mapper、协议 Schema   |
| `lexical-island`           | Editor Document   | Lexical                   |
| `composer-editor-adapter`  | 编辑器与业务映射          | Editor Model + Bridge DTO |
| AssetStore                 | Blob 字节           | 平台存储 API                  |

这里我会坚持一个原则：

**不要让任何一层为了方便，直接借用另一层的完整结构体。**

可以复用语义，但必须显式映射。

最后，实施顺序上不要先把附件、Lexical、持久化和 Bridge 全部同时改掉。先把四层类型定义、转换规则和 V2 Golden Tests 建好，再接入实际链路。这样出了问题，可以准确定位是编辑器映射、领域状态、持久化迁移还是协议转换的问题。

我认为这项工作比单纯补全 `Edit` 附件字段更重要，因为它决定了 Composer 将来能否更换编辑器、持久化后端和 Host，而不需要重写 Core。