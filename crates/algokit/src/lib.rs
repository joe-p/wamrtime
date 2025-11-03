#[cfg(target_arch = "wasm32")]
use core::panic::PanicInfo;

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    core::arch::wasm32::unreachable();
}

const MAX_GLOBAL_VALUE_SIZE: usize = 128;

pub fn panic() -> ! {
    #[cfg(target_arch = "wasm32")]
    core::arch::wasm32::unreachable();

    #[cfg(not(target_arch = "wasm32"))]
    panic!();
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
    fn avm_get_global_var_uint(field_index: u64) -> u64;
}

pub fn get_global_uint(app: u64, key: &[u8]) -> u64 {
    unsafe { avm_get_global_uint(app, key.as_ptr(), key.len() as u32) }
}

pub fn set_global_uint(app: u64, key: &[u8], value: u64) {
    unsafe { avm_set_global_uint(app, key.as_ptr(), key.len() as u32, value) };
}

pub fn get_global_bytes(app: u64, key: &[u8]) -> Vec<u8> {
    let mut buf = [0u8; MAX_GLOBAL_VALUE_SIZE];
    let ret_len = unsafe {
        avm_get_global_bytes(
            app,
            key.as_ptr(),
            key.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        ) as usize
    };

    buf[..ret_len].to_vec()
}

pub fn set_global_bytes(app: u64, key: &[u8], src: &[u8]) {
    unsafe {
        avm_set_global_bytes(
            app,
            key.as_ptr(),
            key.len() as u32,
            src.as_ptr(),
            src.len() as u32,
        )
    };
}

#[repr(u64)]
pub enum GlobalVar {
    AppID = 8,
}

pub fn get_global_var_uint(field_index: GlobalVar) -> u64 {
    unsafe { avm_get_global_var_uint(field_index as u64) }
}
