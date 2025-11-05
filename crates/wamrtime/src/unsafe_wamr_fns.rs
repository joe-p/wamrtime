//! # SAFETY
//! This module simply wraps the unsafe WAMR bindings in safe Rust functions. The safety of
//! these functions is entirely dependent on WAMR C implementation and correct usage.
//!
//! If there is additional unsafe functionality that needs to be implemented for WAMR usage, it
//! SHOULD NOT be added here. This module is only for wrapping existing unsafe WAMR functions
//! without any changes to the signature or behavior.
use crate::wamr;

pub fn wasm_memory_type_get_max_page_count(memory_type: wamr::wasm_memory_type_t) -> u32 {
    unsafe { wamr::wasm_memory_type_get_max_page_count(memory_type) }
}

pub fn wasm_runtime_get_export_type(
    module: wamr::wasm_module_t,
    export_index: i32,
    export_type: *mut wamr::wasm_export_t,
) {
    unsafe {
        wamr::wasm_runtime_get_export_type(module, export_index, export_type);
    }
}

pub fn wasm_runtime_get_export_count(module: wamr::wasm_module_t) -> i32 {
    unsafe { wamr::wasm_runtime_get_export_count(module) }
}

pub fn wasm_runtime_module_malloc(
    module_inst: wamr::wasm_module_inst_t,
    size: u64,
    p_native_addr: *mut *mut ::std::os::raw::c_void,
) -> u64 {
    unsafe { wamr::wasm_runtime_module_malloc(module_inst, size, p_native_addr) }
}

pub fn wasm_runtime_module_free(module_inst: wamr::wasm_module_inst_t, ptr: u64) {
    unsafe {
        wamr::wasm_runtime_module_free(module_inst, ptr);
    }
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

pub fn wasm_runtime_instantiate_ex(
    module: wamr::wasm_module_t,
    args: *const wamr::InstantiationArgs,
    error_buf: *mut ::std::os::raw::c_char,
    error_buf_size: u32,
) -> wamr::wasm_module_inst_t {
    unsafe { wamr::wasm_runtime_instantiate_ex(module, args, error_buf, error_buf_size) }
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
