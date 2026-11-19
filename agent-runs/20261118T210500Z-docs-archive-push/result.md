# Run 结果报告：会话存储调研三份存档 + main 4/4 分叉强推收敛

## 做了什么

docs/notes/ 三份落盘（agent-session-formats/im-storage-patterns/session-storage-design，含证据四级标注与行数口径警告）；推送时发现本地与 origin/main 4/4 分叉=同工作双链（校机推的是旧草稿，本地终稿严格超集、多 15 行精修），经用户拍板 force-with-lease 强推收敛（c3d4eea→e03a903，零内容损失）。推送坑三连记录：GCM 在 helper 链抢在自注入 helper 之前（须先 -c credential.helper= 清空）、GT=$(…) && git 的赋值不 export（要前缀赋值）、直连间歇 reset 重试即过。

## 测试结果

见上（各验证数字为当次实录，工具链与判据细节在对应 goal-log/commit）；本条为回填复原，非实时报告。

## 已知问题

回填条目：时间与提示词按 metadata 的 time_basis / prompt_note 口径复原。
