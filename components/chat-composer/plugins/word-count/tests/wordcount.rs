//! word-count 原生测试：装载→命令计算正确→槽位贡献在案→卸载回收。

use composer_plugin_api::Mountable;
use composer_plugin_api::{CommandHandler, Loader, SlotId};
use word_count::{WordCount, WordCountPlugin, SLOT_EDITOR_STATUS};

#[test]
fn command_counts_chars_and_words() {
    let wc = WordCount;
    // 纯 ASCII。
    let out = wc.invoke(&serde_json::json!({ "text": "hello world" })).expect("invoke");
    assert_eq!(out["chars"], 11u64);
    assert_eq!(out["words"], 2u64);
    // 中文按字符计（CJK 无空格 → words 近似口径如实）。
    let out = wc.invoke(&serde_json::json!({ "text": "你好世界" })).expect("invoke");
    assert_eq!(out["chars"], 4u64);
    assert_eq!(out["words"], 1u64);
    // 空文本。
    let out = wc.invoke(&serde_json::json!({ "text": "" })).expect("invoke");
    assert_eq!(out["chars"], 0u64);
    assert_eq!(out["words"], 0u64);
}

#[test]
fn mount_registers_command_and_status_slot() {
    let mut loader = Loader::new();
    let fiber = WordCountPlugin.mount(&mut loader).expect("mount");
    assert_eq!(loader.fiber_status(fiber), Some(composer_plugin_api::FiberStatus::Active));

    let commands = loader
        .context()
        .get::<composer_plugin_api::Commands>()
        .expect("commands");
    assert!(commands.handler("wordcount.current").is_some());

    let slots = loader.context().get::<composer_plugin_api::Slots>().expect("slots");
    let views = slots.slot_views(SLOT_EDITOR_STATUS);
    assert_eq!(views.len(), 1, "editor.status 一个贡献");
    assert_eq!(views[0].component, "word-count");
}

#[test]
fn retire_removes_command_and_slot() {
    let mut loader = Loader::new();
    let fiber = WordCountPlugin.mount(&mut loader).expect("mount");
    loader.retire(fiber);
    let commands = loader
        .context()
        .get::<composer_plugin_api::Commands>()
        .expect("commands");
    assert!(commands.handler("wordcount.current").is_none(), "命令已回收");
    let slots = loader.context().get::<composer_plugin_api::Slots>().expect("slots");
    assert!(slots.slot_views(SLOT_EDITOR_STATUS).is_empty(), "槽位贡献已回收");
}
