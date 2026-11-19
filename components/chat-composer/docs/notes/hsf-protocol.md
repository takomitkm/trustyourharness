# 会话存储协议书（HSF 1.0 Draft · 一页）

- **日期**：2026-11-19 · **状态**：契约草案，未排期；实现方必须遵守本文全部 MUST
- **配套**：表结构见 [hsf-data-model.md](hsf-data-model.md)；背景见 [session-storage-design.md](session-storage-design.md)
- **原文存档**：规格全文逐字节存档于 [hsf-spec-1.0-draft-full.md](hsf-spec-1.0-draft-full.md)——本页是 MUST 契约的精选，**若有偏差以原文为准**，修改本页前先对拍原文

## 0. 权威与派生（数据来源的唯一性）

- **V1 权威**：SQLite 规范表（sessions/messages/parts/runs/tool_invocations/events）+ 已哈希验证的 Blob 原始字节
- **派生（可重建、不可作权威）**：FTS、会话卡片摘要、Checkpoint、Runtime Snapshot、Archive
- 不同表/文件之间**不允许出现无法定优先级的双权威**；Archive 是验证过的不可变副本，第一阶段不承担唯一来源

## 1. 排序与读取

- 排序分页**一律 seq**（keyset pagination）：`WHERE seq < ? ORDER BY seq DESC LIMIT 51`，多取一条判断 `has_more`
- **禁** `OFFSET`、禁滚动时 `SELECT COUNT(*)`、禁打开会话 `SELECT *` 全史
- 分页终点必须显式（`next_cursor: null` + `has_more: false`，XMPP MAM complete 同款）
- UI：初始窗口 ~50 条、虚拟化列表、附件进可视区才加载、卡片只读 sessions 投影

## 2. 写入与事务

- 消息写入原子化：`BEGIN IMMEDIATE` → 分配 seq → 插 Message/Parts → 插 event → 更新计数/预览/活跃时间 → COMMIT
- seq 分配在短写事务内原子完成；**禁** 两进程 `SELECT MAX(seq)+1`
- 外部调用（模型请求/工具执行）期间**不得持有写事务**；采用"开始提交 / 执行（无锁）/ 完成短事务提交"两阶段
- 运行配置：`WAL` + `synchronous=FULL` + `foreign_keys=ON`（每个连接）+ `busy_timeout`；备份走 SQLite Backup API，禁止只拷 `.db` 不带 WAL；WAL 库不上网络文件系统

## 3. 压缩（Checkpoint）协议

1. 只有 **completed** 的 checkpoint 生效；running/failed/stale 不得改变模型上下文
2. **绝不删除**被覆盖的原始消息；摘要+raw_keep_from 组合成模型实际所见
3. 保存被总结范围、生成时的模型与模板版本、`source_digest`——历史被改必须按 digest 失效相关 checkpoint
4. checkpoint 是不可变产物：新压缩=新记录；pinned 消息与最近消息有独立上下文入口
5. 每次 ModelCall 记 **Run Context Manifest**（看到了什么、顺序、content_hash），能回答"为什么这次上下文不一样"
6. 不照搬 Matrix"每 100 次变化落全量"——那是对房间状态，不是线性对话

## 4. Blob 协议

- 写临时文件 → 完整写入 → SHA-256 校验 → **原子重命名**至哈希路径 → 事务内建引用
- 同内容只存一份；**GC 只回收无引用对象**（共享引用不因单条消息删除而释放）
- 数据库不存大型 Base64；可配置内联阈值（建议 64KiB），小文本留在 SQLite 保可查性
- 读取先验证引用；禁止用户文件名直达存储路径（防目录穿越/覆盖）

## 5. 归档协议

- 8 步：临时文件 → 导出结构化记录（含工具结果与附件关系，不遗漏）→ zstd → 逐行验证 → 记录数与 seq 范围核验 → SHA-256 → **独立 Manifest**（校验和不放被哈希文件自身）→ 原子重命名 + 库内登记
- 未通过验证的归档**不算有效归档**；chunk ≈8MiB 未压缩；普通 zstd 流不假定随机访问，随机读另评估 seekable frames
- **三分离**：Archive（归档副本）/ Evict（冷载荷迁出热库）/ Delete（真删）——默认只做 Archive；V1 归档后**不删**库内原始数据，冷热迁移另案且必须先证"归档完整+附件齐全+可恢复"

## 6. 恢复协议

- `status=streaming` 且进程已死：保留已有 Part、合并已持久化 chunk、标 **interrupted**——**不伪造 completed**
- `status=running` 的工具按类型恢复：纯查询可安全重放 / 文件写入查结果与操作 ID / 网络用幂等键 / Shell 查进程与环境 / **不可逆操作禁止自动重放**
- 启动时提供 `db check / session check / blob verify / archive verify / archive restore-test`；先报告，自动修复须有明确规则

## 7. 版本与导入

- 四种版本分开管理：SQLite schema / 归档格式 / 事件 / Part schema；升级允许过渡期，未知类型保留原样，迁移失败回滚，禁止重导旧文件覆盖新数据
- 导入统一走 Import Service + 各来源 Adapter（Adapter 不直接摸底层表）；`import_mappings` 防重复；Provider 私有字段进 `metadata_json` 来源命名空间，**不丢弃**
- 导入内容与工具输出一律视为不可信数据；敏感字段不进普通日志

## 8. 明确禁止（§22 精选）

一会话一无限 JSON 当唯一库 / 只靠时间戳排序 / 开会话全量 SELECT / 多模态 Base64 塞 JSON / 工具结果拼进 assistant 一条 text / 把一次模型调用等同一次 Agent 执行 / 用摘要覆盖原始历史 / FTS 当权威 / OFFSET 长分页 / 失败的 compaction 当上下文边界 / 工具执行期间长锁库 / 未验证 blob 或归档就建立引用或删原始数据

## 9. 验收（三条端到端）

1. 几万条消息+多工具+多模态的会话：分页与搜索快（EXPLAIN QUERY PLAN 验证索引，禁全表扫）
2. 模型生成与工具执行中**强杀进程**：重启后状态正确恢复
3. 压缩→归档→数据库重建后：原始历史与附件完整还原

配套压测集：100 条 / 100k / 1M 消息、单消息 100 parts、10MiB 工具输出、1000 附件、10k 会话、并发读写、归档多代、写/执行/归档中途崩溃注入。

## 参考资料（外部规格提供，未逐条核验）

OpenCode `session/sql.ts`+`session-message.ts`、CloudEvents 1.0、OpenTelemetry Log Model、
Matrix CS API、Jupyter nbformat、Git Objects、SQLite WAL/FTS5（中文需应用层分词或 trigram，
**FTS5 默认 tokenizer 不解决中文**，记 tokenizer_version）。
