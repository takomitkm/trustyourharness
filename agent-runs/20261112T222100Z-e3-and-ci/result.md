# Run 结果报告：main 合并 + E3a–E3d（菜单/附件/IDB/网络 E2E）+ 双 CI 建立与按额度关停

## 做了什么

feat 52 提交合入 main（API 逐字节复刻 13 提交 + 校机 git 直推）+ 计划状态同步（7564327）；E3a 候选菜单（14/14）、E3b 附件/图文调位（12/12）、E3c Rust 主线 IDB 持久化+菜单悬停（6/6）、E3d 不可靠网络 E2E（12/12×3 稳定）；composer 工作流建立并首轮 13 步全绿（run 43322487339）后按用户指示 API 禁用；composer-react 工作流建成、从未 dispatch。

## 测试结果

见上（各验证数字为当次实录，工具链与判据细节在对应 goal-log/commit）；本条为回填复原，非实时报告。

## 已知问题

回填条目：时间与提示词按 metadata 的 time_basis / prompt_note 口径复原。
