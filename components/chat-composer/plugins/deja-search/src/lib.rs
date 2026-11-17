//! E5d 插件：deja-search——dsh-deja 的 session-search tool 按 composer 契约重写
//! （移植示范，D025）。
//!
//! 语义（dsh-deja 同款）：
//! - 二进制解析：env `DEJA_BIN` 优先，其次 PATH 上的 `deja`；候选必须通过
//!   `deja version` 验证（名字存在但跑不动比没有更糟——dsh-deja resolveDeja 同款）；
//! - **降级纪律**：解析失败 → 插件跳过注册（诚实缺席，不伪造功能）——宿主
//!   capability 面不缺失，只是该插件的命令不出现；
//! - `deja.version` / `deja.search` 两个命令：子进程调用，stdout 作为结果。
//!
//! 与 dsh-deja 原版的差异：原版注册 model-facing tools（defineTool），本插件注册
//! Commands（composer 的命令面）；原版依赖 dsh-sessions，本插件无会话树依赖。

use composer_plugin_api::{CommandHandler, FiberId, Loader, Mountable};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

/// 解析 deja 二进制：env 优先 → PATH → 验证（`deja version` 能跑才算存在）。
pub fn resolve_deja() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(from_env) = std::env::var("DEJA_BIN") {
        candidates.push(PathBuf::from(from_env));
    }
    candidates.push(PathBuf::from(if cfg!(windows) { "deja.exe" } else { "deja" }));
    for candidate in candidates {
        if runs_and_prints_version(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn runs_and_prints_version(path: &PathBuf) -> bool {
    Command::new(path)
        .arg("version")
        .creation_flags_no_window()
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

trait CreationFlagsNoWindow {
    fn creation_flags_no_window(&mut self) -> &mut Self;
}

/// Windows：隐藏控制台窗口（本地/校机实测——弹窗扰民）。
#[cfg(windows)]
impl CreationFlagsNoWindow for Command {
    fn creation_flags_no_window(&mut self) -> &mut Self {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        use std::os::windows::process::CommandExt;
        self.creation_flags(CREATE_NO_WINDOW)
    }
}
#[cfg(not(windows))]
impl CreationFlagsNoWindow for Command {
    fn creation_flags_no_window(&mut self) -> &mut Self {
        self
    }
}

fn run_deja(deja: &PathBuf, args: &[&str]) -> Result<String, String> {
    let output = Command::new(deja)
        .args(args)
        .creation_flags_no_window()
        .output()
        .map_err(|e| format!("deja 子进程失败: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "deja 退出码 {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// deja-search 插件（cordis Mountable 形态）：降级纪律——解析失败则跳过注册。
pub struct DejaSearchPlugin;

impl DejaSearchPlugin {
    pub const ID: &'static str = "deja-search";
}

impl Mountable for DejaSearchPlugin {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn mount(&self, loader: &mut Loader) -> Result<FiberId, String> {
        let Some(deja) = resolve_deja() else {
            // 诚实缺席：不注册（宿主 capability 面不缺失，只是命令不出现）。
            return Ok(loader.mount(composer_plugin_api::fiber(Self::ID), |_scope| {
                Ok(())
            })?);
        };
        loader.mount(composer_plugin_api::fiber(Self::ID), move |scope| {
            struct VersionHandler(PathBuf);
            impl CommandHandler for VersionHandler {
                fn invoke(&self, _payload: &serde_json::Value) -> Result<serde_json::Value, String> {
                    let out = run_deja(&self.0, &["version"])?;
                    Ok(serde_json::json!({ "version": out }))
                }
            }
            struct SearchHandler(PathBuf);
            impl CommandHandler for SearchHandler {
                fn invoke(&self, payload: &serde_json::Value) -> Result<serde_json::Value, String> {
                    let query = payload
                        .get("query")
                        .and_then(|v| v.as_str())
                        .ok_or("缺少 query")?;
                    let out = run_deja(&self.0, &["search", query])?;
                    Ok(serde_json::from_str(&out).unwrap_or(serde_json::Value::String(out)))
                }
            }
            scope.register_command("deja.version", Arc::new(VersionHandler(deja.clone())))?;
            scope.register_command("deja.search", Arc::new(SearchHandler(deja.clone())))?;
            Ok(())
        })
    }
}

