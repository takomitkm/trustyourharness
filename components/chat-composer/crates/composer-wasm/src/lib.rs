//! E5c WASM 插件载体：wasmtime 装载 `.wasm` 插件，把 exports 映射到 Commands 服务
//! （可逆副作用语义跨载体一致——retire 撤销注册后 wasmtime 实例随句柄释放）。
//!
//! ABI = composer.wit v0（docs/architecture/composer.wit）：guest 导出
//! memory / alloc / command_count / command_id / invoke，字符串 NUL 结尾 UTF-8。
//! host 只提供 alloc + memory 写入（载荷传入），全部逻辑在插件侧（dsh-deja 同款）。
//!
//! 边界：本载体是**原生宿主**面（wasmtime 是原生库）——浏览器内无法运行
//! wasmtime；插件行为验证走原生测试（校机）。

use composer_plugin_api::{CommandHandler, FiberId, Loader, Mountable};
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::AsContextMut;

/// 装载后的 WASM 插件宿主：engine/store/instance 三件套（互斥保护，供多命令共享）。
pub struct WasmPluginHost {
    engine: wasmtime::Engine,
    instance: wasmtime::Instance,
    store: wasmtime::Store<()>,
}

impl WasmPluginHost {
    pub fn from_path(path: &std::path::Path) -> Result<Self, String> {
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::from_file(&engine, path).map_err(|e| e.to_string())?;
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[])
            .map_err(|e| e.to_string())?;
        Ok(Self {
            engine,
            instance,
            store,
        })
    }

    fn memory(&mut self) -> wasmtime::Memory {
        self.instance
            .get_memory(&mut self.store, "memory")
            .expect("guest exports memory")
    }

    /// 读 NUL 结尾 UTF-8 字符串（线性内存，ptr 起）。
    fn guest_string(&mut self, ptr: i32) -> String {
        let memory = self.memory();
        let data = memory.data(&self.store);
        let start = ptr as usize;
        let end = data[start..]
            .iter()
            .position(|&b| b == 0)
            .map(|i| start + i)
            .unwrap_or(data.len());
        String::from_utf8_lossy(&data[start..end]).into_owned()
    }

    fn write_bytes(&mut self, ptr: i32, bytes: &[u8]) {
        let memory = self.memory();
        memory
            .write(&mut self.store, ptr as usize, bytes)
            .expect("memory write within guest alloc region");
    }

    /// 已注册命令 id（guest command_count + command_id）。
    pub fn command_ids(&mut self) -> Vec<String> {
        let count_func = self
            .instance
            .get_typed_func::<(), i32>(&mut self.store, "command_count")
            .expect("command_count export");
        let count = count_func.call(&mut self.store, ()).expect("call") as usize;
        let id_func = self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, "command_id")
            .expect("command_id export");
        (0..count as i32)
            .map(|i| {
                let ptr = id_func.call(&mut self.store, i).expect("call") as usize;
                let memory = self.memory();
                let data = memory.data(&self.store);
                let end = data[ptr..]
                    .iter()
                    .position(|&b| b == 0)
                    .map(|i| ptr + i)
                    .unwrap_or(data.len());
                String::from_utf8_lossy(&data[ptr..end]).into_owned()
            })
            .collect()
    }

    /// 调用 guest 命令：id/payload 写入 guest alloc 区，invoke 后读结果 JSON。
    pub fn invoke(&mut self, id: &str, payload: &str) -> Result<String, String> {
        let alloc_func = self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, "alloc")
            .expect("alloc export");
        let id_ptr = alloc_func
            .call(&mut self.store, (id.len() + 1) as i32)
            .map_err(|e| e.to_string())?;
        self.write_bytes(id_ptr, &[id.as_bytes(), &[0]].concat());
        let payload_ptr = alloc_func
            .call(&mut self.store, (payload.len() + 1) as i32)
            .map_err(|e| e.to_string())?;
        self.write_bytes(payload_ptr, &[payload.as_bytes(), &[0]].concat());
        let invoke_func = self
            .instance
            .get_typed_func::<(i32, i32, i32), i32>(&mut self.store, "invoke")
            .expect("invoke export");
        let result_ptr = invoke_func
            .call(
                &mut self.store,
                (id_ptr, payload_ptr, payload.len() as i32),
            )
            .map_err(|e| e.to_string())?;
        Ok(self.guest_string(result_ptr))
    }
}

/// WASM 命令代理：CommandHandler 调用委托进 wasmtime 实例（互斥保护）。
struct WasmCommandHandler {
    host: Arc<Mutex<WasmPluginHost>>,
    id: String,
}
impl CommandHandler for WasmCommandHandler {
    fn invoke(&self, payload: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut host = self.host.lock().expect("wasm host lock");
        // JSON 字符串值解包为原始文本（guest 的文本类命令按纯文本处理——reverse 语义
        // 才落在内容上而非 JSON 包装上）；其余 payload 按原样 JSON 文本传递。
        let payload_text = match payload {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let result = host.invoke(&self.id, &payload_text)?;
        // v0 原始 ABI：guest 结果是文本——能按 JSON 解析则透传，否则包装为字符串值
        //（JSON-in/JSON-out 对结构化插件成立；纯文本 guest 的结果经此约定不丢语义）。
        Ok(serde_json::from_str(&result).unwrap_or(serde_json::Value::String(result)))
    }
}

/// WASM 插件（cordis Mountable 形态）：装载 = 创建 wasmtime 宿主 + 把全部命令
/// 注册进 Commands（可逆：retire 撤销注册，wasmtime 实例随句柄释放）。
pub struct WasmPlugin {
    pub path: String,
}

impl WasmPlugin {
    pub const ID: &'static str = "wasm-demo";
}

impl Mountable for WasmPlugin {
    fn id(&self) -> &'static str {
        "wasm-demo"
    }

    fn mount(&self, loader: &mut Loader) -> Result<FiberId, String> {
        let host = Arc::new(Mutex::new(WasmPluginHost::from_path(
            std::path::Path::new(&self.path),
        )?));
        let ids = host.lock().expect("wasm host lock").command_ids();
        loader.mount(composer_plugin_api::fiber(Self::ID), move |scope| {
            for id in &ids {
                scope.register_command(
                    id,
                    Arc::new(WasmCommandHandler {
                        host: host.clone(),
                        id: id.clone(),
                    }),
                )?;
            }
            Ok(())
        })
    }
}
