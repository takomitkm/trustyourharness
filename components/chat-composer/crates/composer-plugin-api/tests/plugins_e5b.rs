//! E5b 原生测试：Fiber 装载 config + PromptContext 注册/回收（cordis patch 同形）。

use composer_plugin_api::{fiber_with_config, Loader, Mountable};

/// 带 config 装载的示范插件：Scope::config 读取自己的配置。
struct ConfiguredPlugin {
    expected_key: &'static str,
}

impl Mountable for ConfiguredPlugin {
    fn id(&self) -> &'static str {
        "configured"
    }
    fn mount(&self, loader: &mut Loader) -> Result<composer_plugin_api::FiberId, String> {
        loader.mount(fiber_with_config("configured", serde_json::json!({ "threshold": 42 })), |scope| {
            let config = scope.config().expect("config present");
            assert_eq!(config["threshold"], 42, "Fiber 装载 config 可读");
            Ok(())
        })
    }
}

#[test]
fn fiber_spec_carries_config_into_scope() {
    let mut loader = Loader::new();
    let spec = fiber_with_config("configured", serde_json::json!({ "threshold": 42 }));
    let fiber = loader
        .mount(spec, |scope| {
            let config = scope.config().expect("config present");
            assert_eq!(config["threshold"], 42, "Fiber 装载 config 可读");
            Ok(())
        })
        .expect("mount");
    assert_eq!(loader.fiber_status(fiber), Some(composer_plugin_api::FiberStatus::Active));
}

/// PromptContext 注册插件：系统提示上下文片段（可逆）。
struct PromptPlugin;

impl Mountable for PromptPlugin {
    fn id(&self) -> &'static str {
        "prompt-plugin"
    }
    fn mount(&self, loader: &mut Loader) -> Result<composer_plugin_api::FiberId, String> {
        loader.mount(composer_plugin_api::fiber("prompt-plugin"), |scope| {
            scope.register_prompt_context(serde_json::json!({
                "style": "concise",
                "maxChars": 2000,
            }));
            Ok(())
        })
    }
}

#[test]
fn prompt_context_registers_and_retires() {
    let mut loader = Loader::new();
    let fiber = PromptPlugin.mount(&mut loader).expect("mount");

    let prompt = loader
        .context()
        .get::<composer_plugin_api::PromptContext>()
        .expect("prompt context service");
    let contexts = prompt.contexts();
    assert_eq!(contexts.len(), 1, "一个上下文片段");
    assert_eq!(contexts[0].0, "prompt-plugin");
    assert_eq!(contexts[0].1["style"], "concise");

    // 可逆：retire 后片段消失。
    loader.retire(fiber);
    assert!(prompt.contexts().is_empty(), "retire 后上下文回收");
}

#[test]
fn fiber_spec_config_defaults_to_none() {
    let spec = composer_plugin_api::fiber("plain");
    assert!(spec.config.is_none(), "无 config 装载 = None");
}
