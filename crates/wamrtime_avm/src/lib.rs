use std::ffi::c_void;
use std::ops::Deref;
use std::sync::LazyLock;

use wamrtime::{
    runtime::{WamrHostFunction, WamrType},
    runtime_thread::RuntimeThread,
};

const KB: usize = 1024;
const MAX_PROGRAM_SIZE: usize = 8 * KB;
const MAX_PROGRAM_DEPTH: usize = 256;

/// The size of the heap that the RUNTIME will use. This is separate from the module's linear
/// memory and is used for things like instantiated programs.
const RUNTIME_HEAP_SIZE: usize = (MAX_PROGRAM_SIZE + 1) * MAX_PROGRAM_DEPTH;

/// The WASM execution stack size. Note that most languages will use their own stack within linear memory.
const STACK_SIZE: u32 = 16 * KB as u32;

/// The managed heap is ADDED to the linear memory of the WASM module before defined __heap_base.
const MANAGED_HEAP_SIZE: usize = 128 * KB;

/// The maximum number of memory pages (64KB each) that a module can have. This is BEFORE adding the managed heap.
/// The total possible memory size is (MAX_MODULE_PAGES * 64KB) + MANAGED_HEAP_SIZE.
const MAX_MODULE_PAGES: u32 = 2;

static mut AVM_CTX: *mut c_void = core::ptr::null_mut();

static AVM_RUNTIME_THREAD: LazyLock<RuntimeThread> = LazyLock::new(|| {
    RuntimeThread::new(
        host_gas_check_impl,
        AVM_FUNCTIONS.iter().map(WamrHostFunction::from).collect(),
        RUNTIME_HEAP_SIZE,
        STACK_SIZE,
        MANAGED_HEAP_SIZE,
        MAX_MODULE_PAGES,
    )
});

macro_rules! avm_host_functions {
    (
        $(
            $fn_name:ident ( $($arg_name:ident : $arg_type:ty),* $(,)? ) $(-> $ret_type:ty)?
        );* $(;)?
    ) => {
        $(
            ::paste::paste! {
                pub type [<$fn_name:camel Fn>] = unsafe extern "C" fn(
                    exec_env: *mut wamrtime::wamr::WASMExecEnv,
                    ctx: *mut ::std::ffi::c_void,
                    $($arg_name: $arg_type),*
                ) $(-> $ret_type)?;

                static mut [<$fn_name:snake:upper _IMPL>]: Option<[<$fn_name:camel Fn>]> = None;

                extern "C" fn $fn_name(
                    exec_env: *mut wamrtime::wamr::WASMExecEnv,
                    $($arg_name: $arg_type),*
                ) $(-> $ret_type)? {
                    let impl_fn = unsafe {
                        [<$fn_name:snake:upper _IMPL>].expect(concat!("AVM ", stringify!($fn_name), " not set"))
                    };
                    let ctx = unsafe { AVM_CTX };
                    unsafe { impl_fn(exec_env, ctx, $($arg_name),*) }
                }
            }
        )*

        ::paste::paste! {
            #[unsafe(no_mangle)]
            pub extern "C" fn avm_init(
                $([<$fn_name _impl>]: [<$fn_name:camel Fn>]),*
            ) {
                let _ = AVM_RUNTIME_THREAD.deref();

                unsafe {
                    $(
                        [<$fn_name:snake:upper _IMPL>] = Some([<$fn_name _impl>]);
                    )*
                }
            }
        }
    };
}

avm_host_functions! {
    avm_get_global_uint(app: u64, key_ptr: *const u8, key_len: u32) -> u64;
    avm_set_global_uint(app: u64, key_ptr: *const u8, key_len: u32, value: u64);
    avm_get_global_bytes(app: u64, key_ptr: *const u8, key_len: u32, dest_ptr: *mut u8, dest_len: u32) -> i32;
    avm_set_global_bytes(app: u64, key_ptr: *const u8, key_len: u32, src_ptr: *const u8, src_len: u32);
    avm_get_global_var_uint(field_index: u64) -> u64;
}

enum AvmType {
    U64,
    App,
    ByteSlice,
    BytesLen,
    MutByteSlice,
}

impl From<&AvmType> for wamrtime::runtime::WamrType {
    fn from(avm_type: &AvmType) -> Self {
        match avm_type {
            AvmType::U64 => wamrtime::runtime::WamrType::I64,
            AvmType::App => wamrtime::runtime::WamrType::I64,
            AvmType::BytesLen => wamrtime::runtime::WamrType::I32,
            AvmType::ByteSlice => wamrtime::runtime::WamrType::ByteSlice,
            AvmType::MutByteSlice => wamrtime::runtime::WamrType::MutByteSlice,
        }
    }
}

pub struct AvmFunction {
    name: &'static str,
    args: &'static [AvmType],
    returns: Option<AvmType>,
    host_func: *mut c_void,
}

impl From<&AvmFunction> for wamrtime::runtime::WamrHostFunction {
    fn from(avm_func: &AvmFunction) -> Self {
        let args = avm_func.args.iter().map(WamrType::from).collect();

        wamrtime::runtime::WamrHostFunction::new(
            avm_func.name.to_string(),
            avm_func.host_func,
            Some(args),
            avm_func.returns.as_ref().map(WamrType::from),
        )
    }
}

const AVM_FUNCTIONS: &[AvmFunction] = &[
    AvmFunction {
        name: "avm_get_global_uint",
        args: &[AvmType::App, AvmType::ByteSlice],
        returns: Some(AvmType::U64),
        host_func: avm_get_global_uint as *mut c_void,
    },
    AvmFunction {
        name: "avm_set_global_uint",
        args: &[AvmType::App, AvmType::ByteSlice, AvmType::U64],
        returns: None,
        host_func: avm_set_global_uint as *mut c_void,
    },
    AvmFunction {
        name: "avm_get_global_bytes",
        args: &[AvmType::App, AvmType::ByteSlice, AvmType::MutByteSlice],
        returns: Some(AvmType::BytesLen),
        host_func: avm_get_global_bytes as *mut c_void,
    },
    AvmFunction {
        name: "avm_set_global_bytes",
        args: &[AvmType::App, AvmType::ByteSlice, AvmType::ByteSlice],
        returns: None,
        host_func: avm_set_global_bytes as *mut c_void,
    },
    AvmFunction {
        name: "avm_get_global_var_uint",
        args: &[AvmType::U64],
        returns: Some(AvmType::U64),
        host_func: avm_get_global_var_uint as *mut c_void,
    },
];

const GAS_LIMIT: i64 = 1_000_000;
static mut GAS_USED: i64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn host_gas_check_impl(_exec_env: *mut c_void, requested_gas: i64) {
    unsafe {
        GAS_USED += requested_gas;
        if GAS_USED > GAS_LIMIT {
            panic!("Out of gas");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn avm_set_ctx(ctx: *mut c_void) {
    unsafe {
        AVM_CTX = ctx;
    }
}

/// # Safety
/// We assume the exec_env and msg_ptr are valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn avm_set_exception(
    exec_env: *mut wamrtime::wamr::WASMExecEnv,
    msg_ptr: *const i8,
) {
    let module_inst = unsafe { wamrtime::wamr::wasm_runtime_get_module_inst(exec_env) };
    unsafe {
        wamrtime::wamr::wasm_runtime_set_exception(module_inst, msg_ptr);
    }
}

// Functions exposed for testing purposes

static TEST_MODULE_BYTES: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let raw_wasm_bytes = include_bytes!(
        "/Users/joe/git/joe-p/wamrtime/target/wasm32-unknown-unknown/wasm_small/state_loop.wasm"
    );

    let compiler = wamrtime::compiler::Compiler::new();
    compiler
        .compile_wasm(&mut raw_wasm_bytes.clone())
        .expect("should be able to compile test module")
});

#[unsafe(no_mangle)]
pub extern "C" fn test_avm_instrument_wasm() {
    let _ = TEST_MODULE_BYTES.deref();
}

#[unsafe(no_mangle)]
pub extern "C" fn test_avm_run_program() -> u64 {
    let bytes = TEST_MODULE_BYTES.clone();
    AVM_RUNTIME_THREAD.call_program(bytes)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{LazyLock, Mutex},
    };

    use std::time::Instant;

    use wamrtime::compiler::Compiler;

    use super::*;

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_host_function(_ctx: *mut c_void) {
        println!("Hello from Rust!");
    }

    static GLOBAL_STATE_UINTS: LazyLock<Mutex<HashMap<Vec<u8>, u64>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    static GLOBAL_STATE_BYTES: LazyLock<Mutex<HashMap<Vec<u8>, Vec<u8>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_impl_get_global_uint(
        _exec_env: *mut wamrtime::wamr::WASMExecEnv,
        _ctx: *mut c_void,
        _app: u64,
        key_ptr: *const u8,
        key_len: u32,
    ) -> u64 {
        let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len as usize) }.to_vec();
        let state = GLOBAL_STATE_UINTS.lock().unwrap();
        *state.get(&key).unwrap_or(&0)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_impl_set_global_uint(
        _exec_env: *mut wamrtime::wamr::WASMExecEnv,
        _ctx: *mut c_void,
        _app: u64,
        key_ptr: *const u8,
        key_len: u32,
        value: u64,
    ) {
        let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len as usize) }.to_vec();
        let mut state = GLOBAL_STATE_UINTS.lock().unwrap();
        state.insert(key, value);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_impl_get_global_bytes(
        _exec_env: *mut wamrtime::wamr::WASMExecEnv,
        _ctx: *mut c_void,
        _app: u64,
        key_ptr: *const u8,
        key_len: u32,
        dest_ptr: *mut u8,
        dest_len: u32,
    ) -> i32 {
        let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len as usize) }.to_vec();
        let state = GLOBAL_STATE_BYTES.lock().unwrap();
        if let Some(value) = state.get(&key) {
            if dest_len < value.len() as u32 {
                return -1;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), dest_ptr, value.len());
            }
            value.len() as i32
        } else {
            0
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_impl_set_global_bytes(
        _exec_env: *mut wamrtime::wamr::WASMExecEnv,
        _ctx: *mut c_void,
        _app: u64,
        key_ptr: *const u8,
        key_len: u32,
        src_ptr: *const u8,
        src_len: u32,
    ) {
        let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len as usize) }.to_vec();
        let value = unsafe { std::slice::from_raw_parts(src_ptr, src_len as usize) }.to_vec();
        let mut state = GLOBAL_STATE_BYTES.lock().unwrap();
        state.insert(key, value);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_impl_avm_get_global_var_uint(
        _exec_env: *mut wamrtime::wamr::WASMExecEnv,
        _ctx: *mut c_void,
        field_index: u64,
    ) -> u64 {
        match field_index {
            8 => 42, // CurrentApplicationID
            _ => panic!("Unknown global field index {}", field_index),
        }
    }

    fn run_wasm_test(wasm_file: &str) {
        avm_init(
            rust_impl_get_global_uint,
            rust_impl_set_global_uint,
            rust_impl_get_global_bytes,
            rust_impl_set_global_bytes,
            rust_impl_avm_get_global_var_uint,
        );

        let wasm_path = PathBuf::from(wasm_file);

        let wasm_bytes = std::fs::read(wasm_path).expect("Failed to read WASM file");

        let inst_start = Instant::now();
        let instrumented_bytes = Compiler::new()
            .compile_wasm(&mut wasm_bytes.clone())
            .expect("Failed to compile WASM");
        let inst_duration = inst_start.elapsed();
        println!("Instrumentation time: {:?}", inst_duration);

        let mut times = Vec::new();

        for _ in 0..1000 {
            let cloned_bytes = instrumented_bytes.clone();
            let start = Instant::now();
            AVM_RUNTIME_THREAD.call_program(cloned_bytes);

            let duration = start.elapsed();
            times.push(duration);
            unsafe {
                GAS_USED = 0;
            }
        }

        let avg = times.iter().sum::<std::time::Duration>() / (times.len() as u32);
        println!("Average execution time: {:?}", avg);
        let min = times.iter().min().unwrap();
        println!("Minimum execution time: {:?}", min);
        let max = times.iter().max().unwrap();
        println!("Maximum execution time: {:?}", max);

        println!("All iterations completed successfully.");
    }

    #[test]
    fn test_avm_blank_key() {
        run_wasm_test(
            "/Users/joe/git/joe-p/wamrtime/target/wasm32-unknown-unknown/wasm_small/avm_blank_key.wasm",
        );
    }

    #[test]
    fn test_avm_complex() {
        run_wasm_test(
            "/Users/joe/git/joe-p/wamrtime/target/wasm32-unknown-unknown/wasm_small/avm_complex.wasm",
        );
    }

    #[test]
    fn test_avm_fibo() {
        run_wasm_test(
            "/Users/joe/git/joe-p/wamrtime/target/wasm32-unknown-unknown/wasm_small/fibo.wasm",
        );
    }

    #[test]
    fn test_avm_ret_1() {
        run_wasm_test(
            "/Users/joe/git/joe-p/wamrtime/target/wasm32-unknown-unknown/wasm_small/ret_1.wasm",
        );
    }

    #[test]
    fn test_avm_state_loop() {
        run_wasm_test(
            "/Users/joe/git/joe-p/wamrtime/target/wasm32-unknown-unknown/wasm_small/state_loop.wasm",
        );
    }
}
