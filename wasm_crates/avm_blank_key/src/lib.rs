#![no_std]

// The cfg attributes are needed because the default target is set per workspace,
// and we don't want to set the default target for every crate to wasm.

#[cfg(target_arch = "wasm32")]
use core::panic::PanicInfo;

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    core::arch::wasm32::unreachable();
}

fn wasm_panic() {
    #[cfg(target_arch = "wasm32")]
    core::arch::wasm32::unreachable();
}

unsafe extern "C" {
    fn avm_get_global_uint(app: u64, key_ptr: *const u8, key_len: u32) -> u64;
    fn avm_set_global_uint(app: u64, key_ptr: *const u8, key_len: u32, value: u64);

}

const APP_ID: u64 = 42;
const KEY: &[u8] = b"foo";

// export exactly "program" without keeping the Rust name table
#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    let key_ptr = KEY.as_ptr();
    let key_len = KEY.len() as u32;

    // uint should start at 0
    let mut value = unsafe { avm_get_global_uint(APP_ID, key_ptr, key_len) };
    if value != 0 {
        wasm_panic();
    }

    unsafe { avm_set_global_uint(APP_ID, key_ptr, key_len, 7) };

    value = unsafe { avm_get_global_uint(APP_ID, key_ptr, key_len) };
    if value != 7 {
        wasm_panic();
    }

    unsafe { avm_set_global_uint(APP_ID, key_ptr, key_len, 0) };

    0
}
