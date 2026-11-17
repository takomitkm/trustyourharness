# OUT-022：E5a 完成——原生插件生态（4 插件 + 可逆注册 API 修复 + 七套 105+118 全绿）

- **轮次**：goal 后第二十二次输出
- **日期**：2026-11-17

## 本轮做了什么

1. **三个原生 Fiber 插件**（slash-commands / file-completion / word-count）：
   候选源、命令、槽位贡献全经 Scope 可逆注册；原生测试 7 项（含 retire 回收断言）。
2. **composer-plugin-api 真缺口修复**（E5a 逼出）：内建服务注册改为可逆副作用——
   `unregister` 三方法 + `Scope::register_command/register_completion_source/register_view`
   （注册即登记 Disposer）；attachment-tools 同步改道。
3. **app-web 插件接线**：Loader.mount ×4 + 候选菜单消费 CompletionSources（多来源
   合并 + TriggerKind 双类型映射）+ / 命令面（dispatch 前拦截）+ word-count 状态栏。
4. **仓库改名**：betterharness → **trustyourharness**（API PATCH 执行，本地 remote
   与 main 均已对齐）。
5. **上游确认**：dioxus-desktop 适配器零排期；PR #24 open 等响应。

## 验证证据（校机）

- `e5a-full13.log`：**七套全绿**——workspace 118/0 + taino 18/18 + 菜单 14/14 +
  附件 12/12 + IDB 6/6 + island 24/24 + network 12/12 + **plugins 9/9**
- 三插件原生测试：2+2+3（含 retire 回收断言）

## E4a–E4d / E5a–E5d 排期状态

- E4（乐观确认）：计划已落（§十九），**待做**
- E5a：✅ 本轮；E5b（PromptContext+config）/ E5c（WIT+WASM）/ E5d（dsh-deja 移植）：待做

## goal-log：D001–D022 + OUT-001–OUT-022
