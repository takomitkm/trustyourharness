# OUT-024：E5c 完成——WIT 契约冻结 + WASM 插件载体（composer-wasm 6/6）

- **轮次**：goal 后第二十四次输出
- **日期**：2026-11-18

## 本轮做了什么

1. **WIT 契约冻结**（docs/architecture/composer.wit，`composer:plugin@0.1.0`）：
   command-count / command-id / alloc / invoke 四导出 + memory；语义=插件自带
   全部逻辑（dsh-deja 同款），宿主面薄。component-model 迁移时形状不变。
2. **guest 载体**（plugins/wasm-demo-guest，no_std + bump allocator）：echo/reverse
   两个演示命令。GlobalAlloc impl 必须显式写（漏了即 trait bound 报错——实录）。
3. **host 载体**（crates/composer-wasm）：WasmPluginHost + WasmPlugin（Mountable）
   + WasmCommandHandler（互斥委托）；可逆语义跨载体一致（retire 撤销注册）。
4. **payload 约定**：JSON 字符串值解包为原始文本；guest 非 JSON 结果包装为字符串值。
5. **跨机构建铁律补录**：解包 + touch + build + bindgen + run 同脚本（本轮实测
   "改了没生效"的第三种形态——运行脚本缺 cargo build，vendor 用旧 wasm）。

## 验证证据（校机）

- `e5c-final3.log`：composer-wasm 载体测试 **6/6**（装载/命令面/echo UTF-8 往返/
  reverse guest 侧逻辑/retire 回收/契约边界）+ plugin-api 10/0
- workspace **127/0**（wasmtime 依赖入工作区后的全量回归）

## E5 线状态

- E5a ✅ → E5b ✅ → **E5c ✅（本轮）** → E5d（dsh-deja tool 移植示范）待做
- goal-log：D001–D024 + OUT-001–OUT-024
