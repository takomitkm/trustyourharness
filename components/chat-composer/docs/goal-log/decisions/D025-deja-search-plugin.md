# D025：E5d——dsh-deja tool 移植示范（deja-search 插件 + 降级纪律 + 并行 env 竞态）

- **日期**：2026-11-18
- **状态**：已落地（deja-search 原生测试 6/6 + workspace 133/0 + 七套 E2E 全绿）
- **关联**：E5 计划（dsh-deja 依赖面实测）；D023（契约补全）；OUT-025

## 决定

1. **dsh-deja 的 session-search tool 按契约重写**为 composer 插件 `deja-search`：
   - 二进制解析：env `DEJA_BIN` 优先 → PATH → **候选必须过 `deja version` 验证**
     （dsh-deja resolveDeja 同款——名字在 PATH 但跑不动比没有更糟）；
   - `deja.version` / `deja.search` 两个命令：子进程调用（Windows 隐藏控制台
     CREATE_NO_WINDOW——弹窗扰民纪律），stdout 透传（能按 JSON 解析则透传）。
2. **降级纪律 = 诚实缺席**：resolve 失败 → mount 成功但命令不注册（dsh-deja 的
   「只有 model-facing tools are skipped」同款）。宿主 capability 面不缺失。
3. **并行 env 竞态实录**：DEJA_BIN 是进程级全局——mount_with_missing（坏路径）与
   mount_with_stub（好路径）并行互踩 → 测试互斥串行化（DEJA_BIN_LOCK）。
   通用教训：**测试触碰进程级 env 时必须互斥或唯一 scope**（与 taino 系的唯一
   scope 隔离同源）。

## 后果

- E5 线收官：E5a（原生插件）→ E5b（契约补全）→ E5c（WIT+WASM 载体）→
  **E5d（dsh-deja 移植示范）**——「DSH 插件逻辑可移植到 composer」从论证变为
  实证（subprocess 面 + 降级纪律 + 命令注册全链）。
- deja 真实索引数据的 E2E 在部署机（本机）才有——校机只验证解析/降级面（如实）。
