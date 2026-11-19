# Run 结果报告：调研：dsh-chat-import 22 家会话数据结构，判定哪家最好

## 做了什么

通读 lib/sources/ 全部 22 个解析器（4858 行）：判定 opencode V2 最优（类型化 seq 记录+compaction 边界语义+四家 fork 复用实证）、codex rollout JSONL 次优（抗 crash 最强、读者复杂度高）、豆包 IndexedDB 对外部读取方最差（三层解码 1084 行；口径=读取方复杂度，非其运行性能）；病理合集（cline 三头分裂/crush 无 cwd/cursor-trae 借壳/goose 孤儿数据）。结论后落入 e03a903 的 docs/notes/agent-session-formats.md。

## 测试结果

见上（各验证数字为当次实录，工具链与判据细节在对应 goal-log/commit）；本条为回填复原，非实时报告。

## 已知问题

回填条目：时间与提示词按 metadata 的 time_basis / prompt_note 口径复原。
