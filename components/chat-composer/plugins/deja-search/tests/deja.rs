//! E5d 原生测试：deja-search 插件的移植面 + 降级纪律。
//!
//! 真实路径：DEJA_BIN 指向 stub 二进制（CARGO_BIN_EXE 机制）——version/search 子进程
//! 真跑、解析罐头 JSON。降级路径：resolve 失败 → 插件注册跳过（诚实缺席）。

use composer_plugin_api::{Loader, Mountable};
use deja_search::DejaSearchPlugin;
use std::sync::Mutex;

/// DEJA_BIN 是进程级全局：并行测试互踩 env（mount_with_missing 与 stub 两条并行
/// 竞态实录）——互斥串行化所有触碰 DEJA_BIN 的测试。
static DEJA_BIN_LOCK: Mutex<()> = Mutex::new(());
use std::sync::atomic::{AtomicU64, Ordering};

fn unique_scope() -> String {
    format!("deja-test-{}", AtomicU64::new(rand_suffix()).fetch_add(1, Ordering::SeqCst))
}

fn rand_suffix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0)
}

fn stub_bin() -> &'static str {
    env!("CARGO_BIN_EXE_deja-stub")
}

#[test]
fn stub_bin_runs_version() {
    // stub 自身可用性守卫（测试前置）。
    let out = std::process::Command::new(stub_bin())
        .arg("version")
        .output()
        .expect("stub runs");
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("deja-stub"));
}

#[test]
#[test]
fn mount_with_missing_deja_degrades_to_empty_commands() {
    let _guard = DEJA_BIN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 降级纪律：二进制不存在（DEJA_BIN 指向不存在文件）→ mount 成功但命令不注册。
    unsafe {
        std::env::set_var("DEJA_BIN", "Z:/nonexistent/deja.exe");
    }
    let mut loader = Loader::new();
    let fiber = DejaSearchPlugin.mount(&mut loader).expect("mount");
    assert_eq!(loader.fiber_status(fiber), Some(composer_plugin_api::FiberStatus::Active));
    let commands = loader.context().get::<composer_plugin_api::Commands>().expect("commands");
    assert!(
        commands.handler("deja.search").is_none() && commands.handler("deja.version").is_none(),
        "诚实缺席：命令不注册"
    );
}

#[test]
fn mount_with_stub_resolves_and_registers() {
    let _guard = DEJA_BIN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::set_var("DEJA_BIN", stub_bin());
    }
    let mut loader = Loader::new();
    let fiber = DejaSearchPlugin.mount(&mut loader).expect("mount");
    let commands = loader.context().get::<composer_plugin_api::Commands>().expect("commands");
    assert!(commands.handler("deja.search").is_some(), "解析成功 → 命令注册");
    assert!(commands.handler("deja.version").is_some());

    // 子进程真跑：stub 输出罐头 JSON → 命令结果透传。
    let handler = commands.handler("deja.search").expect("search handler");
    let out = handler
        .invoke(&serde_json::json!({ "query": "hello" }))
        .expect("invoke");
    assert_eq!(out["query"], "hello", "query 回显");
    assert_eq!(out["hits"].as_array().map(|h| h.len()), Some(2), "罐头两命中");
}

#[test]
fn retire_removes_deja_commands() {
    let _guard = DEJA_BIN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::set_var("DEJA_BIN", stub_bin());
    }
    let mut loader = Loader::new();
    let fiber = DejaSearchPlugin.mount(&mut loader).expect("mount");
    loader.retire(fiber);
    let commands = loader.context().get::<composer_plugin_api::Commands>().expect("commands");
    assert!(commands.handler("deja.search").is_none(), "retire 回收");
}

#[test]
fn unique_scope_shape_documented() {
    // scope 形状记录（与 taino 系一致：唯一 scope 隔离会话）。
    assert!(unique_scope().starts_with("deja-test-"));
}
