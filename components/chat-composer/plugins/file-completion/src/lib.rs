//! E5a 插件 2：file-completion——项目文件候选源（Reference 触发，独立于
//! attachment-tools 的 "attachment-files" 来源；多来源同触发由菜单合并——
//! CompletionSources::for_trigger 返回 Vec 的设计意图）。
//!
//! 候选表从宿主 demo 硬编码迁出（E5a：DEMO_REFERENCES 的插件化）——
//! 真实宿主接文件系统/上传列表，插件只贡献清单与匹配。

use composer_plugin_api::{CandidateDto, CompletionSource, FiberId, Loader, Mountable, TriggerKind};
use std::sync::Arc;

/// 项目文件候选源：`@` → 项目文档清单（含中文文件名——匹配走 contains）。
pub fn project_files_source() -> CompletionSource {
    CompletionSource {
        name: "project-files".into(),
        triggers: vec![TriggerKind::Reference],
        fetch: Arc::new(|query| {
            let known = [
                "project-docs.md",
                "design-notes.txt",
                "meeting-2026-11-14.md",
                "架构总览.md",
                "用户手册.pdf",
            ];
            let needle = query.to_lowercase();
            known
                .iter()
                .filter(|name| name.to_lowercase().contains(&needle))
                .map(|name| CandidateDto {
                    label: (*name).into(),
                    // @ 引用语义：写回保留 @ 前缀（TS demo insertText 同款惯例）。
                    insert_text: format!("@{name}"),
                })
                .collect()
        }),
    }
}

/// 项目文件补全插件（cordis Fiber 形态）。
pub struct FileCompletionPlugin;

impl FileCompletionPlugin {
    pub const ID: &'static str = "file-completion";
}

impl Mountable for FileCompletionPlugin {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn mount(&self, loader: &mut Loader) -> Result<FiberId, String> {
        loader.mount(composer_plugin_api::fiber(Self::ID), |scope| {
            scope.register_completion_source(project_files_source());
            Ok(())
        })
    }
}
