use std::convert::TryFrom;
use std::ffi::c_void;

use crate::runtime::WamrRuntime;
use crate::unsafe_wamr_fns;
use crate::{ERROR_BUFFER_SIZE, Result, wamr};
use color_eyre::eyre::{ensure, eyre};

use radix_wasm_instrument::{
    gas_metering::{ConstantCostRules, host_function, inject},
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

    pub fn compile_wasm(&self, raw_wasm_bytes: &mut [u8], err_buf: &mut [i8]) -> Result<Vec<u8>> {
        ensure!(
            err_buf.len() >= ERROR_BUFFER_SIZE,
            "Error buffer must be at least {ERROR_BUFFER_SIZE} bytes"
        );

        err_buf.fill(0);

        let backend = host_function::Injector::new("env", "host_gas_check");

        let mut module = ModuleInfo::new(raw_wasm_bytes)
            .map_err(|err| eyre!("Failed to create ModuleInfo from bytes: {err}"))?;

        let mut wasm_bytes = inject(&mut module, backend, &ConstantCostRules::new(1, 10_000, 1))
            .map_err(|err| eyre!("Failed to inject gas metering: {err}"))?;

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

        let wasm_len =
            u32::try_from(wasm_bytes.len()).map_err(|_| eyre!("WASM length exceeds u32::MAX"))?;
        let err_buf_len = u32::try_from(ERROR_BUFFER_SIZE)
            .map_err(|_| eyre!("ERROR_BUFFER_SIZE exceeds u32::MAX"))?;

        let module = unsafe_wamr_fns::wasm_runtime_load(
            wasm_bytes.as_mut_ptr(),
            wasm_len,
            err_buf.as_mut_ptr(),
            err_buf_len,
        );
        if module.is_null() {
            let err_msg = unsafe { std::ffi::CStr::from_ptr(err_buf.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            return Err(eyre!(
                "Failed to load WASM module for compilation: {}",
                err_msg
            ));
        }

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
            return Err(eyre!("Failed to create compilation data: {}", err_msg));
        }

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
            return Err(eyre!("Failed to create compilation context: {}", err_msg));
        }

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
            return Err(eyre!("Failed to compile WASM: {}", err_msg));
        }

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
            return Err(eyre!("Failed to create AOT object data: {}", err_msg));
        }

        let compiled_size = unsafe { wamr::aot_get_aot_file_size(comp_ctx, comp_data, obj_data) };

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
            return Err(eyre!("Failed to emit AOT file buffer: {}", err_msg));
        }

        unsafe {
            wamr::aot_obj_data_destroy(obj_data);
            wamr::aot_destroy_comp_context(comp_ctx);
            wamr::aot_destroy_comp_data(comp_data);
        }
        unsafe_wamr_fns::wasm_runtime_unload(module);

        Ok(aot_bytes)
    }
}
