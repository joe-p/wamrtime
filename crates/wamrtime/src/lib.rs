#[allow(warnings)]
pub mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod compiler;
pub mod evaluator;
pub mod program;
pub mod runtime;
mod unsafe_wamr_fns;

pub const ERROR_BUFFER_SIZE: usize = 128;

const KB: usize = 1024;

/// The size of the heap that each WAMR program gets
const APP_HEAP_SIZE: usize = 32 * KB;

/// The maximum number of WAMR programs that can be called per outer call
const MAX_WAMR_PROGRAM_REFERENCES: usize = 256;

/// The maximum number of outer calls in a group
const MAX_OUTER_CALLS: usize = 16;

/// The total runtime heap size needed to support all WAMR possible programs
const RUNTIME_HEAP_SIZE: usize = APP_HEAP_SIZE * (MAX_WAMR_PROGRAM_REFERENCES + MAX_OUTER_CALLS);

/// Since everything is AoT, we don't use the WASM stack
/// See https://bytecodealliance.github.io/wamr.dev/blog/understand-the-wamr-stacks/
/// NOTE: PR to support this upstream is here: https://github.com/bytecodealliance/wasm-micro-runtime/pull/4688
const STACK_SIZE: u32 = 0;

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use crate::ERROR_BUFFER_SIZE;

    use super::compiler::Compiler;
    use super::evaluator::Evaluator;
    use super::runtime::{WamrHostFunction, WamrRuntime};

    const GAS_LIMIT: i64 = 1_000_000;
    static mut GAS_USED: i64 = 0;

    #[unsafe(no_mangle)]
    pub extern "C" fn host_gas_check_impl(_exec_env: *mut c_void, requested_gas: i64) {
        unsafe {
            GAS_USED += requested_gas;
            if GAS_USED > GAS_LIMIT {
                panic!("Out of gas");
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn call_host_function() {
        println!("Hello from Rust!");
    }

    #[test]
    fn test_evaluator() {
        let runtime = WamrRuntime::new(
            host_gas_check_impl,
            vec![WamrHostFunction::new(
                "call_host_function".to_string(),
                call_host_function as *mut c_void,
                None,
                None,
            )],
        );

        let mut wasm_bytes =
            std::fs::read("../../zig-out/bin/program.wasm").expect("Failed to read WASM file");
        let mut err_buf = Vec::with_capacity(ERROR_BUFFER_SIZE);
        let compiler = Compiler::new(&runtime);
        println!("Compiling WASM to AOT...: {}", wasm_bytes.len());
        let aot_bytes = compiler.compile_wasm(&mut wasm_bytes, &mut err_buf);

        println!("AOT bytes length: {}", aot_bytes.len());
        println!("Error buffer (if any): {}", unsafe {
            std::ffi::CStr::from_ptr(err_buf.as_ptr()).to_string_lossy()
        });

        let mut evaluator = Evaluator::new(&runtime);

        let aot_bytes_vec = vec![aot_bytes.clone(); 10];

        evaluator
            .next_round(aot_bytes_vec.clone())
            .expect("Initial round failed");
        evaluator
            .next_round(aot_bytes_vec.clone())
            .expect("Second round failed");

        for i in 0..aot_bytes_vec.len() {
            println!("\nIteration {}:", i + 1);
            let start = std::time::Instant::now();
            evaluator.call_program(i).expect("Program call failed");
            let duration = start.elapsed();
            println!("Iteration {} completed in: {:?}", i + 1, duration);
        }

        println!("All iterations completed successfully.");
    }
}
