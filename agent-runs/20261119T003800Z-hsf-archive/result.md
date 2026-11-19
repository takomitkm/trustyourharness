# Run 结果报告：HSF 会话存储：规格原文逐字节存档 + 两页蒸馏（数据结构/协议书）

## 做了什么

hsf-data-model.md（16 表清单+Run≠ModelCall/Checkpoint≠Snapshot+ID/seq/时间规则+五架构决策）与 hsf-protocol.md（九条 MUST 契约+三条端到端验收）两页蒸馏；hsf-spec-1.0-draft-full.md 从粘贴附件逐字节复制存档（52,198 字节，cmp 验证一致）；session-storage-design.md 加指针串起六份文档；蒸馏稿页头声明"有偏差以原文为准"。

## 测试结果

见上（各验证数字为当次实录，工具链与判据细节在对应 goal-log/commit）；本条为回填复原，非实时报告。

## 已知问题

回填条目：时间与提示词按 metadata 的 time_basis / prompt_note 口径复原。
