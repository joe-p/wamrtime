use std::ffi::c_void;
use std::ops::Deref;
use std::sync::LazyLock;

use wamrtime::runtime::{WamrHostFunction, WamrRuntime, WamrType};

static mut AVM_CTX: *mut c_void = core::ptr::null_mut();

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
                unsafe {
                    // TODO: Eventually should actually only get called once when creating the
                    // block evaluator
                    $(
                        [<$fn_name:snake:upper _IMPL>] = Some([<$fn_name _impl>]);
                    )*
                    let _ = RUNTIME.deref();
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

static RUNTIME: LazyLock<WamrRuntime> = LazyLock::new(|| {
    WamrRuntime::new(
        host_gas_check_impl,
        AVM_FUNCTIONS.iter().map(WamrHostFunction::from).collect(),
    )
    .expect("should be able to create AVM WAMR runtime")
});

#[unsafe(no_mangle)]
pub extern "C" fn avm_set_ctx(ctx: *mut c_void) {
    unsafe {
        AVM_CTX = ctx;
    }
}

// Functions exposed for testing purposes

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

// # Safety
// We assume the err_buf is valid for writes of at least err_buf_len bytes.
// TODO: go-algorand tests
// #[unsafe(no_mangle)]
// pub unsafe extern "C" fn test_avm_run_program(err_buf: *mut u8, err_buf_len: u64) -> u64 {
// }

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{LazyLock, Mutex},
    };

    use std::time::Instant;

    use wamrtime::{
        compiler::Compiler,
        program::{self},
    };

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

        let instrumented_bytes = Compiler::new()
            .compile_wasm(&mut wasm_bytes.clone())
            .expect("Failed to compile WASM");

        let err_buf = &mut [0i8; 512];

        let mut times = Vec::new();

        for _ in 0..1000 {
            let mut cloned_bytes = instrumented_bytes.clone();
            let start = Instant::now();
            let program = program::Program::new(cloned_bytes.as_mut_slice(), err_buf, 128 * 1024)
                .expect("Failed to create program from WASM");
            let _ = program.call().expect("Program call failed");
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
}
