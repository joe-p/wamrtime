use crate::unsafe_wamr_fns;
use crate::wamr;
use crate::{ERROR_BUFFER_SIZE, HEAP_SIZE, STACK_SIZE};
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
    pub fn new(aot_bytes: &mut [u8], err_buf: &mut [i8]) -> Self {
        let module = unsafe_wamr_fns::wasm_runtime_load(
            aot_bytes.as_mut_ptr(),
            aot_bytes.len().try_into().expect("should fit"),
            err_buf.as_mut_ptr(),
            ERROR_BUFFER_SIZE.try_into().expect("should fit"),
        );

        if module.is_null() {
            panic!("Failed to load WASM module");
        }

        let instance = unsafe_wamr_fns::wasm_runtime_instantiate(
            module,
            STACK_SIZE as u32,
            HEAP_SIZE as u32,
            err_buf.as_mut_ptr(),
            ERROR_BUFFER_SIZE as u32,
        );

        if instance.is_null() {
            unsafe_wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to instantiate WASM module");
        }

        let exec_env = unsafe_wamr_fns::wasm_runtime_create_exec_env(instance, 8192);
        if exec_env.is_null() {
            unsafe_wamr_fns::wasm_runtime_deinstantiate(instance);
            unsafe_wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to create execution environment");
        }

        let program_func =
            unsafe_wamr_fns::wasm_runtime_lookup_function(instance, c"program".as_ptr());

        if program_func.is_null() {
            unsafe_wamr_fns::wasm_runtime_destroy_exec_env(exec_env);
            unsafe_wamr_fns::wasm_runtime_deinstantiate(instance);
            unsafe_wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to find 'program' function");
        }

        Program {
            module,
            instance,
            exec_env,
            program_func,
        }
    }

    pub fn call(&self) -> u64 {
        let mut call_results = [wamr::wasm_val_t {
            kind: wamr::wasm_valkind_enum_WASM_I64.try_into().unwrap(),
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

            panic!("WASM function call failed: {}", msg);
        }

        unsafe_wamr_fns::wasm_val_t_get_i64(&call_results[0]) as u64
    }
}
