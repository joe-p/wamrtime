use std::ffi::c_void;

use crate::runtime::WamrRuntime;
use crate::unsafe_wamr_fns;
use crate::{ERROR_BUFFER_SIZE, wamr};

use radix_wasm_instrument::{
    gas_metering::{ConstantCostRules, host_function, inject},
    inject_stack_limiter,
    utils::module_info::ModuleInfo,
};

pub struct Compiler<'runtime> {
    _runtime: &'runtime WamrRuntime,
}

impl Drop for Compiler<'_> {
    fn drop(&mut self) {
        unsafe {
            wamr::aot_compiler_destroy();
        }
    }
}

impl<'runtime> Compiler<'runtime> {
    pub fn new(runtime: &'runtime WamrRuntime) -> Self {
        unsafe {
            wamr::aot_compiler_init();
        }
        Compiler { _runtime: runtime }
    }

    pub fn compile_wasm(&self, raw_wasm_bytes: &mut [u8]) -> Vec<u8> {
        let backend = host_function::Injector::new("env", "host_gas_check");

        let mut module =
            ModuleInfo::new(raw_wasm_bytes).expect("Failed to create ModuleInfo from bytes");

        let gas_metered_module_bytes =
            inject(&mut module, backend, &ConstantCostRules::new(1, 10_000, 1)).unwrap();

        println!(
            "Gas Metering: {} bytes -> {} bytes",
            raw_wasm_bytes.len(),
            gas_metered_module_bytes.len()
        );

        let mut gas_metered_module = ModuleInfo::new(&gas_metered_module_bytes)
            .expect("Failed to create ModuleInfo from gas-metered bytes");

        let stack_limited_and_gas_metered_module_bytes =
            inject_stack_limiter(&mut gas_metered_module, 1000)
                .expect("Failed to inject stack limiter");

        println!(
            "Stack Limited: {} bytes -> {} bytes",
            gas_metered_module_bytes.len(),
            stack_limited_and_gas_metered_module_bytes.len()
        );

        let mut wasm_bytes = stack_limited_and_gas_metered_module_bytes;

        let arch = c"aarch64";

        // These are the default options found in wamr-compiler/main.c
        let mut compile_option = wamr::AOTCompOption {
            target_arch: arch.as_ptr() as *mut i8,
            opt_level: 3,
            size_level: 3,
            output_format: wamr::AOT_FORMAT_FILE,
            bounds_checks: 2,
            stack_bounds_checks: 2,
            enable_simd: false,
            enable_aux_stack_check: true,
            enable_bulk_memory: true,
            enable_ref_types: true,
            enable_gc: false,
            enable_extended_const: false,
            ..Default::default()
        };

        println!("Loading WASM module for compilation...");
        let mut err_buf = [0i8; ERROR_BUFFER_SIZE];
        let module = unsafe_wamr_fns::wasm_runtime_load(
            wasm_bytes.as_mut_ptr(),
            wasm_bytes.len().try_into().expect("should fit"),
            err_buf.as_mut_ptr(),
            ERROR_BUFFER_SIZE.try_into().expect("should fit"),
        );
        if module.is_null() {
            let err_msg = unsafe { std::ffi::CStr::from_ptr(err_buf.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            panic!("Failed to load WASM module for compilation: {}", err_msg);
        }

        println!("WASM module loaded at: {:?}", module);
        println!("Creating compilation data...");

        let comp_data =
            unsafe { wamr::aot_create_comp_data(module as *mut c_void, arch.as_ptr(), false) };

        if comp_data.is_null() {
            let err_ptr = unsafe { wamr::aot_get_last_error() };
            let err_msg = if err_ptr.is_null() {
                "unknown AOT compilation error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(err_ptr) }
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe_wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to create compilation data: {}", err_msg);
        }

        println!("Comp data created at: {:?}", comp_data);

        let comp_ctx = unsafe { wamr::aot_create_comp_context(comp_data, &mut compile_option) };

        if comp_ctx.is_null() {
            let err_ptr = unsafe { wamr::aot_get_last_error() };
            let err_msg = if err_ptr.is_null() {
                "unknown AOT compilation context error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(err_ptr) }
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe {
                wamr::aot_destroy_comp_data(comp_data);
            }
            unsafe_wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to create compilation context: {}", err_msg);
        }

        println!("Compiling WASM to AOT...");
        let compile_result = unsafe { wamr::aot_compile_wasm(comp_ctx) };

        // TODO: PR to wamr-compiler to add a "silent" option to avoid stdout pollution
        unsafe extern "C" {
            fn fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
        }
        unsafe {
            fflush(std::ptr::null_mut());
        }

        if !compile_result {
            let err_ptr = unsafe { wamr::aot_get_last_error() };
            let err_msg = if err_ptr.is_null() {
                "unknown AOT compilation error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(err_ptr) }
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe {
                wamr::aot_destroy_comp_context(comp_ctx);
                wamr::aot_destroy_comp_data(comp_data);
            }
            unsafe_wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to compile WASM: {}", err_msg);
        }

        println!("Creating AOT object data...");
        let obj_data = unsafe { wamr::aot_obj_data_create(comp_ctx) };

        if obj_data.is_null() {
            let err_ptr = unsafe { wamr::aot_get_last_error() };
            let err_msg = if err_ptr.is_null() {
                "unknown AOT obj data creation error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(err_ptr) }
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe {
                wamr::aot_destroy_comp_context(comp_ctx);
                wamr::aot_destroy_comp_data(comp_data);
            }
            unsafe_wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to create AOT object data: {}", err_msg);
        }

        let compiled_size = unsafe { wamr::aot_get_aot_file_size(comp_ctx, comp_data, obj_data) };
        println!("Compiled AOT size: {} bytes", compiled_size);

        let mut aot_bytes = vec![0u8; compiled_size as usize];

        let emit_result = unsafe {
            wamr::aot_emit_aot_file_buf_ex(
                comp_ctx,
                comp_data,
                obj_data,
                aot_bytes.as_mut_ptr(),
                compiled_size,
            )
        };

        if !emit_result {
            let err_ptr = unsafe { wamr::aot_get_last_error() };
            let err_msg = if err_ptr.is_null() {
                "unknown AOT emission error".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(err_ptr) }
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe {
                wamr::aot_obj_data_destroy(obj_data);
                wamr::aot_destroy_comp_context(comp_ctx);
                wamr::aot_destroy_comp_data(comp_data);
            }
            unsafe_wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to emit AOT file buffer: {}", err_msg);
        }

        unsafe {
            wamr::aot_obj_data_destroy(obj_data);
            wamr::aot_destroy_comp_context(comp_ctx);
            wamr::aot_destroy_comp_data(comp_data);
        }
        unsafe_wamr_fns::wasm_runtime_unload(module);

        aot_bytes
    }
}
