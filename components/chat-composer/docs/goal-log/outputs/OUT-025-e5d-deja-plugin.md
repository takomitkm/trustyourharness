# OUT-025：E5d 完成——dsh-deja tool 移植示范（deja-search 插件 6/6）+ E5 线收官

- **轮次**：goal 后第二十五次输出
- **日期**：2026-11-18

## 本轮做了什么

1. **deja-search 插件**（plugins/deja-search）：dsh-deja 的 session-search tool
   按 composer 契约重写——二进制解析（env/PATH/验证）+ 降级纪律（诚实缺席）+
   deja.version / deja.search 两命令（子进程，Windows 隐藏控制台）。
2. **并行 env 竞态实录**：DEJA_BIN 进程级全局——并行测试互踩 → 互斥串行化
   （通用教训：测试触碰进程级 env 必须互斥或唯一 scope）。
3. **全量验证**：workspace **133/0**（含 deja-search 6 测试 + WASM 载体 6 测试）
   + **七套 E2E 95 判据全绿**。

## E5 线终态

| 阶段 | 状态 |
| --- | --- |
| E5a 原生插件生态 | ✓ 4 插件 + 可逆注册 API 修复 |
| E5b 契约补全 | ✓ PromptContext + config + 降级纪律 |
| E5c WIT+WASM 载体 | ✓ composer.wit 冻结 + wasmtime 载体 6/6 |
| E5d dsh-deja 移植示范 | ✓ deja-search 插件 6/6（降级纪律实证） |

「DSH 插件逻辑可移植到 composer」从论证（E5 调研的依赖面对应表）变为**全链实证**
（注册面/命令面/子进程面/降级面/载体面各有测试守卫）。

## goal-log：D001–D025 + OUT-001–OUT-025
