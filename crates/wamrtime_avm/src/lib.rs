use std::ffi::c_void;
use std::path::PathBuf;
use std::time::Instant;

use wamrtime::compiler::Compiler;
use wamrtime::evaluator::Evaluator;
use wamrtime::runtime::{WamrHostFunction, WamrRuntime, WamrType};

static mut AVM_CTX: *mut c_void = core::ptr::null_mut();

pub type AvmGetGlobalUintFn = unsafe extern "C" fn(
    exec_env: *mut wamrtime::wamr::WASMExecEnv,
    app: u64,
    key_ptr: *const u8,
    key_len: u32,
) -> u64;

static mut AVM_GET_GLOBAL_UINT_IMPL: Option<AvmGetGlobalUintFn> = None;

extern "C" fn avm_get_global_uint(
    _exec_env: *mut wamrtime::wamr::WASMExecEnv,
    app: u64,
    key_ptr: *const u8,
    key_len: u32,
) -> u64 {
    let avm_get_global_uint_impl =
        unsafe { AVM_GET_GLOBAL_UINT_IMPL.expect("AVM get_global_uint not set") };
    unsafe { avm_get_global_uint_impl(_exec_env, app, key_ptr, key_len) }
}

pub type AvmSetGlobalUintFn = unsafe extern "C" fn(
    exec_env: *mut wamrtime::wamr::WASMExecEnv,
    app: u64,
    key_ptr: *const u8,
    key_len: u32,
    value: u64,
);

static mut AVM_SET_GLOBAL_UINT_IMPL: Option<AvmSetGlobalUintFn> = None;

extern "C" fn avm_set_global_uint(
    _exec_env: *mut wamrtime::wamr::WASMExecEnv,
    app: u64,
    key_ptr: *const u8,
    key_len: u32,
    value: u64,
) {
    let avm_set_global_uint_impl =
        unsafe { AVM_SET_GLOBAL_UINT_IMPL.expect("AVM set_global_uint not set") };
    unsafe { avm_set_global_uint_impl(_exec_env, app, key_ptr, key_len, value) }
}

#[unsafe(no_mangle)]
pub extern "C" fn avm_init(
    ctx: *mut c_void,
    get_global_uint_impl: AvmGetGlobalUintFn,
    set_global_uint_impl: AvmSetGlobalUintFn,
) {
    if !ctx.is_null() {
        panic!("AVM context already set");
    }
    unsafe {
        AVM_CTX = ctx;
        AVM_GET_GLOBAL_UINT_IMPL = Some(get_global_uint_impl);
        AVM_SET_GLOBAL_UINT_IMPL = Some(set_global_uint_impl);
    }
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

#[unsafe(no_mangle)]
pub extern "C" fn test_run() {
    let runtime = WamrRuntime::new(
        host_gas_check_impl,
        AVM_FUNCTIONS.iter().map(WamrHostFunction::from).collect(),
    );

    let mut wasm_path = PathBuf::from("../../zig-out/bin/avm.wasm");

    if !wasm_path.exists() {
        wasm_path = PathBuf::from("./zig-out/bin/avm.wasm");
    }
    let mut wasm_bytes = std::fs::read(wasm_path).expect("Failed to read WASM file");
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
