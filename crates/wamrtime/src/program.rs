use crate::unsafe_wamr_fns;
use crate::wamr;
use crate::{ERROR_BUFFER_SIZE, Result, STACK_SIZE};
use color_eyre::eyre::{ensure, eyre};
use std::convert::TryFrom;
pub struct Program {
    module: *mut wamr::WASMModuleCommon,
    instance: *mut wamr::WASMModuleInstanceCommon,
    exec_env: *mut wamr::WASMExecEnv,
    program_func: wamr::wasm_function_inst_t,
}

unsafe impl Send for Program {}
unsafe impl Sync for Program {}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe_wamr_fns::wasm_runtime_destroy_exec_env(self.exec_env);
        unsafe_wamr_fns::wasm_runtime_deinstantiate(self.instance);
        unsafe_wamr_fns::wasm_runtime_unload(self.module);
    }
}

impl Program {
    pub fn new(aot_bytes: &mut [u8], err_buf: &mut [i8], app_heap_size: usize) -> Result<Self> {
        ensure!(
            err_buf.len() >= ERROR_BUFFER_SIZE,
            "Error buffer must be at least {ERROR_BUFFER_SIZE} bytes"
        );
        err_buf.fill(0);

        let aot_len =
            u32::try_from(aot_bytes.len()).map_err(|_| eyre!("AOT length exceeds u32::MAX"))?;
        let err_buf_len = u32::try_from(ERROR_BUFFER_SIZE)
            .map_err(|_| eyre!("ERROR_BUFFER_SIZE exceeds u32::MAX"))?;
        let app_heap_size =
            u32::try_from(app_heap_size).map_err(|_| eyre!("App heap size exceeds u32::MAX"))?;

        let module = unsafe_wamr_fns::wasm_runtime_load(
            aot_bytes.as_mut_ptr(),
            aot_len,
            err_buf.as_mut_ptr(),
            err_buf_len,
        );

        if module.is_null() {
            let err_msg = unsafe { std::ffi::CStr::from_ptr(err_buf.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            return Err(eyre!("Failed to load WASM module: {}", err_msg));
        }

        let instance = unsafe_wamr_fns::wasm_runtime_instantiate(
            module,
            STACK_SIZE,
            app_heap_size,
            err_buf.as_mut_ptr(),
            err_buf_len,
        );

        if instance.is_null() {
            unsafe_wamr_fns::wasm_runtime_unload(module);
            let err_msg = unsafe { std::ffi::CStr::from_ptr(err_buf.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            return Err(eyre!("Failed to instantiate WASM module: {}", err_msg));
        }

        let exec_env = unsafe_wamr_fns::wasm_runtime_create_exec_env(instance, STACK_SIZE);
        if exec_env.is_null() {
            unsafe_wamr_fns::wasm_runtime_deinstantiate(instance);
            unsafe_wamr_fns::wasm_runtime_unload(module);
            return Err(eyre!("Failed to create execution environment"));
        }

        let program_func =
            unsafe_wamr_fns::wasm_runtime_lookup_function(instance, c"program".as_ptr());

        if program_func.is_null() {
            unsafe_wamr_fns::wasm_runtime_destroy_exec_env(exec_env);
            unsafe_wamr_fns::wasm_runtime_deinstantiate(instance);
            unsafe_wamr_fns::wasm_runtime_unload(module);
            return Err(eyre!("Failed to find 'program' function"));
        }

        Ok(Program {
            module,
            instance,
            exec_env,
            program_func,
        })
    }

    pub fn call(&self) -> Result<u64> {
        let kind = u8::try_from(wamr::wasm_valkind_enum_WASM_I64)
            .map_err(|_| eyre!("WASM value kind does not fit in u8"))?;
        let mut call_results = [wamr::wasm_val_t {
            kind,
            of: wamr::wasm_val_t__bindgen_ty_1 { i64_: 0 },
            ..Default::default()
        }];

        if !unsafe_wamr_fns::wasm_runtime_call_wasm_a(
            self.exec_env,
            self.program_func,
            1,
            call_results.as_mut_ptr(),
            0,
            std::ptr::null_mut(),
        ) {
            let ptr = unsafe_wamr_fns::wasm_runtime_get_exception(self.instance);
            let msg = if ptr.is_null() {
                "unknown WAMR exception".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned()
            };

            return Err(eyre!("WASM function call failed: {}", msg));
        }

        Ok(unsafe_wamr_fns::wasm_val_t_get_i64(&call_results[0]) as u64)
    }
}
