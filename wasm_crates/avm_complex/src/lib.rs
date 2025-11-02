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
    fn avm_get_global_bytes(
        app: u64,
        key_ptr: *const u8,
        key_len: u32,
        dest_ptr: *mut u8,
        dest_len: u32,
    ) -> i32;
    fn avm_set_global_bytes(
        app: u64,
        key_ptr: *const u8,
        key_len: u32,
        src_ptr: *const u8,
        src_len: u32,
    );
    fn amv_get_global_var_uint(field_index: u64) -> u64;
}

const KEY: &[u8] = b"foo";
const VALUE_BYTES: &[u8] = b"Hello AVM!";

// export exactly "program" without keeping the Rust name table
#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    let app_id = unsafe { amv_get_global_var_uint(8) };
    let key_ptr = KEY.as_ptr();
    let key_len = KEY.len() as u32;

    // uint should start at 0
    let mut value = unsafe { avm_get_global_uint(app_id, key_ptr, key_len) };
    if value != 0 {
        wasm_panic();
    }

    unsafe { avm_set_global_uint(app_id, key_ptr, key_len, 7) };

    value = unsafe { avm_get_global_uint(app_id, key_ptr, key_len) };
    if value != 7 {
        wasm_panic();
    }

    unsafe { avm_set_global_uint(app_id, key_ptr, key_len, 0) };

    unsafe {
        avm_set_global_bytes(
            app_id,
            key_ptr,
            key_len,
            VALUE_BYTES.as_ptr(),
            VALUE_BYTES.len() as u32,
        );
    }

    let mut retrieved_value = [0u8; VALUE_BYTES.len()];

    let ret_len = unsafe {
        avm_get_global_bytes(
            app_id,
            key_ptr,
            key_len,
            retrieved_value.as_mut_ptr(),
            retrieved_value.len() as u32,
        )
    };
    if ret_len != VALUE_BYTES.len() as i32 {
        wasm_panic();
    }

    if &retrieved_value[..ret_len as usize] != VALUE_BYTES {
        wasm_panic();
    }

    0
}
