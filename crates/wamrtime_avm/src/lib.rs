use std::ffi::c_void;
use std::time::Instant;

use wamrtime::compiler::Compiler;
use wamrtime::evaluator::Evaluator;
use wamrtime::runtime::{WamrHostFunction, WamrRuntime, WamrType};

pub type AvmDispatcher = unsafe extern "C" fn(
    ctx: *mut c_void,
    function: u64,
    args: *const u64,
    arg_count: u32,
    ret_ptr: *mut u64,
) -> u64;

static mut AVM_DISPATCHER: Option<AvmDispatcher> = None;
static mut AVM_CTX: *mut c_void = core::ptr::null_mut();

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

/// (app: u64, key_ptr: u64, key_len: u64)
type GetGlobalUintArgs = [u64; 3];

/// [u64]
type GetGlobalUintRet = [u64; 1];

/// (app: u64, key_ptr: u64, key_len: u64, value: u64)
type SetGlobalUintArgs = [u64; 4];

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_avm_dispatcher(dispatcher: AvmDispatcher, ctx: *mut c_void) {
    unsafe {
        AVM_DISPATCHER = Some(dispatcher);
        AVM_CTX = ctx;
    }
}

#[allow(clippy::missing_safety_doc)]
extern "C" fn avm_get_global_uint(
    exec_env: *mut wamrtime::wamr::WASMExecEnv,
    app: u64,
    key_ptr: u32,
    key_len: u32,
) -> u64 {
    let dispatcher = unsafe { AVM_DISPATCHER.expect("AVM dispatcher not set") };
    let ctx = unsafe { AVM_CTX };

    let key = wamrtime::runtime::get_wamr_slice(exec_env, key_ptr as u64, key_len as u64);
    let args: GetGlobalUintArgs = [app, key.as_ptr() as u64, key.len() as u64];
    let mut ret: GetGlobalUintRet = [0];
    let num_returns = unsafe {
        dispatcher(
            ctx,
            AvmFunctions::GetGlobalUint as u64,
            args.as_ptr(),
            args.len() as u32,
            ret.as_mut_ptr(),
        )
    };

    assert_eq!(num_returns, 1, "Expected 1 return value from AVM");
    ret[0]
}

#[allow(clippy::missing_safety_doc)]
extern "C" fn avm_set_global_uint(
    exec_env: *mut wamrtime::wamr::WASMExecEnv,
    app: u64,
    key_ptr: u32,
    key_len: u32,
    value: u64,
) {
    let dispatcher = unsafe { AVM_DISPATCHER.expect("AVM dispatcher not set") };
    let ctx = unsafe { AVM_CTX };

    let key = wamrtime::runtime::get_wamr_slice(exec_env, key_ptr as u64, key_len as u64);
    let args: SetGlobalUintArgs = [app, key.as_ptr() as u64, key.len() as u64, value];
    unsafe {
        dispatcher(
            ctx,
            AvmFunctions::SetGlobalUint as u64,
            args.as_ptr(),
            args.len() as u32,
            std::ptr::null_mut(),
        )
    };
}

#[repr(u64)]
enum AvmFunctions {
    GetGlobalUint,
    SetGlobalUint,
}

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
pub extern "C" fn test_run() {
    let runtime = WamrRuntime::new(
        host_gas_check_impl,
        AVM_FUNCTIONS.iter().map(WamrHostFunction::from).collect(),
    );

    let mut wasm_bytes =
        std::fs::read("../../zig-out/bin/program.wasm").expect("Failed to read WASM file");
    let compiler = Compiler::new(&runtime);
    let aot_bytes = compiler.compile_wasm(&mut wasm_bytes);

    let mut evaluator = Evaluator::new(&runtime);

    let aot_bytes_vec = vec![
        aot_bytes.clone(),
        aot_bytes.clone(),
        aot_bytes.clone(),
        aot_bytes.clone(),
    ];

    for i in 0..3 {
        println!("\nIteration {}:", i + 1);
        evaluator
            .next_round(aot_bytes_vec.clone())
            .expect("Round failed");
    }

    let start = Instant::now();
    evaluator
        .next_round(aot_bytes_vec)
        .expect("Final round failed");
    let duration = start.elapsed();
    println!("Final iteration executed in {} ns", duration.as_nanos());

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

    unsafe extern "C" fn rust_dispatcher_impl(
        _ctx: *mut c_void,
        function: u64,
        args: *const u64,
        arg_count: u32,
        ret_ptr: *mut u64,
    ) -> u64 {
        match function {
            x if x == AvmFunctions::GetGlobalUint as u64 => {
                assert_eq!(arg_count, 3);
                let args = unsafe { std::slice::from_raw_parts(args, arg_count as usize) };
                let _app = args[0];
                let key_ptr = args[1] as *const u8;
                let key_len = args[2] as usize;
                let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len) };

                let value = GLOBAL_STATE.lock().unwrap().get(key).cloned().unwrap_or(0);

                unsafe {
                    let ret_slice = std::slice::from_raw_parts_mut(ret_ptr, 1);
                    ret_slice[0] = value;
                }

                1 // number of return values
            }
            x if x == AvmFunctions::SetGlobalUint as u64 => {
                assert_eq!(arg_count, 4);
                let args = unsafe { std::slice::from_raw_parts(args, arg_count as usize) };
                let _app = args[0];
                let key_ptr = args[1] as *const u8;
                let key_len = args[2] as usize;
                let value = args[3];
                let key = unsafe { std::slice::from_raw_parts(key_ptr, key_len) };

                GLOBAL_STATE.lock().unwrap().insert(key.to_vec(), value);

                0 // number of return values
            }
            _ => panic!("Unknown function ID: {}", function),
        }
    }

    #[test]
    fn test_avm() {
        unsafe {
            set_avm_dispatcher(rust_dispatcher_impl, std::ptr::null_mut());
        }
        let runtime = WamrRuntime::new(
            host_gas_check_impl,
            AVM_FUNCTIONS.iter().map(WamrHostFunction::from).collect(),
        );

        let mut wasm_bytes =
            std::fs::read("../../zig-out/bin/avm.wasm").expect("Failed to read WASM file");
        let compiler = Compiler::new(&runtime);
        let aot_bytes = compiler.compile_wasm(&mut wasm_bytes);

        let mut evaluator = Evaluator::new(&runtime);

        let aot_bytes_vec = vec![aot_bytes.clone()];

        evaluator
            .next_round(aot_bytes_vec.clone())
            .expect("Initial round failed");

        for i in 0..11 {
            println!("\nIteration {}:", i + 1);
            evaluator
                .next_round(aot_bytes_vec.clone())
                .expect("Round failed");
            println!(
                "Iteration {} completed. Global: {:?}",
                i + 1,
                GLOBAL_STATE.lock().unwrap()
            );
            GLOBAL_STATE.lock().unwrap().clear();
        }

        println!("\nFinal Iteration:");
        let start = Instant::now();
        evaluator
            .next_round(aot_bytes_vec)
            .expect("Final round failed");
        let duration = start.elapsed();
        println!("Final iteration executed in {} ns", duration.as_nanos());

        println!("All iterations completed successfully.");
    }
}
