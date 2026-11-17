# D024：E5c——WIT 契约冻结 + WASM 插件载体（wasmtime）

- **日期**：2026-11-18
- **状态**：已落地（composer-wasm 载体测试 6/6 + workspace 127/0）
- **关联**：E5 计划；composer.wit（docs/architecture/composer.wit）；OUT-024

## 决定

1. **WIT 契约冻结**（composer.wit v0）：`composer:plugin@0.1.0` interface——
   `command-count` / `command-id` / `alloc` / `invoke(id, payload) -> result` +
   `memory` 导出。语义：插件自带全部逻辑（dsh-deja 同款），宿主只提供
   alloc + memory 写入。当前载体是 core-wasm 原始 ABI（NUL 字符串）；
   component-model 迁移时接口形状不变，仅编码升级。
2. **guest 载体**（plugins/wasm-demo-guest，wasm32-unknown-unknown，no_std）：
   bump allocator（128KB）+ 两个演示命令（echo/reverse）。**GlobalAlloc impl
   必须显式写**——只挂 `#[global_allocator]` 不写 `unsafe impl GlobalAlloc`
   会在 wasm 目标报 trait bound 不满足（E5c 实录）。
3. **host 载体**（crates/composer-wasm）：`WasmPluginHost`（engine/store/instance）
   + `WasmPlugin`（Mountable 形态，mount = 创建宿主 + 全部命令经
   `Scope::register_command` 注册——**可逆语义跨载体一致**：retire 撤销注册，
   wasmtime 实例随句柄释放）。
4. **payload 约定**：JSON 字符串值解包为原始文本传入（文本类命令的 reverse
   语义落在内容上而非 JSON 包装上）；guest 结果能按 JSON 解析则透传，
   否则包装为字符串值（纯文本 guest 不丢语义）。
5. **边界如实**：wasmtime 是原生库——本载体只在原生宿主可用（浏览器内不可
   运行 wasmtime，行为验证走原生测试）；web 面插件仍是 taino/Fiber 编译期形态。

## 排查实录（三轮编译迭代）

- guest 漏 `unsafe impl GlobalAlloc`（只挂 allocator 属性）；
- wasmtime 新 API：`get_typed_func` 要求 `AsContextMut` by-value（`&mut self.store`
  而非 `&self.store`）、单参数 call 不用元组；
- `Mountable` 从 composer_plugin_api 导入（composer_wasm 自身 re-export 无）。

## 后果

- WASM 插件装载进同一 Loader 的可逆语义实证：retire 撤销 WASM 注册 ✓。
- 契约输入齐备 → E5d（dsh-deja tool 移植示范）直接复用本载体纪律。
