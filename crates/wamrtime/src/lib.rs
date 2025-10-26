#[allow(warnings)]
pub mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod compiler;
pub mod evaluator;
pub mod program;
pub mod runtime;
mod unsafe_wamr_fns;

const ERROR_BUFFER_SIZE: usize = 128;

const HEAP_SIZE: usize = 1024 * 1024 * 2;
const STACK_SIZE: usize = 1024 * 128;

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

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
        let compiler = Compiler::new(&runtime);
        let aot_bytes = compiler.compile_wasm(&mut wasm_bytes);

        let mut evaluator = Evaluator::new(&runtime);

        let aot_bytes_vec = vec![
            aot_bytes.clone(),
            aot_bytes.clone(),
            aot_bytes.clone(),
            aot_bytes.clone(),
        ];

        for i in 0..3 {
            println!("\nIteration {}:", i + 1);
            evaluator
                .next_round(aot_bytes_vec.clone())
                .expect("Round failed");
        }

        let start = std::time::Instant::now();
        evaluator
            .next_round(aot_bytes_vec)
            .expect("Final round failed");
        let duration = start.elapsed();
        println!("Final iteration executed in {} ns", duration.as_nanos());

        println!("All iterations completed successfully.");
    }
}
