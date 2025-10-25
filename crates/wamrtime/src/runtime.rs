use crate::unsafe_wamr_fns;
use crate::wamr;
use crate::{HEAP_SIZE, HOST_CTX, HOST_FUNCTION};
use std::ffi::c_void;

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_host_function() {
    unsafe {
        HOST_FUNCTION.expect("host function should be set")(HOST_CTX);
    }
}

const GAS_LIMIT: i64 = 1_000_000;
static mut GAS_USED: i64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn host_gas_check(_exec_env: *mut c_void, requested_gas: i64) {
    unsafe {
        GAS_USED += requested_gas;
        if GAS_USED > GAS_LIMIT {
            panic!("Out of gas");
        }
    }
}

pub struct WamrRuntime {
    heap: Vec<u8>,
    native_symbols: Vec<wamr::NativeSymbol>,
}

impl Drop for WamrRuntime {
    fn drop(&mut self) {
        unsafe_wamr_fns::wasm_runtime_destroy();
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
            native_symbols: vec![
                wamr::NativeSymbol {
                    symbol: c"call_host_function".as_ptr(),
                    func_ptr: call_host_function as *mut c_void,
                    signature: c"()".as_ptr(),
                    ..Default::default()
                },
                wamr::NativeSymbol {
                    symbol: c"host_gas_check".as_ptr(),
                    func_ptr: host_gas_check as *mut c_void,
                    signature: c"(I)".as_ptr(),
                    ..Default::default()
                },
            ],
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

        if !unsafe_wamr_fns::wasm_runtime_full_init(&mut init_args as *mut wamr::RuntimeInitArgs) {
            panic!("Failed to initialize WAMR runtime");
        }

        println!("Registering native functions...");

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
