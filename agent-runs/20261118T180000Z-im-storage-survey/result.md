# Run 结果报告：调研扩展：常见 IM（Matrix/Telegram 等 12 家）存储与防膨胀机制

## 做了什么

12 家 IM 机制对比；Matrix state-group delta（每约 100 次状态变化落全量）当日经官方文档核实原话；Telegram 内嵌迷你预览+可逐出缓存；"过长卡顿"拆为磁盘/加载/内存三问题各配解法；组合建议（opencode 类型化序列+codex/DSH 追加归档+Signal 载荷出列+Telegram 轻量预览+Matrix 快照思想+XMPP 分页终点）。后落入 im-storage-patterns.md 与六层蓝图。

## 测试结果

见上（各验证数字为当次实录，工具链与判据细节在对应 goal-log/commit）；本条为回填复原，非实时报告。

## 已知问题

回填条目：时间与提示词按 metadata 的 time_basis / prompt_note 口径复原。
