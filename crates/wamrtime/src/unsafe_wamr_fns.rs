//! # SAFETY
//! This module simply wraps the unsafe WAMR bindings in safe Rust functions. The safety of
//! these functions is entirely dependent on WAMR C implementation and correct usage.
//!
//! If there is additional unsafe functionality that needs to be implemented for WAMR usage, it
//! SHOULD NOT be added here. This module is only for wrapping existing unsafe WAMR functions
//! without any changes to the signature or behavior.
use crate::wamr;

pub fn wasm_runtime_get_module_inst(exec_env: wamr::wasm_exec_env_t) -> wamr::wasm_module_inst_t {
    unsafe { wamr::wasm_runtime_get_module_inst(exec_env) }
}

pub fn wasm_runtime_addr_app_to_native(
    module_inst: wamr::wasm_module_inst_t,
    app_offset: u64,
) -> *mut ::std::os::raw::c_void {
    unsafe { wamr::wasm_runtime_addr_app_to_native(module_inst, app_offset) }
}

pub fn wasm_runtime_validate_app_addr(
    module_inst: wamr::wasm_module_inst_t,
    app_offset: u64,
    size: u64,
) -> bool {
    unsafe { wamr::wasm_runtime_validate_app_addr(module_inst, app_offset, size) }
}

pub fn wasm_runtime_destroy_exec_env(exec_env: *mut wamr::WASMExecEnv) {
    unsafe {
        wamr::wasm_runtime_destroy_exec_env(exec_env);
    }
}

pub fn wasm_runtime_deinstantiate(instance: *mut wamr::WASMModuleInstanceCommon) {
    unsafe {
        wamr::wasm_runtime_deinstantiate(instance);
    }
}

pub fn wasm_runtime_unload(module: *mut wamr::WASMModuleCommon) {
    unsafe {
        wamr::wasm_runtime_unload(module);
    }
}

pub fn wasm_runtime_load(
    buf: *mut u8,
    size: u32,
    error_buf: *mut ::std::os::raw::c_char,
    error_buf_size: u32,
) -> wamr::wasm_module_t {
    unsafe { wamr::wasm_runtime_load(buf, size, error_buf, error_buf_size) }
}

pub fn wasm_runtime_instantiate(
    module: wamr::wasm_module_t,
    default_stack_size: u32,
    host_managed_heap_size: u32,
    error_buf: *mut ::std::os::raw::c_char,
    error_buf_size: u32,
) -> wamr::wasm_module_inst_t {
    unsafe {
        wamr::wasm_runtime_instantiate(
            module,
            default_stack_size,
            host_managed_heap_size,
            error_buf,
            error_buf_size,
        )
    }
}

pub fn wasm_runtime_create_exec_env(
    instance: *mut wamr::WASMModuleInstanceCommon,
    stack_size: u32,
) -> *mut wamr::WASMExecEnv {
    unsafe { wamr::wasm_runtime_create_exec_env(instance, stack_size) }
}

pub fn wasm_runtime_lookup_function(
    instance: *mut wamr::WASMModuleInstanceCommon,
    name: *const ::std::os::raw::c_char,
) -> wamr::wasm_function_inst_t {
    unsafe { wamr::wasm_runtime_lookup_function(instance, name) }
}

pub fn wasm_runtime_call_wasm_a(
    exec_env: *mut wamr::WASMExecEnv,
    func: wamr::wasm_function_inst_t,
    num_results: u32,
    results: *mut wamr::wasm_val_t,
    num_args: u32,
    args: *mut wamr::wasm_val_t,
) -> bool {
    unsafe { wamr::wasm_runtime_call_wasm_a(exec_env, func, num_results, results, num_args, args) }
}

pub fn wasm_runtime_get_exception(
    instance: *mut wamr::WASMModuleInstanceCommon,
) -> *const ::std::os::raw::c_char {
    unsafe { wamr::wasm_runtime_get_exception(instance) }
}

pub fn wasm_runtime_destroy() {
    unsafe {
        wamr::wasm_runtime_destroy();
    }
}

pub fn wasm_runtime_full_init(args: *mut wamr::RuntimeInitArgs) -> bool {
    unsafe { wamr::wasm_runtime_full_init(args) }
}

pub fn wasm_runtime_register_natives(
    module_name: *const ::std::os::raw::c_char,
    natives: *mut wamr::NativeSymbol,
    n_native_symbols: u32,
) -> bool {
    unsafe { wamr::wasm_runtime_register_natives(module_name, natives, n_native_symbols) }
}

pub fn wasm_val_t_get_i64(v: &wamr::wasm_val_t) -> i64 {
    unsafe { v.of.i64_ }
}
