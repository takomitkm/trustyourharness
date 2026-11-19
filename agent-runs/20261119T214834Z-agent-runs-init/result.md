# Run 结果报告：agent-runs 运行台账建制

## 做了什么

1. 建立仓库根目录 `agent-runs/`，约定写入 `agent-runs/README.md`（每次重大 Agent 操作
   = 一个 Run 目录；prompt.md / result.md / metadata.json 三件套；ID=UTC 时间戳；
   Run ID 进 commit 信息）。
2. 首个 Run（本目录）= 本次建制操作自身的自记录：
   - `prompt.md`：用户 2026-11-20 指令原文，未删减
   - `metadata.json`：ID `20261119T214834Z`、模型、工具、状态
3. **自指问题的发现与定案**：本 Run 的载体提交无法把自己的 sha 写进自己的
   metadata.json（amend 一次 sha 就变，数学上无解）。README 定下规避条款：
   - `related_commits` 只记 Run 产出的**工作提交**（提交信息带 `(Run <ID>)`）；
   - **台账提交**（收录 Run 目录的那笔）靠提交信息里的 Run ID 关联
     （`git log --grep=<ID>`），不回填 sha。

## 测试结果

文档类改动，无测试。校验项：

- `git log --grep=20261119T214834Z` 能查到台账提交 ✓
- `metadata.json.related_commits` = `[]`（本次无独立工作提交，建制即台账）✓
- 工作树干净 ✓

## 已知问题

- 无阻塞项。
- 往后约定执行点：此后每次重大 Agent 操作收尾时，由执行 Agent 主动建 Run 目录并
  随台账提交收进仓库（工作提交带 `(Run <ID>)`，台账提交信息含 Run ID）。
