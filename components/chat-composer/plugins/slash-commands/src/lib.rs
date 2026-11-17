//! E5a 插件 1：slash-commands——斜杠命令走 Commands 服务，候选走 Slash 触发源。
//!
//! 语义（与 TS demo 的 DEMO_COMMANDS 对齐，但从宿主硬编码改为插件贡献）：
//! - `/clear`：清空当前草稿（宿主消费 `{"action": "clear-draft"}`）；
//! - `/help`：返回已注册命令清单（handler 构造时持有 Commands 服务，
//!   演示「插件可以反射宿主命令表」——dsh-deja 的命令注册同形）。
//! 候选来源：Slash 触发 → `/help` `/clear` 两项。
//! 全部经 Scope 注册为可逆副作用——Fiber 卸载时统一回收。

use composer_plugin_api::{
    CandidateDto, CommandHandler, CompletionSource, FiberId, Loader, Mountable, TriggerKind,
};
use serde_json::json;
use std::sync::Arc;

/// `/clear`：产出清空草稿的宿主动作指令。
pub struct ClearDraft;
impl CommandHandler for ClearDraft {
    fn invoke(&self, _payload: &serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(json!({ "action": "clear-draft" }))
    }
}

/// `/help`：列出宿主命令表（构造时持有 Commands 服务，反射 ids）。
pub struct Help {
    commands: Arc<composer_plugin_api::Commands>,
}
impl CommandHandler for Help {
    fn invoke(&self, _payload: &serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(json!({ "action": "show-help", "commands": self.commands.ids() }))
    }
}

/// Slash 触发候选源：`/` → 两项（/help、/clear）。
pub fn slash_completion_source() -> CompletionSource {
    CompletionSource {
        name: "slash-commands".into(),
        triggers: vec![TriggerKind::Slash],
        fetch: Arc::new(|query| {
            let known = [("/help 帮助", "/help"), ("/clear 清理上下文", "/clear")];
            known
                .iter()
                .filter(|(label, insert)| {
                    label.to_lowercase().contains(&query.to_lowercase())
                        || insert.to_lowercase().contains(&query.to_lowercase())
                })
                .map(|(label, insert)| CandidateDto {
                    label: (*label).into(),
                    insert_text: (*insert).into(),
                })
                .collect()
        }),
    }
}

/// 斜杠命令插件（cordis Fiber 形态）。
pub struct SlashCommandsPlugin;

impl SlashCommandsPlugin {
    pub const ID: &'static str = "slash-commands";
}

impl Mountable for SlashCommandsPlugin {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn mount(&self, loader: &mut Loader) -> Result<FiberId, String> {
        loader.mount(composer_plugin_api::fiber(Self::ID), |scope| {
            scope.register_command("slash.clear", Arc::new(ClearDraft))?;
            scope.register_command(
                "slash.help",
                Arc::new(Help { commands: scope.commands() }),
            )?;
            scope.register_completion_source(slash_completion_source());
            Ok(())
        })
    }
}
