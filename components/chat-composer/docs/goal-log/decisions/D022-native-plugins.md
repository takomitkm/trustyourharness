# D022：E5a——原生插件生态落地（三个 Fiber 插件 + host 装载 + 命令面）

- **日期**：2026-11-17
- **状态**：已落地（三插件原生 7 测试 + plugins E2E 9/9 + 六套回归全绿）
- **关联**：D021（composerTest 钩子→本线复用为注入面）；E5 计划文档；OUT-022

## 决定

1. **三个原生 Fiber 插件**（`plugins/`，workspace 自动收录）：
   - `slash-commands`：/clear（清草稿动作）+ /help（反射宿主命令表——插件持
     `Arc<Commands>` 演示"插件可反射注册面"）+ Slash 候选源；
   - `file-completion`：项目文件候选源（Reference 触发），`insert_text` 带 `@`
     前缀（@ 引用语义——TS demo 惯例，E5a 实录补正）；
   - `word-count`：`wordcount.current` 命令（chars/words 计算）+ `editor.status`
     槽位贡献——展示「无副作用计算 + 槽位」组合。
2. **可逆注册 API 补全**（composer-plugin-api 真缺口，E5a 逼出）：内建服务的
   register **不是可逆副作用**——retire 后 handler/来源/视图全部残留（attachment-tools
   同样带病）。修法对齐 EventBus 模式：`Commands::unregister` / `Slots::unregister_plugin` /
   `CompletionSources::unregister` + `Scope::register_command/register_completion_source/
   register_view`（注册即登记 Disposer，Fiber 回滚自动撤销）。
3. **host 装载与消费**（app-web 协程内）：Loader.mount ×4（含 attachment-tools）→
   候选菜单消费 `CompletionSources.for_trigger`（多来源合并，TriggerKind 跨 crate
   变体映射）/ 命令面（`/` 开头 Submit 在 **dispatch 前拦截**走 Commands——/clear
   清编辑器事务、/help 经 slash-help div 呈现）→ word-count 按需 invoke 更新状态栏。
4. **TriggerKind 双类型**：composer-completion 与 composer-plugin-api 各有 TriggerKind
   （独立 enum）——协程接线处按变体映射（Reference/Slash）。

## 排查实录

- **test 文件作用域三连**：Mountable/CommandHandler trait 未导入（.mount/.invoke
  E0599）、stub 内 CompletionSource/Arc 缺导入——测试与 lib 的导入面分开维护。
- **"改了没生效"两种新形态**：py replace 无 assert 静默未命中（wordcount 消费代码
  第一次没写进去）；tar 打包先于本地修改（判据文件旧版上校机）——解法：py replace
  一律 assert + tar 打包永远放命令链最后。
- E2E 流程语义坑：Escape 关菜单后**同一 token 继续打字会重开菜单**（detect_trigger
  允许重开语义）——Enter 又走 accept；流程改为打完整命令 → Escape → 直接 Enter。

## 后果

- Cordis 哲学从"架构同构"落到"插件生态实跑"：4 个插件（attachment-tools + 3 新）
  在同一 Loader 上装载/卸载/级联验证。
- E5b（PromptContext + config）/ E5c（WIT + WASM）的路由不变。
