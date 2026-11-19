# agent-runs — Agent 操作运行台账

每一次重大 Agent 操作对应一个 Run（目录）。约定（2026-11-20 用户定）：

```
agent-runs/<UTC时间戳>-<短slug>/
  prompt.md      完整原始指令，不做删减
  result.md      Agent 最终报告、测试结果、已知问题
  metadata.json  时间、模型、工具、关联 Commit、任务状态
```

- **唯一 ID** = UTC 时间戳（格式 `YYYYMMDDTHHMMSSZ`），同时是目录名前缀与 `metadata.json` 的 `id` 字段
- **Run ID 必须记录在对应 commit 的提交信息中**
- 目录名 = `<ID>-<短slug>`（slug 为可读性补充，ID 以 metadata.json 为准）
- `task_status` 取值：`completed` / `partial` / `failed`
- **与 Commit 的关联（自指规避）**：
  - Run 期间产出的**工作提交**：提交信息带 `(Run <ID>)`，sha 记入 `metadata.related_commits`
  - **台账提交**（把 Run 目录本身收进仓库的那笔提交）：提交信息含 Run ID 即可，
    **不**写入 `related_commits`——提交无法包含自己的 sha，关联靠提交信息回查
    （`git log --grep=<ID>`）

