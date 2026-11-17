//! E5c WASM guest 插件（wasm32-unknown-unknown，no_std + bump allocator）。
//!
//! 契约 = docs/architecture/composer.wit v0：
//! exports: memory / alloc(len) / command_count() / command_id(index) /
//!          invoke(id_ptr, payload_ptr, payload_len)。
//! 字符串 = NUL 结尾 UTF-8（指针指向插件线性内存；宿主经 alloc 写入 id/payload）。
//! 全部逻辑在插件侧（dsh-deja 同款：宿主面薄）——echo / reverse 两个演示命令。

#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::panic::PanicInfo;

extern crate alloc;
use alloc::format;
use alloc::string::String;

const HEAP_SIZE: usize = 128 * 1024;

/// bump allocator：只增不减（插件内存随调用增长，v0 简化；测试量级远低于上限）。
struct Bump {
    heap: UnsafeCell<[u8; HEAP_SIZE]>,
    top: UnsafeCell<usize>,
}
unsafe impl Sync for Bump {}

impl Bump {
    fn bump_alloc(&self, len: usize) -> *mut u8 {
        unsafe {
            let base = self.heap.get() as *mut u8;
            let top = *self.top.get();
            let aligned = (top + 15) & !15;
            if aligned + len > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            *self.top.get() = aligned + len;
            base.add(aligned)
        }
    }
}

unsafe impl GlobalAlloc for Bump {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.bump_alloc(layout.size() + layout.align())
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: Bump = Bump {
    heap: UnsafeCell::new([0; HEAP_SIZE]),
    top: UnsafeCell::new(0),
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// 泄漏 NUL 结尾字符串到线性内存，返回指针。
fn leak_str(s: &str) -> i32 {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    let ptr = ALLOCATOR.bump_alloc(bytes.len());
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    }
    ptr as i32
}

const COMMANDS: [&str; 2] = ["wasm.echo", "wasm.reverse"];

#[no_mangle]
pub extern "C" fn alloc(len: i32) -> i32 {
    ALLOCATOR.bump_alloc(len.max(0) as usize) as i32
}

#[no_mangle]
pub extern "C" fn command_count() -> i32 {
    COMMANDS.len() as i32
}

#[no_mangle]
pub extern "C" fn command_id(index: i32) -> i32 {
    match COMMANDS.get(index as usize) {
        Some(id) => leak_str(id),
        None => 0,
    }
}

fn read_c_str(ptr: i32) -> String {
    unsafe {
        let mut p = ptr as *const u8;
        let mut out = String::new();
        while *p != 0 {
            out.push(*p as char);
            p = p.add(1);
        }
        out
    }
}

fn read_bytes(ptr: i32, len: usize) -> String {
    unsafe {
        let slice = core::slice::from_raw_parts(ptr as *const u8, len);
        String::from_utf8_lossy(slice).into_owned()
    }
}

#[no_mangle]
pub extern "C" fn invoke(id_ptr: i32, payload_ptr: i32, payload_len: i32) -> i32 {
    let id = read_c_str(id_ptr);
    let payload = read_bytes(payload_ptr, payload_len as usize);
    let result = match id.as_str() {
        "wasm.echo" => payload,
        "wasm.reverse" => payload.chars().rev().collect(),
        _ => format!("{{\"error\":\"unknown command: {}\"}}", id),
    };
    leak_str(&result)
}
