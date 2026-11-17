//! slash-commands 原生测试：装载→命令/候选生效→卸载→全部回收（cordis 可逆语义）。

use composer_plugin_api::Mountable;
use composer_plugin_api::{CommandHandler, Loader, TriggerKind};
use slash_commands::{Help, SlashCommandsPlugin};

#[test]
fn mount_registers_commands_and_source() {
    let mut loader = Loader::new();
    let fiber = SlashCommandsPlugin.mount(&mut loader).expect("mount");
    assert_eq!(loader.fiber_status(fiber), Some(composer_plugin_api::FiberStatus::Active));

    // 命令生效：clear 产出动作指令；help 反射命令表（含自己与 clear）。
    let commands = loader
        .context()
        .get::<composer_plugin_api::Commands>()
        .expect("commands");
    let clear = commands.handler("slash.clear").expect("clear handler");
    let out = clear.invoke(&serde_json::json!({})).expect("invoke");
    assert_eq!(out["action"], "clear-draft");

    let help = commands.handler("slash.help").expect("help handler");
    let out = help.invoke(&serde_json::json!({})).expect("invoke");
    let ids = out["commands"].as_array().expect("commands array");
    assert!(ids.iter().any(|v| v == "slash.clear"), "help 反射含 clear");
    assert!(ids.iter().any(|v| v == "slash.help"), "help 反射含自己");

    // 候选源生效：Slash 触发 → 两项；过滤缩窄。
    let sources = loader
        .context()
        .get::<composer_plugin_api::CompletionSources>()
        .expect("sources")
        .for_trigger(TriggerKind::Slash);
    assert_eq!(sources.len(), 1, "一个 Slash 来源");
    let candidates = (sources[0].fetch)("");
    assert_eq!(candidates.len(), 2);
    let narrowed = (sources[0].fetch)("cle");
    assert_eq!(narrowed.len(), 1);
    assert_eq!(narrowed[0].insert_text, "/clear");
}

#[test]
fn retire_recovers_all_registrations() {
    let mut loader = Loader::new();
    let fiber = SlashCommandsPlugin.mount(&mut loader).expect("mount");
    loader.retire(fiber);

    // 可逆回收：命令与候选源全部消失（Scope 登记的 Disposer 逆序执行）。
    let commands = loader
        .context()
        .get::<composer_plugin_api::Commands>()
        .expect("commands");
    assert!(commands.handler("slash.clear").is_none(), "clear 命令已回收");
    assert!(commands.handler("slash.help").is_none(), "help 命令已回收");
    let sources = loader
        .context()
        .get::<composer_plugin_api::CompletionSources>()
        .expect("sources")
        .for_trigger(TriggerKind::Slash);
    assert!(sources.is_empty(), "候选源已回收");
}
