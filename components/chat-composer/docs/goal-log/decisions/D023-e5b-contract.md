# D023：E5b——契约补全（PromptContext + Fiber 装载 config + 能力降级纪律）

- **日期**：2026-11-18
- **状态**：已落地（plugin-api 原生测试 10/0 + workspace 121/0）
- **关联**：E5 计划（dsh-deja 依赖面实测）；D022；OUT-023

## 决定

1. **PromptContext 服务**（≈ DSH `ctx.systemPrompt.context`）：插件按需注入系统提示
   上下文片段（JSON），宿主发送前按注册顺序组装。注册经 Scope 登记为可逆副作用
   （retire 自动撤销——`PromptContext::unregister(plugin)`）。
2. **FiberSpec 带装载 config**（cordis.patch.yml 的 id 定向 config 同形）：
   `fiber_with_config(plugin, config)` → `Scope::config()` 在 apply 内读取——
   插件按 config 调整行为，卸载即失效（随 Fiber 生命周期）。
3. **能力降级纪律**（dsh-deja 实测同款）：插件对缺失服务**降级不崩溃**——
   composer-host 的 `capability::<T>()` 显式报错，插件 mount 侧按需跳过对应功能
   （已有 cascade/capability 测试守卫）。

## 排查实录

- Fiber 结构体加字段后 mount 的初始化器漏 `config`——E0063；tar 同步链再断一轮
  （本地改+校机旧文件，解包顺序验证后修复）。

## 后果

- DSH 插件依赖面 6 项中 5 项已有对应（tools/commands/inject/events/config/prompt），
  仅 `connection/webServer`（HTTP 面）与 `sessions`（会话树）不适用（composer 无此域）。
- E5c（WIT 冻结 + WASM 载体）的契约输入齐备。
