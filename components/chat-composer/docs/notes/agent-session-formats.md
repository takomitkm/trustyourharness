# 调研：各 Agent 会话数据结构对比（dsh-chat-import 22 个解析器）

- **调研日期**：2026-11-19
- **被检对象**：`dsh-chat-import` 0.26.6（DSH 桌面版 node_modules 安装，非 git 检出，故记版本号不记 commit）
- **方法**：通读 `lib/sources/` 全部 22 个解析器源码
- **行数口径**：`wc -l lib/sources/*.mjs`，22 文件合计 **4858 行**；只含 sources/（存储读取层），
  不含 lib/convert/（纯函数转换器）与宿主编排层
- **证据级别**：本文件所有机制描述均为**源码观察**（直接读解析器代码）；"哪家最好"的判定是
  基于这些观察的**分析结论**，不是实测性能数据

## 全景表

| 家 | 存储 | 解析器行数 | 特点（源码观察） |
|---|---|---|---|
| opencode | SQLite 单库；V1 `session/message/part`，V2 `session_v2/session_message`（同一 opencode.db 双世代） | 390 | 类型化消息记录；`(session_id, seq)` 唯一排序；显式 compaction 边界语义 |
| kilocode / mimocode / teleagent / zcode | opencode schema 同构/超集 | 72–91 | 纯适配层，复用 opencode 通用读法（四家 fork 实证该 schema 可直接抄） |
| codex (ChatGPT) | JSONL 追加：`rollout-<ts>-<threadId>[_<pageId>].jsonl`，归档转 `.zst` | 521 | 首行 `session_meta` 自描述（`history_mode: "paginated"`）；`history_base` 链接上页；threadId 全链一致；解析端需预算限制（maxEntries=200000/maxPages=2048）+ 链缺口检测 |
| copilot-cli | 每会话一个 `events.jsonl`，cwd 在转录内 | ~34 | 极简 |
| vibe | 每会话目录 `meta.json` + `messages.jsonl` | 211 | 双文件联合指纹 |
| codearts | `meta.json` + `history.jsonl` + `messages.index.jsonl`（指向 history.jsonl 的**字节区间索引**） | 242 | 22 家中唯一带 byte-offset 索引 |
| DSH 自家 | `session.vN.jsonl` 代次命名 + zstd | 50 | 实测 52 会话中 48 个是 v3——没有代次命名就读不到当前代次 |
| goose | `sessions.db`（WAL）；旧 `sessions/*.jsonl` 建库时迁移一次后不再读 | 219 | 旧数据孤儿化 |
| hermes | `state.db`（SQLite）权威 + `sessions/*.jsonl|.json` 回退 | 189 | 双轨 |
| zed | `threads.db` | 172 | — |
| cline | SQLite `sessions.db` **只存索引**；权威元数据在 `<id>.json` manifest；消息在 `<id>.messages.json` | 204 | 三个真相来源，导入端要自己对账 |
| crush | 项目内 `.crush/crush.db`；**DB 里没有 cwd** | 204 | 项目身份靠 `projects.json` 反查 |
| cursor / trae | VSCode `state.vscdb`（globalStorage 共享库）+ workspaceStorage sidecar | 143 / 265 | 借壳 VSCode，与所有扩展共表；schema 漂移靠"无登记"兜底 |
| opencowork | 双 schema（legacy/next），按表签名区分 | 181 | 需签名检测 |
| 豆包 | Chromium IndexedDB（LevelDB 目录） | 832 + 252 = **1084** | 读数据要剥三层：LevelDB SSTable 内部键尾标+snappy → Chromium IDB value 包装（`FF 11 02`，Blob 外置 `FF 11 01` 读不到）→ V8 SerializedScriptValue（自写按 tag 读取器） |

## 判定（分析结论）

**最好：opencode V2**。四条依据（全部源码观察）：

1. 消息是类型化记录（user/assistant/compaction/synthetic/shell/... 十种显式 type），
   assistant content 是 `text | reasoning | tool{status,input,error} | file` 结构化部件——导入端不用猜。
2. 排序靠 `(session_id, seq)` 唯一键，不靠文件名时间戳、不靠 mtime、不靠可能秒/毫秒混用的
   JSON 数字（trae 就得专门处理这两种坑）。
3. compaction 边界有语义：显式记录"模型实际看到了哪条之后的历史"
   （`seq >= 最近 completed compaction`），正文取 summary+recent 两段；running/failed 的
   compaction 不当边界、不静默丢。全场唯一把上下文窗口语义存进数据结构的设计。
4. kilocode、mimocode、teleagent、zcode 四家 fork 原样沿用该 schema——适者生存的实证。

**次优：codex rollout JSONL**。抗 crash 最强（追加流、坏行跳过、converter 有守卫）、自描述
（首行 meta + 分页链 + zstd 过渡态有身份键归一）；代价是把复杂度推给读取方（分页链解析
521 行）。适合"永不丢数据"，不适合"随手查"。

**最差（对外部读取方而言）：豆包 IndexedDB**——见下方口径警告。

## 口径警告（不要过度解读）

- 解析器行数是**读取方实现复杂度**的代理指标，**不等于**豆包存储格式本身的运行时性能。
  1084 行首先说明"外部程序想读它要付多大代价"，以及 schema 跟着 Chromium 内部实现走
  （上游一改即失效）、Blob 外置值不可读这些结构性问题。
- "最好/最差"的判定口径 = 对外部读取方的可读性、可维护性、语义完整性；不是存储引擎
  本身的写性能或压缩率。

## 病理合集（反面教材）

- **cline 三头分裂**：SQLite 索引 / manifest 权威元数据 / 消息文件三方对账。
- **crush 库里没有 cwd**：库文件搬走身份就断。
- **cursor/trae 借壳 VSCode**：与所有扩展挤一张 `state.vscdb`。
- **goose 旧 JSONL 迁移后不读**：数据在盘上但工具链弃了。
- **豆包**：活体二进制（schema 不可见、随 Chromium 内部版本漂移）。

## 对 composer 的启示

见 [session-storage-design.md](session-storage-design.md)（设计结论单独成文）。
