use std::ffi::c_void;
use std::time::Instant;

use wamrtime::compiler::Compiler;
use wamrtime::evaluator::Evaluator;
use wamrtime::runtime::{WamrHostFunction, WamrRuntime};

pub type HostFunction = unsafe extern "C" fn(ctx: *mut c_void);

static mut HOST_FUNCTION: Option<HostFunction> = None;
static mut HOST_CTX: *mut c_void = core::ptr::null_mut();

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_host_function() {
    unsafe {
        HOST_FUNCTION.expect("host function should be set")(HOST_CTX);
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_host_function(host_fn: Option<HostFunction>, ctx: *mut c_void) {
    unsafe {
        HOST_FUNCTION = host_fn;
        HOST_CTX = ctx;
    }
}

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
pub extern "C" fn test_run() {
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

    let start = Instant::now();
    evaluator
        .next_round(aot_bytes_vec)
        .expect("Final round failed");
    let duration = start.elapsed();
    println!("Final iteration executed in {} ns", duration.as_nanos());

    println!("All iterations completed successfully.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_host_function(_ctx: *mut c_void) {
        println!("Hello from Rust!");
    }

    #[test]
    fn test_evaluator() {
        unsafe {
            set_host_function(Some(rust_host_function), std::ptr::null_mut());
        }

        test_run();
    }
}
