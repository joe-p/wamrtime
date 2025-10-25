use std::ffi::c_void;
use std::time::Instant;

#[allow(warnings)]
mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
mod compiler;
mod evaluator;
mod program;
mod runtime;
mod unsafe_wamr_fns;

use compiler::Compiler;
use evaluator::Evaluator;
use runtime::WamrRuntime;

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

#[unsafe(no_mangle)]
pub extern "C" fn rust_host_function(_ctx: *mut c_void) {
    println!("Hello from Rust!");
}

const ERROR_BUFFER_SIZE: usize = 128;

const HEAP_SIZE: usize = 1024 * 1024 * 2;
const STACK_SIZE: usize = 1024 * 128;

#[unsafe(no_mangle)]
pub extern "C" fn test_run() {
    let runtime = WamrRuntime::new();

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

    #[test]
    fn test_evaluator() {
        unsafe {
            set_host_function(Some(rust_host_function), std::ptr::null_mut());
        }

        test_run();
    }
}
