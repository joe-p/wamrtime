#[allow(warnings)]
pub mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod compiler;
pub mod evaluator;
pub mod program;
pub mod runtime;
mod unsafe_wamr_fns;

pub type Result<T> = color_eyre::Result<T>;

pub const ERROR_BUFFER_SIZE: usize = 128;

const KB: usize = 1024;

/// The size of the heap that each WAMR program gets
const APP_HEAP_SIZE: usize = 32 * KB;

/// The maximum number of WAMR programs that can be called per outer call
const MAX_WAMR_PROGRAM_REFERENCES: usize = 256;

/// The total runtime heap size needed to support all WAMR possible programs
const RUNTIME_HEAP_SIZE: usize = APP_HEAP_SIZE * MAX_WAMR_PROGRAM_REFERENCES;

/// Since everything is AoT, we don't use the WASM stack
/// See https://bytecodealliance.github.io/wamr.dev/blog/understand-the-wamr-stacks/
/// NOTE: PR to support this upstream is here: https://github.com/bytecodealliance/wasm-micro-runtime/pull/4688
const STACK_SIZE: u32 = 0;

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use crate::{ERROR_BUFFER_SIZE, Result, wamr};

    use super::compiler::Compiler;
    use super::evaluator::Evaluator;
    use super::runtime::{WamrHostFunction, WamrRuntime};
    use color_eyre::eyre::Context;

    const GAS_LIMIT: i64 = 1_000_000;
    static mut GAS_USED: i64 = 0;

    #[unsafe(no_mangle)]
    pub extern "C" fn host_gas_check_impl(exec_env: *mut c_void, requested_gas: i64) {
        unsafe {
            GAS_USED += requested_gas;
            if GAS_USED > GAS_LIMIT {
                let exec_env = exec_env as *mut wamr::WASMExecEnv;
                let module_inst = wamr::wasm_runtime_get_module_inst(exec_env);
                if !module_inst.is_null() {
                    wamr::wasm_runtime_set_exception(module_inst, c"Out of gas".as_ptr());
                }
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn call_host_function() {
        println!("Hello from Rust!");
    }

    #[test]
    fn test_evaluator() -> Result<()> {
        unsafe {
            GAS_USED = 0;
        }

        let runtime = WamrRuntime::new(
            host_gas_check_impl,
            vec![WamrHostFunction::new(
                "call_host_function".to_string(),
                call_host_function as *mut c_void,
                None,
                None,
            )],
        )?;

        let mut wasm_bytes = std::fs::read("../../zig-out/bin/program.wasm")
            .with_context(|| "Failed to read WASM file".to_string())?;
        let mut err_buf = vec![0i8; ERROR_BUFFER_SIZE];
        let compiler = Compiler::new(&runtime);
        println!("Compiling WASM to AOT...: {}", wasm_bytes.len());
        let aot_bytes = compiler.compile_wasm(&mut wasm_bytes, &mut err_buf)?;

        println!("AOT bytes length: {}", aot_bytes.len());
        println!("Error buffer (if any): {}", unsafe {
            std::ffi::CStr::from_ptr(err_buf.as_ptr()).to_string_lossy()
        });

        let mut evaluator = Evaluator::new(&runtime);

        let aot_bytes_vec = vec![aot_bytes.clone(); 10];

        evaluator.next_round(aot_bytes_vec.clone())?;
        evaluator.next_round(aot_bytes_vec.clone())?;

        for i in 0..aot_bytes_vec.len() {
            println!("\nIteration {}:", i + 1);
            let start = std::time::Instant::now();
            evaluator.call_program(i)?;
            let duration = start.elapsed();
            println!("Iteration {} completed in: {:?}", i + 1, duration);
        }

        println!("All iterations completed successfully.");
        Ok(())
    }
}
