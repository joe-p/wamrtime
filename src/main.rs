use std::ffi::c_void;

#[allow(warnings)]
mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
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
        unsafe {
            wamr::wasm_runtime_deinstantiate(self.instance);
            wamr::wasm_runtime_unload(self.module);
            wamr::wasm_runtime_destroy_exec_env(self.exec_env);
        }
    }
}

impl Program {
    pub fn new(aot_bytes: &mut [u8], err_buf: &mut [i8]) -> Self {
        println!("Loading WASM module...");
        unsafe {
            let module = wamr::wasm_runtime_load(
                aot_bytes.as_mut_ptr(),
                aot_bytes.len().try_into().expect("should fit"),
                err_buf.as_mut_ptr(),
                ERROR_BUFFER_SIZE.try_into().expect("should fit"),
            );
            if module.is_null() {
                panic!("Failed to load WASM module");
            }

            let instance = wamr::wasm_runtime_instantiate(
                module,
                STACK_SIZE as u32,
                HEAP_SIZE as u32,
                err_buf.as_mut_ptr(),
                ERROR_BUFFER_SIZE as u32,
            );
            if instance.is_null() {
                wamr::wasm_runtime_unload(module);
                panic!("Failed to instantiate WASM module");
            }

            let exec_env = wamr::wasm_runtime_create_exec_env(instance, 8192);
            if exec_env.is_null() {
                wamr::wasm_runtime_deinstantiate(instance);
                wamr::wasm_runtime_unload(module);
                panic!("Failed to create execution environment");
            }

            let program_func = wamr::wasm_runtime_lookup_function(instance, c"program".as_ptr());
            if program_func.is_null() {
                wamr::wasm_runtime_destroy_exec_env(exec_env);
                wamr::wasm_runtime_deinstantiate(instance);
                wamr::wasm_runtime_unload(module);
                panic!("Failed to find 'program' function");
            }

            Program {
                module,
                instance,
                exec_env,
                program_func,
            }
        }
    }
}

pub struct Runtime {}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            wamr::wasm_runtime_destroy();
        }
    }
}

impl Runtime {
    pub fn new(heap_buf: &mut [u8]) -> Self {
        unsafe {
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

            if !wamr::wasm_runtime_full_init(&mut init_args as *mut wamr::RuntimeInitArgs) {
                panic!("Failed to initialize WAMR runtime");
            }

            println!("Registering native functions...");

            let native_symbols = [wamr::NativeSymbol {
                symbol: c"ret_1337".as_ptr(),
                func_ptr: ret_1337 as *mut c_void,
                signature: c"()I".as_ptr(),
                ..Default::default()
            }];

            if !wamr::wasm_runtime_register_natives(
                c"env".as_ptr(),
                native_symbols.as_ptr() as *mut wamr::NativeSymbol,
                native_symbols.len() as u32,
            ) {
                panic!("Failed to register native symbols");
            }

            println!("Native functions registered successfully.");
            Runtime {}
        }
    }
}

fn main() {
    let mut aot_bytes = std::fs::read("zig-out/bin/program.aot").expect("Failed to read AOT file");
    let heap_buf: Vec<u8> = vec![0; HEAP_SIZE];

    unsafe {
        let t = wamr::get_package_type(aot_bytes.as_ptr(), aot_bytes.len().try_into().unwrap());
        println!("Package type: {}", t);

        let mut _runtime = Runtime::new(&mut heap_buf.clone());
        println!("WAMR Runtime initialized.");

        let mut err_buffer: [i8; ERROR_BUFFER_SIZE] = [0; ERROR_BUFFER_SIZE];
        let mut _program = Program::new(&mut aot_bytes, &mut err_buffer);
        println!("WASM Program instantiated.");
    }
}
