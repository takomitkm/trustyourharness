# 调研：常见 IM 的会话存储与防膨胀机制

- **调研日期**：2026-11-19
- **动机**：为 composer 会话存储设计找"对话过长导致占用过大、加载/渲染卡顿"的成熟解法
- **证据级别**：
  - **已核实**（当日抓取官方文档原话）：Matrix Synapse state groups（URL 见下）
  - **公开资料级**（常识级、未逐条复核）：Telegram/TDLib、Discord、Signal、WhatsApp、
    iMessage、XMPP MAM、Slack
  - **逆向推断**（社区逆向资料，细节各家说法略有出入）：微信、QQ NT
  - **存疑**：ICQ（2024-06 关停，细节凭旧资料）
- **口径说明**：`ipc` 按 **ICQ** 理解（老牌 IM）；如指进程间 IPC 或其他，需另补

## 全景表

| 家 | 底座 | 防膨胀机制 | 最值得偷的部分 |
|---|---|---|---|
| Matrix | 服务器 PostgreSQL/SQLite；客户端只持有窗口 | ① state group delta 压缩（见下）② timeline 有界窗口+分页 token，永不全量加载 ③ `m.room.retention` 把保留策略做成协议字段 | delta+周期快照思想；retention 字段 |
| Telegram | **服务器权威**，本地 TDLib 库是**可逐出缓存** | 本地缓存可设上限、LRU 逐出；`pts/qts/seq` 增量同步（序号缺口确定性补拉）；**stripped thumbnail**（消息内嵌几十字节迷你预览，列表先渲染后取图）；`offset_id` 锚点分页 | 内嵌迷你预览；"本地只是缓存"的姿态 |
| Discord | 服务器；客户端 Electron（IndexedDB/LevelDB 缓存） | 按需拉取窗口；**snowflake id**（id 高位编码毫秒时间戳→排序键/去重键/分页锚三合一） | id 即时间的编码 |
| Signal | 本地 SQLCipher 单库 | 附件出列：媒体放独立目录，消息行只留指针+缩略图 | 附件出列 |
| WhatsApp | 本地 msgstore.db（加密） | 媒体全在 `Media/` 目录，库内只有路径和缩略图；媒体独立老化清理 | 附件出列（同上） |
| 微信 | 本地 EnMicroMsg.db（SQLCipher）*（逆向推断）* | 消息表+独立 FTS 全文表；图片以 dat 混淆文件落盘，库内只存引用 | FTS 独立成表 |
| QQ NT | 本地 nt_msg.db（SQLite）*（逆向推断）* | 消息按会话分片；富媒体独立缓存目录 | 无特别可取 |
| iMessage | 本地 chat.db（SQLite） | 范式化 schema：chat/message/handle/attachment + join 表，附件路径在 attachment 表 | 范式化关系模型 |
| XMPP (MAM) | 服务器归档 | RSM 分页：按 stanza-id `after/before/max` 翻页，响应带 **`complete` 标志**明确"到底了" | complete 标志（分页有终点） |
| Slack | 全服务器 | 客户端零历史负担；导出格式（按天每频道 JSON 打 zip + users.json）是不错的冷档形状 | 冷档导出形状 |
| IRC | 纯文本日志，按天/频道轮转 | 无结构无索引——反例 | 无 |
| ICQ | 早年私有 .db/.idx（ODB），后期 SQLite *（存疑）* | 无 | 无 |

## 已核实的关键证据

**Matrix state group delta 压缩**（Synapse 官方文档，2026-11-19 抓取）：
<https://matrix-org.github.io/synapse/latest/usage/administration/state_groups.html>

> "for each state group, we normally store the **difference** between the state in that
> group and some other state group"

父链记在 `state_group_edges`；**"only occasionally (every 100 state changes or so) record
the full state"**——增量为主、约每 100 次状态变化落一次全量快照；另有
`rust-synapse-compress-state` 工具事后压缩（"dramatic reductions"，无数值）。动机：
房间状态（成员/权限/别名）几乎不变，每事件全量抄状态表是纯浪费。

**Telegram 的姿态**：客户端库不是"数据"而是"缓存"。历史在服务器，本地只留窗口+上限，
超出即逐出；pts 序号保证"我看到了哪、缺了哪"，缺口一次 `getDifference` 补齐，不靠遍历对账。
（公开资料级，未逐条复核源码。）

## 关键发现："过长卡顿"是三个问题

| 问题 | 谁解决得好 | 手段 |
|---|---|---|
| **磁盘占用**随历史线性涨 | Telegram / Signal / Matrix | 载荷出列（媒体不进消息行）、冷档压缩、retention 清理 |
| **加载卡顿**（打开会话/滚动） | Element / Telegram / Discord | **有界窗口**：任何时刻只持有 N 条，往上滚才拉前一页；锚点定位，不数总数 |
| **内存卡顿**（渲染/解析全史） | Element | 虚拟化列表：只实例化窗口内对象；冷数据流式解码 |

单机 IM（微信/QQ/Signal/iMessage/WhatsApp）没有服务器兜底，全靠**附件出列 + 索引**把
"库文件"和"大文件"切开——这是它们能撑十年聊天记录的原因。

## 设计结论

组合方案见 [session-storage-design.md](session-storage-design.md)。
