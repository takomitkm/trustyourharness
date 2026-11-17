//! file-completion 原生测试：装载→Reference 触发候选→卸载回收；与
//! attachment-tools 的同触发来源互不干扰（多来源合并）。

use composer_plugin_api::{CompletionSource, Loader, Mountable, TriggerKind};
use file_completion::FileCompletionPlugin;
use std::sync::Arc;

#[test]
fn mount_registers_reference_source() {
    let mut loader = Loader::new();
    let fiber = FileCompletionPlugin.mount(&mut loader).expect("mount");
    assert_eq!(loader.fiber_status(fiber), Some(composer_plugin_api::FiberStatus::Active));

    let sources = loader
        .context()
        .get::<composer_plugin_api::CompletionSources>()
        .expect("sources")
        .for_trigger(TriggerKind::Reference);
    assert_eq!(sources.len(), 1);
    let all = (sources[0].fetch)("");
    assert_eq!(all.len(), 5, "五个项目文件候选");
    let narrowed = (sources[0].fetch)("proj");
    assert_eq!(narrowed.len(), 1);
    assert_eq!(narrowed[0].label, "project-docs.md");
    assert_eq!(narrowed[0].insert_text, "@project-docs.md", "引用写回保留 @ 前缀");
    // 中文文件名匹配（to_lowercase 口径）。
    let cjk = (sources[0].fetch)("架构");
    assert_eq!(cjk.len(), 1);
    assert_eq!(cjk[0].insert_text, "@架构总览.md");
}

#[test]
fn coexists_with_same_trigger_source_then_retire_only_own() {
    let mut loader = Loader::new();
    // 先装一个 stub（贡献 Reference 来源 "attachment-files"）——多来源合并面：
    // 卸载只回收自己的贡献，别家来源保留（可逆副作用的按 Fiber 粒度）。
    composer_plugin_api::Mountable::mount(&StubAttachmentSource, &mut loader).expect("mount stub");
    let fiber = FileCompletionPlugin.mount(&mut loader).expect("mount file-completion");
    let sources = loader
        .context()
        .get::<composer_plugin_api::CompletionSources>()
        .expect("sources")
        .for_trigger(TriggerKind::Reference);
    assert_eq!(sources.len(), 2, "两个 Reference 来源并存");

    loader.retire(fiber);
    let sources = loader
        .context()
        .get::<composer_plugin_api::CompletionSources>()
        .expect("sources")
        .for_trigger(TriggerKind::Reference);
    assert_eq!(sources.len(), 1, "stub 来源保留");
    assert_eq!(sources[0].name, "attachment-files");
}

/// 最小 stub：只贡献一个 Reference 来源（测试聚焦多来源合并，不拉真 attachment-tools 依赖）。
struct StubAttachmentSource;
impl Mountable for StubAttachmentSource {
    fn id(&self) -> &'static str {
        "attachment-tools"
    }
    fn mount(&self, loader: &mut Loader) -> Result<composer_plugin_api::FiberId, String> {
        loader.mount(composer_plugin_api::fiber("attachment-tools"), |scope| {
            scope.completion_sources().register(CompletionSource {
                name: "attachment-files".into(),
                triggers: vec![TriggerKind::Reference],
                fetch: Arc::new(|query| {
                    vec![composer_plugin_api::CandidateDto {
                        label: format!("stub-{query}"),
                        insert_text: format!("stub-{query}"),
                    }]
                }),
            });
            Ok(())
        })
    }
}
