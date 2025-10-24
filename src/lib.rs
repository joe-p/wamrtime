use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[allow(warnings)]
mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

mod unsafe_wamr_fns;

// NOTE: This module is aliased here so it's easier to audit other uses of unsafe code.
#[allow(clippy::unsafe_removed_from_name)]
use unsafe_wamr_fns as wamr_fns;

pub type HostFunction = unsafe extern "C" fn(ctx: *mut c_void);

static mut HOST_FUNCTION: Option<HostFunction> = None;
static mut HOST_CTX: *mut c_void = core::ptr::null_mut();

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_host_function(host_fn: Option<HostFunction>, ctx: *mut c_void) {
    unsafe {
        HOST_FUNCTION = host_fn;
        HOST_CTX = ctx;
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_host_function() {
    unsafe {
        HOST_FUNCTION.expect("host function should be set")(HOST_CTX);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_host_function(_ctx: *mut c_void) {
    println!("Hello from Rust!");
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

unsafe impl Send for Program {}
unsafe impl Sync for Program {}

impl Drop for Program {
    fn drop(&mut self) {
        wamr_fns::wasm_runtime_destroy_exec_env(self.exec_env);
        wamr_fns::wasm_runtime_deinstantiate(self.instance);
        wamr_fns::wasm_runtime_unload(self.module);
    }
}

impl Program {
    pub fn new(aot_bytes: &mut [u8], err_buf: &mut [i8]) -> Self {
        let module = wamr_fns::wasm_runtime_load(
            aot_bytes.as_mut_ptr(),
            aot_bytes.len().try_into().expect("should fit"),
            err_buf.as_mut_ptr(),
            ERROR_BUFFER_SIZE.try_into().expect("should fit"),
        );

        if module.is_null() {
            panic!("Failed to load WASM module");
        }

        let instance = wamr_fns::wasm_runtime_instantiate(
            module,
            STACK_SIZE as u32,
            HEAP_SIZE as u32,
            err_buf.as_mut_ptr(),
            ERROR_BUFFER_SIZE as u32,
        );

        if instance.is_null() {
            wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to instantiate WASM module");
        }

        let exec_env = wamr_fns::wasm_runtime_create_exec_env(instance, 8192);
        if exec_env.is_null() {
            wamr_fns::wasm_runtime_deinstantiate(instance);
            wamr_fns::wasm_runtime_unload(module);
            panic!("Failed to create execution environment");
        }

        let program_func = wamr_fns::wasm_runtime_lookup_function(instance, c"program".as_ptr());

        if program_func.is_null() {
            wamr_fns::wasm_runtime_destroy_exec_env(exec_env);
            wamr_fns::wasm_runtime_deinstantiate(instance);
            wamr_fns::wasm_runtime_unload(module);
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

        if !wamr_fns::wasm_runtime_call_wasm_a(
            self.exec_env,
            self.program_func,
            1,
            call_results.as_mut_ptr(),
            0,
            std::ptr::null_mut(),
        ) {
            let ptr = wamr_fns::wasm_runtime_get_exception(self.instance);
            let msg = if ptr.is_null() {
                "unknown WAMR exception".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned()
            };

            panic!("WASM function call failed: {}", msg);
        }

        wamr_fns::wasm_val_t_get_i64(&call_results[0]) as u64
    }
}

pub struct WamrRuntime {
    heap: Vec<u8>,
    native_symbols: Vec<wamr::NativeSymbol>,
}

impl Drop for WamrRuntime {
    fn drop(&mut self) {
        wamr_fns::wasm_runtime_destroy();
    }
}

impl Default for WamrRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WamrRuntime {
    pub fn new() -> Self {
        let runtime = WamrRuntime {
            native_symbols: vec![wamr::NativeSymbol {
                symbol: c"call_host_function".as_ptr(),
                func_ptr: call_host_function as *mut c_void,
                signature: c"()".as_ptr(),
                ..Default::default()
            }],
            heap: vec![0; HEAP_SIZE],
        };

        let mut init_args = wamr::RuntimeInitArgs {
            mem_alloc_type: wamr::mem_alloc_type_t_Alloc_With_Pool,
            running_mode: wamr::RunningMode_Mode_Interp,
            mem_alloc_option: wamr::MemAllocOption {
                pool: wamr::MemAllocOption__bindgen_ty_1 {
                    heap_buf: runtime.heap.as_ptr() as *mut c_void,
                    heap_size: HEAP_SIZE as u32,
                },
            },
            ..Default::default()
        };

        if !wamr_fns::wasm_runtime_full_init(&mut init_args as *mut wamr::RuntimeInitArgs) {
            panic!("Failed to initialize WAMR runtime");
        }

        println!("Registering native functions...");

        if !wamr_fns::wasm_runtime_register_natives(
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

const MAX_PROGRAMS: usize = 256;

type ProgramArray = [Option<Program>; MAX_PROGRAMS];

struct SharedEvaluatorState {
    programs: [ProgramArray; 3],
    program_lens: [usize; 3],
}

pub struct Evaluator {
    state: Arc<Mutex<SharedEvaluatorState>>,
    current_idx: usize,
    init_thread: Option<thread::JoinHandle<Result<(), String>>>,
    _runtime: WamrRuntime,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Evaluator {
    fn drop(&mut self) {
        if let Some(thread) = self.init_thread.take() {
            thread.join().ok();
        }
    }
}

impl Evaluator {
    pub fn new() -> Self {
        const INIT: Option<Program> = None;
        Evaluator {
            state: Arc::new(Mutex::new(SharedEvaluatorState {
                programs: [
                    [INIT; MAX_PROGRAMS],
                    [INIT; MAX_PROGRAMS],
                    [INIT; MAX_PROGRAMS],
                ],
                program_lens: [0, 0, 0],
            })),
            current_idx: 0,
            init_thread: None,
            _runtime: WamrRuntime::new(),
        }
    }

    fn init_next(
        state: Arc<Mutex<SharedEvaluatorState>>,
        current_idx: usize,
        aot_bytes_vec: Vec<Vec<u8>>,
    ) -> Result<(), String> {
        let prev_idx = (current_idx + 2) % 3;
        let next_idx = (current_idx + 1) % 3;

        {
            let mut state_guard = state.lock().unwrap();
            for idx in 0..state_guard.program_lens[prev_idx] {
                state_guard.programs[prev_idx][idx] = None;
            }
        }

        let len = aot_bytes_vec.len();
        let mut new_programs = Vec::new();
        for mut aot_bytes in aot_bytes_vec {
            let mut err_buf = [0i8; ERROR_BUFFER_SIZE];
            let program = Program::new(&mut aot_bytes, &mut err_buf);
            new_programs.push(program);
        }

        {
            let mut state_guard = state.lock().unwrap();
            for (idx, program) in new_programs.into_iter().enumerate() {
                state_guard.programs[next_idx][idx] = Some(program);
            }
            state_guard.program_lens[next_idx] = len;
        }

        Ok(())
    }

    // NOTE: We need ownership of aot_bytes_vec because WAMR may modify it. We'll let the
    // caller worry about whether they need to clone it, but in most real-world cases they won't
    pub fn next_round(&mut self, aot_bytes_vec: Vec<Vec<u8>>) -> Result<(), String> {
        let join_start = Instant::now();
        if let Some(thread) = self.init_thread.take() {
            thread
                .join()
                .map_err(|_| "Thread join failed".to_string())??;
        }
        let join_duration = join_start.elapsed();
        println!("Join duration: {} ns", join_duration.as_nanos());

        let spawn_start = Instant::now();
        self.current_idx = (self.current_idx + 1) % 3;

        let state = Arc::clone(&self.state);
        let current_idx = self.current_idx;

        self.init_thread = Some(thread::spawn(move || {
            Self::init_next(state, current_idx, aot_bytes_vec)
        }));

        let spawn_duration = spawn_start.elapsed();
        println!("Spawn duration: {} ns", spawn_duration.as_nanos());

        let state_guard = self.state.lock().unwrap();
        for idx in 0..state_guard.program_lens[self.current_idx] {
            if let Some(program) = &state_guard.programs[self.current_idx][idx] {
                let start = Instant::now();
                let res = program.call();
                let duration = start.elapsed();
                println!(
                    "Program {} executed in {} ns with return value {}",
                    idx,
                    duration.as_nanos(),
                    res
                );
                assert_eq!(res, 1337);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluator() {
        unsafe {
            set_host_function(Some(rust_host_function), std::ptr::null_mut());
        }

        let aot_bytes = std::fs::read("zig-out/bin/program.aot").expect("Failed to read AOT file");

        let mut evaluator = Evaluator::new();

        let aot_bytes_vec = vec![
            aot_bytes.clone(),
            aot_bytes.clone(),
            aot_bytes.clone(),
            aot_bytes.clone(),
        ];

        for i in 0..10 {
            println!("\nIteration {}:", i + 1);
            evaluator
                .next_round(aot_bytes_vec.clone())
                .expect("Round failed");
        }

        println!("\nSleeping for 1 seconds before final iteration...");
        thread::sleep(std::time::Duration::from_millis(1000));

        let start = Instant::now();
        evaluator
            .next_round(aot_bytes_vec)
            .expect("Final round failed");
        let duration = start.elapsed();
        println!("Final iteration executed in {} ns", duration.as_nanos());

        println!("All iterations completed successfully.");
    }
}
