#!no_std]
use core::alloc::{GlobalAlloc, Layout};

pub enum AlgokitError {
    BufferTooSmall,
}

unsafe extern "C" {
    fn host_malloc(size: u64) -> u64;
    fn host_free(ptr: u64);
}

struct WamrAlloc;

unsafe impl GlobalAlloc for WamrAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { host_malloc(layout.size() as u64) as *mut u8 }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { host_free(ptr as u64) }
    }
}

#[global_allocator]
static GA: WamrAlloc = WamrAlloc;

pub fn avm_panic() -> ! {
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

pub fn read_global_bytes<'buf>(
    app: u64,
    key: &[u8],
    buf: &'buf mut [u8],
) -> core::result::Result<&'buf [u8], AlgokitError> {
    let ret_len = unsafe {
        avm_get_global_bytes(
            app,
            key.as_ptr(),
            key.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        ) as usize
    };

    if ret_len > buf.len() {
        return Err(AlgokitError::BufferTooSmall);
    }

    Ok(&buf[..ret_len])
}

pub fn write_global_bytes(app: u64, key: &[u8], src: &[u8]) {
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

pub struct GlobalUint<ValueType = u64> {
    pub key: &'static [u8],
    phantom: core::marker::PhantomData<ValueType>,
}

impl<ValueType> GlobalUint<ValueType> {
    pub const fn new(key: &'static [u8]) -> Self {
        GlobalUint {
            key,
            phantom: core::marker::PhantomData,
        }
    }

    #[inline(always)]
    fn app_id(&self) -> u64 {
        get_global_var_uint(GlobalVar::AppID)
    }
}

impl<T> GlobalUint<T>
where
    T: From<u64>,
    T: Into<u64>,
{
    pub fn get(&self) -> T {
        let value = get_global_uint(self.app_id(), self.key);
        T::from(value)
    }

    pub fn set(&self, value: T) {
        set_global_uint(self.app_id(), self.key, value.into());
    }
}

pub struct GlobalBytes<ValueType = &'static [u8]> {
    pub key: &'static [u8],
    phantom: core::marker::PhantomData<ValueType>,
}

impl<ValueType> GlobalBytes<ValueType> {
    pub const fn new(key: &'static [u8]) -> Self {
        GlobalBytes {
            key,
            phantom: core::marker::PhantomData,
        }
    }

    #[inline(always)]
    fn app_id(&self) -> u64 {
        get_global_var_uint(GlobalVar::AppID)
    }
}

impl GlobalBytes<&[u8]> {
    pub fn try_read<'buf>(
        &self,
        buf: &'buf mut [u8],
    ) -> core::result::Result<&'buf [u8], AlgokitError> {
        read_global_bytes(self.app_id(), self.key, buf)
    }

    pub fn read<'buf>(&self, buf: &'buf mut [u8]) -> &'buf [u8] {
        match read_global_bytes(self.app_id(), self.key, buf) {
            Ok(v) => v,
            Err(_) => avm_panic(),
        }
    }

    pub fn write(&self, value: &[u8]) {
        write_global_bytes(self.app_id(), self.key, value);
    }
}

#[repr(u64)]
pub enum GlobalVar {
    AppID = 8,
}

pub fn get_global_var_uint(field_index: GlobalVar) -> u64 {
    unsafe { avm_get_global_var_uint(field_index as u64) }
}
