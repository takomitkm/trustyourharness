直接在项目仓库里增加一个目录：每一次重大 Agent 操作对应一个 Run。

文件职责：

文件

内容

prompt.md

完整原始指令，不做删减

result.md

Agent 最终报告、测试结果、已知问题

metadata.json

时间、模型、工具、关联 Commit、任务状态。给每次 Agent 工作分配唯一 ID，用utc时间戳，要记录在commit中
