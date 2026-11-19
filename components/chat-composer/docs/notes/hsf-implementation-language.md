# HSF 实现语言选型评估及架构决策（Draft）

- **日期**：2026-11-19 · **状态**：技术选型评估及架构决策——**不是**已通过全平台验证的最终实现方案
- **关系**：约束 [hsf-data-model.md](hsf-data-model.md) / [hsf-protocol.md](hsf-protocol.md) 的实现语言；
  原文见 [hsf-spec-1.0-draft-full.md](hsf-spec-1.0-draft-full.md)
- **先导实验（§7）未完成前，Web SQLite = 技术可行，≠ 项目已验证能力**

## 1. 最终裁定

| 角色 | 语言 | 职责 |
|---|---|---|
| **存储内核** | **Rust** | 数据模型、事务、状态机、Migration、Import Service、归档 |
| 桌面数据库 | Rust + rusqlite（native） | V1 主战场（校机可全链路测） |
| Web 数据库 | Rust + rusqlite 0.38 + sqlite-wasm-rs | **不做第二套 Repository**——同一套 SQL/事务/Migration，只换 ffi 层与 VFS |
| Web 持久化 | OPFS SAHPool（经 sqlite-wasm-vfs crate） | 专用 Worker 是唯一写协调者 |
| UI / Host 边界 | TypeScript（既有） | **不得成为第二套存储内核** |
| 导入适配器 | 现有 MJS（dsh-chat-import 22 解析器） | 只产出 HSF 归一化记录，经窄边界进 Rust Import Service，**不得直接写 SQLite** |
| Go / C++ | 不进运行时 | 无对应收益支撑第二个运行时/FFI 层 |
| Python | 不进运行时 | 只可做离线检查类 CLI（`harness db check` 等）的可选实现 |

## 2. 事实核对（2026-11-19 当日验证）

| 声明 | 核对结果 | 证据 |
|---|---|---|
| rusqlite 0.38.0 支持 wasm32-unknown-unknown（2025 底） | **属实** | GitHub Release v0.38.0（2025-12-20）：*"Add `wasm32-unknown-unknown` support #1769, #488, #827"* |
| "rusqlite 默认特性已包括 ffi-sqlite-wasm-rs" | **名称不准，实质成立** | docs.rs 0.38.0 的 45 个 feature 里**没有** `ffi-sqlite-wasm-rs` 这个名字；真实机制=按 target 选 ffi 后端（`buildtime_bindgen` 会开 sqlite-wasm-rs 的 bindgen 特性，即 sqlite-wasm-rs 是其 wasm 依赖）。正确表述：**编到 wasm32 时依赖自动落到 sqlite-wasm-rs** |
| ruzstd 有纯 Rust 编码器，不及原版 | **属实** | docs.rs ruzstd 0.9.1：*"does not yet reach the speed, ratio or configurability of the original zstd library"*；解码比 C 慢约 1.4–3.5×；编码支持 level 1–11+字典 |

由此修正原评估：**Web SQLite 从"硬缺口"降级为"平台适配问题"**——Rust 不再需要把存储查询交给 TypeScript。

## 3. Web SQLite 平台限制（先导实验要逐条验证）

| 项 | 限制 | 来源 |
|---|---|---|
| 多线程 | SQLite 编译为 `-DSQLITE_THREADSAFE=0`，**不支持多线程** | sqlite-wasm-rs 文档（已核实） |
| 执行环境 | OPFS/SAHPool 需 Dedicated Worker + secure context | 浏览器平台约束（该 crate 文档页未写明，先导实验验证） |
| 多标签页 | SAHPool 默认**不能**多 Tab 同时打开同一 DB——不是"免 COOP/COEP 就完美支持并发"，是用并发能力换性能 | 同上 |
| COOP/COEP | SAHPool 不需要；其他 OPFS VFS 形态可能需要 | 同上 |
| WAL | Web 环境并发语义与桌面不同，`synchronous=FULL` 语义待验证 | 协议书 §2 的桌面假设需在 Web 重新确认 |

## 4. 对原评估的三处修正（接受）

1. **"事务纪律可编译期强制"表述过强**：Rust 提供的是*能力*（RAII 事务生命周期、Newtype Seq、Typestate 状态转换、事务闭包防句柄逃逸），不是自动保证——持有 Transaction 时调网络/Shell 不会自动被阻止。正确表述：**生命周期与状态约束靠类型系统，跨系统副作用靠 API 形状（事务闭包结束才进入外部执行流程）+ 测试 + 审查**。数据库 Transaction 对象不得传给 Tool Runtime。
2. **内存可预测性有前提**：无 GC ≠ 无峰值。还要管 SQLite page cache、prepared statement cache、JSON 反序列化分配、Blob 缓冲、FTS 缓存、返回 DTO 大小、每连接缓存上限；1M 消息禁止一次性 `Vec<Message>`。Rust 是实施这些纪律的更顺手语言，**性能以实测压测为准**。
3. **zstd 不必引 JS**：Web 解码用 ruzstd；Web 编码先基准测 ruzstd 编码器（存在但未达原版），大规模归档必要时进专门 Worker；实时消息不做全会话压缩。Desktop 归档用原版 zstd binding。

## 5. 项目结构

```
crates/
  composer-store/            # 规范类型、事务接口、存储服务、Migration 编排（平台无关）
  composer-store-sqlite/     # 统一 SQLite 实现：connection/transaction/query/migration/blob/fts
  composer-store-wasm/       # 只做平台适配：browser 初始化、OPFS/SAHPool VFS、Worker 生命周期
  composer-session-format/   # JSONL + Manifest + zstd + 校验 + 归档迁移
adapters/dsh-chat-import/    # 22 个既有 MJS 解析器 → HSF 归一化输出
```

**关键决定**：`composer-store-wasm` 不实现第二套 Repository——存储查询、事务语义、Migration、
数据模型全部复用 composer-store / composer-store-sqlite，Desktop 与 Web 不允许各自维护一套 SQL 逻辑。

## 6. 架构决策记录

**ADR-001 存储后端抽象层**：业务代码不得直接依赖 `rusqlite::Connection`；定义 Store 接口
（Session/Message/Run/Tool/Blob/Archive Repository + Transaction Manager）。但**不做**万能 KV 抽象——
保留 SQLite 特有能力（FTS5、事务、Keyset 分页、批量查询）。

**ADR-002 Web 持久化策略**：一个专用 Worker 管一个数据库实例；Worker 是唯一写协调者；
禁止多 Tab 各自初始化 SAHPool 开同一 DB；明确 OPFS 不可用时的降级方案与配额/隐私模式/数据清理风险；
"SQLite WASM 编译通过"≠"生产持久化完成"，浏览器恢复测试独立做。

**ADR-003 消息编辑与重生成语义**（含用户 2026-11-19 拍板：**要支持编辑**）：

| 操作 | 处理 |
|---|---|
| 正常追加 | 新 seq |
| 模型重新生成 | 新 Message，关联原始（supersedes） |
| **用户编辑历史消息** | **新版本 Message + `supersedes_message_id` 指向旧版（或独立 revision 表），旧版本保留**——UI 呈现为"已编辑"，可回溯 |
| 删除消息 | 删除标记事件，不动底层证据 |
| 工具重试 | 新 ToolInvocation，关联旧调用 |
| Compaction | 新 Checkpoint，不改历史 Message |

**不允许**直接 UPDATE 已完成历史消息的正文：Run Context Manifest 记录了每次模型请求内容的
content_hash、checkpoint 记录了 source_digest——原地改写会让"当时模型看到的是什么"无法回答。
**支持编辑与可追溯同时成立的唯一实现 = 修订链（追加），不是原地改。**

## 7. 先导实验（Web SQLite 能力的验收门槛）

1. **Native**：Rust + rusqlite 建库、`BEGIN IMMEDIATE` 写入事务、进程重启后读回一致。
2. **WASM**：Rust + rusqlite + sqlite-wasm-rs 对 wasm32-unknown-unknown **编译成功**。
3. **Browser**：Dedicated Worker 内经 SAHPool 持久化，**刷新页面后数据仍能恢复**。
4. **行为验证**：重复初始化、跨 Tab 打开同一 DB、浏览器异常终止时的实际行为记录。

四步全过，Web SQLite 才从"技术可行"升级为"项目已验证能力"。

## 参考资料（2026-11-19 已核实）

- rusqlite v0.38.0 Release（api.github.com，2025-12-20）：wasm32-unknown-unknown support
- docs.rs/crate/rusqlite/0.38.0/features：无 `ffi-sqlite-wasm-rs` feature（机制=target 门控）
- docs.rs/sqlite-wasm-rs：wasm32-unknown-unknown 绑定、SAHPool 经 sqlite-wasm-vfs、THREADSAFE=0
- docs.rs/ruzstd（0.9.1）：纯 Rust 编解码，编码未达原版速度/压缩率/可配置性
