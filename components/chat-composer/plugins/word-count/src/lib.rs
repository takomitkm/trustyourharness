//! E5a 插件 3：word-count——编辑器状态槽的贡献者（字数/字符数统计）。
//!
//! 展示「槽位贡献不注册命令也能有用」：视图组件 `word-count` 由宿主渲染，
//! 数据经 `wordcount.current` 命令按需计算（宿主把当前草稿文本作为 payload 传入）。
//! 与 attachment-tools 的「命令 + 槽位」组合不同，这里是「命令（无副作用计算）+ 槽位」。

use composer_plugin_api::{CommandHandler, CommandId, FiberId, Loader, Mountable, SlotId, ViewContribution};
use serde_json::json;
use std::sync::Arc;

/// 编辑器状态槽（宿主在状态栏提供该挂载点）。
pub const SLOT_EDITOR_STATUS: &str = "editor.status";

/// 字数统计：chars = 字符数（含空格与换行，与多数编辑器口径一致）；
/// words = 空白分隔的词数（CJK 场景下等价于"分段数"的近似，演示口径）。
pub struct WordCount;
impl CommandHandler for WordCount {
    fn invoke(&self, payload: &serde_json::Value) -> Result<serde_json::Value, String> {
        let text = payload
            .get("text")
            .and_then(|value| value.as_str())
            .ok_or("缺少 text")?;
        Ok(json!({
            "chars": text.chars().count(),
            "words": text.split_whitespace().count(),
        }))
    }
}

/// 字数统计插件（cordis Fiber 形态）。
pub struct WordCountPlugin;

impl WordCountPlugin {
    pub const ID: &'static str = "word-count";
}

impl Mountable for WordCountPlugin {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn mount(&self, loader: &mut Loader) -> Result<FiberId, String> {
        loader.mount(composer_plugin_api::fiber(Self::ID), |scope| {
            scope.register_command("wordcount.current", Arc::new(WordCount))?;
            scope.register_view(
                Self::ID,
                ViewContribution {
                    slot: SlotId(SLOT_EDITOR_STATUS.into()),
                    component: "word-count".into(),
                    label: "字数".into(),
                    action: Some(CommandId("wordcount.current".into())),
                    props: json!({ "dataKey": "text" }),
                    order: 10,
                    plugin: String::new(),
                },
            );
            Ok(())
        })
    }
}

