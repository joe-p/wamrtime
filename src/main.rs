use std::ffi::c_void;

#[allow(warnings)]
mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

/// SAFETY: This module simply wraps the unsafe WAMR bindings in safe Rust functions. The safety of
/// these functions is entirely dependent on WAMR C implementation and correct usage.
mod unsafe_wamr_fns {
    use crate::wamr;

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
        unsafe {
            wamr::wasm_runtime_call_wasm_a(exec_env, func, num_results, results, num_args, args)
        }
    }

    pub fn wasm_runtime_get_exception_string(
        instance: *mut wamr::WASMModuleInstanceCommon,
    ) -> String {
        unsafe {
            let ptr = wamr::wasm_runtime_get_exception(instance);
            if ptr.is_null() {
                return "unknown WAMR exception".to_string();
            }
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
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

    pub fn get_package_type(buf: *const u8, size: u32) -> wamr::package_type_t {
        unsafe { wamr::get_package_type(buf, size) }
    }

    pub fn wasm_val_t_get_i64(v: &wamr::wasm_val_t) -> i64 {
        unsafe { v.of.i64_ }
    }
}

extern "C" fn ret_1337() -> u64 {
    1337
}

const ERROR_BUFFER_SIZE: usize = 128;

const HEAP_SIZE: usize = 1024 * 1024 * 2;
const STACK_SIZE: usize = 1024 * 128;

pub struct Program {
    module: *mut wamr::WASMModuleCommon,
    instance: *mut wamr::WASMModuleInstanceCommon,
    exec_env: *mut wamr::WASMExecEnv,
    program_func: wamr::wasm_function_inst_t,
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe_wamr_fns::wasm_runtime_destroy_exec_env(self.exec_env);
        unsafe_wamr_fns::wasm_runtime_deinstantiate(self.instance);
        unsafe_wamr_fns::wasm_runtime_unload(self.module);
    }
}

impl Program {
    pub fn new(aot_bytes: &mut [u8], err_buf: &mut [i8]) -> Self {
        println!("Loading WASM module...");

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
            let msg = unsafe_wamr_fns::wasm_runtime_get_exception_string(self.instance);
            panic!("WASM function call failed: {}", msg);
        }

        unsafe_wamr_fns::wasm_val_t_get_i64(&call_results[0]) as u64
    }
}

pub struct Runtime {
    native_symbols: Vec<wamr::NativeSymbol>,
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe_wamr_fns::wasm_runtime_destroy();
    }
}

impl Runtime {
    pub fn new(heap_buf: &mut [u8]) -> Self {
        let mut init_args = wamr::RuntimeInitArgs {
            mem_alloc_type: wamr::mem_alloc_type_t_Alloc_With_Pool,
            running_mode: wamr::RunningMode_Mode_Interp,
            mem_alloc_option: wamr::MemAllocOption {
                pool: wamr::MemAllocOption__bindgen_ty_1 {
                    heap_buf: heap_buf.as_ptr() as *mut c_void,
                    heap_size: HEAP_SIZE as u32,
                },
            },
            ..Default::default()
        };

        if !unsafe_wamr_fns::wasm_runtime_full_init(&mut init_args as *mut wamr::RuntimeInitArgs) {
            panic!("Failed to initialize WAMR runtime");
        }

        println!("Registering native functions...");

        let runtime = Runtime {
            native_symbols: vec![wamr::NativeSymbol {
                symbol: c"ret_1337".as_ptr(),
                func_ptr: ret_1337 as *mut c_void,
                signature: c"()I".as_ptr(),
                ..Default::default()
            }],
        };

        if !unsafe_wamr_fns::wasm_runtime_register_natives(
            c"env".as_ptr(),
            runtime.native_symbols.as_ptr() as *mut wamr::NativeSymbol,
            runtime.native_symbols.len() as u32,
        ) {
            panic!("Failed to register native symbols");
        }

        println!("Native functions registered successfully.");
        runtime
    }
}

fn main() {
    let mut aot_bytes = std::fs::read("zig-out/bin/program.aot").expect("Failed to read AOT file");
    let heap_buf: Vec<u8> = vec![0; HEAP_SIZE];

    let t =
        unsafe_wamr_fns::get_package_type(aot_bytes.as_ptr(), aot_bytes.len().try_into().unwrap());
    println!("Package type: {}", t);

    let mut _runtime = Runtime::new(&mut heap_buf.clone());
    println!("WAMR Runtime initialized.");

    let mut err_buffer: [i8; ERROR_BUFFER_SIZE] = [0; ERROR_BUFFER_SIZE];
    let program = Program::new(&mut aot_bytes, &mut err_buffer);
    println!("WASM Program instantiated.");

    let result = program.call();
    println!("WASM Program returned: {}", result);
}
