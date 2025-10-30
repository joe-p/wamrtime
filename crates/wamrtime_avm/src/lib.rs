use std::ffi::c_void;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::Instant;

use wamrtime::compiler::Compiler;
use wamrtime::evaluator::Evaluator;
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
                ctx: *mut ::std::ffi::c_void,
                $([<$fn_name _impl>]: [<$fn_name:camel Fn>]),*
            ) {
                unsafe {
                    // TODO: Eventually add this back in
                    // if !AVM_CTX.is_null() {
                    //     panic!("AVM context already set");
                    // }
                    AVM_CTX = ctx;
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
}

enum AvmType {
    U64,
    Bytes,
}

impl From<&AvmType> for wamrtime::runtime::WamrType {
    fn from(avm_type: &AvmType) -> Self {
        match avm_type {
            AvmType::U64 => wamrtime::runtime::WamrType::I64,
            AvmType::Bytes => wamrtime::runtime::WamrType::ByteSlice,
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
        args: &[AvmType::U64, AvmType::Bytes],
        returns: Some(AvmType::U64),
        host_func: avm_get_global_uint as *mut c_void,
    },
    AvmFunction {
        name: "avm_set_global_uint",
        args: &[AvmType::U64, AvmType::Bytes, AvmType::U64],
        returns: None,
        host_func: avm_set_global_uint as *mut c_void,
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
});

static EVALUATOR: OnceLock<Mutex<Evaluator>> = OnceLock::new();

// TODO: Put this in avm_init?
#[unsafe(no_mangle)]
pub extern "C" fn avm_init_eval() {
    if EVALUATOR.get().is_some() {
        return;
    }
    let runtime = RUNTIME.deref();
    let evaluator = Evaluator::new(runtime);
    if EVALUATOR.set(Mutex::new(evaluator)).is_err() {
        panic!("Evaluator already initialized");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn test_avm_prep_round() {
    let runtime = RUNTIME.deref();
    let wasm_path = PathBuf::from("/Users/joe/git/joe-p/wamrtime/zig-out/bin/avm.wasm");
    let mut wasm_bytes = std::fs::read(wasm_path).expect("Failed to read WASM file");
    let compiler = Compiler::new(runtime);
    let aot_bytes = compiler.compile_wasm(&mut wasm_bytes);

    let aot_bytes_vec = vec![aot_bytes.clone()];

    let mut evaluator = EVALUATOR
        .get()
        .expect("Evaluator not initialized")
        .lock()
        .unwrap();

    evaluator
        .next_round(aot_bytes_vec.clone())
        .expect("Initial round failed");

    evaluator
        .next_round(aot_bytes_vec.clone())
        .expect("next round failed");
}

#[unsafe(no_mangle)]
pub extern "C" fn test_avm_run_program() {
    let evaluator = EVALUATOR
        .get()
        .expect("Evaluator not initialized")
        .lock()
        .unwrap();

    evaluator.call_program(0).expect("Program call failed");
}

#[unsafe(no_mangle)]
pub extern "C" fn test_run() {
    let runtime = WamrRuntime::new(
        host_gas_check_impl,
        AVM_FUNCTIONS.iter().map(WamrHostFunction::from).collect(),
    );

    let wasm_path = PathBuf::from("/Users/joe/git/joe-p/wamrtime/zig-out/bin/avm.wasm");
    let mut wasm_bytes = std::fs::read(wasm_path).expect("Failed to read WASM file");
    let compiler = Compiler::new(&runtime);
    let aot_bytes = compiler.compile_wasm(&mut wasm_bytes);

    let mut evaluator = Evaluator::new(&runtime);

    let aot_bytes_vec = vec![aot_bytes.clone(); 10];

    evaluator
        .next_round(aot_bytes_vec.clone())
        .expect("Initial round failed");

    evaluator
        .next_round(aot_bytes_vec.clone())
        .expect("Second round failed");

    for i in 0..aot_bytes_vec.len() {
        println!("\nIteration {}:", i + 1);
        let start = Instant::now();
        evaluator.call_program(i).expect("Program call failed");
        let duration = start.elapsed();
        println!("Iteration {} executed in {} ns", i + 1, duration.as_nanos());
    }

    println!("All iterations completed successfully.");
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{LazyLock, Mutex},
    };

    use super::*;

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_host_function(_ctx: *mut c_void) {
        println!("Hello from Rust!");
    }

    static GLOBAL_STATE: LazyLock<Mutex<HashMap<Vec<u8>, u64>>> =
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
        let state = GLOBAL_STATE.lock().unwrap();
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
        let mut state = GLOBAL_STATE.lock().unwrap();
        state.insert(key, value);
    }

    #[test]
    fn test_avm() {
        avm_init(
            std::ptr::null_mut(),
            rust_impl_get_global_uint,
            rust_impl_set_global_uint,
        );
        test_run();
    }
}
