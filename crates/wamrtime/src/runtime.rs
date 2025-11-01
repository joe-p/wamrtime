use crate::RUNTIME_HEAP_SIZE;
use crate::unsafe_wamr_fns;
use crate::wamr;
use std::ffi::c_void;
use std::fmt::Display;

pub struct WamrRuntime {
    heap: Vec<u8>,
    native_symbols: Vec<wamr::NativeSymbol>,
    // A Vec to hold CString references to ensure they live as long as the runtime
    // This seems preferable to having to deal with lifetimes since the allocations
    // only happen once at runtime initialization
    _c_strings: Vec<std::ffi::CString>,
}

/// Safety: We are ensuring that we can use WamrRuntime in LazyLock
/// Maybe in the future we use use once_cell::unsync::Lazy?
unsafe impl Sync for WamrRuntime {}
unsafe impl Send for WamrRuntime {}

impl Drop for WamrRuntime {
    fn drop(&mut self) {
        unsafe_wamr_fns::wasm_runtime_destroy();
    }
}

type HostGasCheckFn = unsafe extern "C" fn(exec_env: *mut c_void, requested_gas: i64);

pub enum WamrType {
    I64,
    I32,
    ByteSlice,
    MutByteSlice,
}

impl Display for WamrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WamrType::I64 => write!(f, "I"),
            WamrType::I32 => write!(f, "i"),
            WamrType::ByteSlice => write!(f, "*~"),
            WamrType::MutByteSlice => write!(f, "*~"),
        }
    }
}

pub struct WamrHostFunction {
    name: String,
    function: *mut c_void,
    args: Option<Vec<WamrType>>,
    return_type: Option<WamrType>,
}

impl WamrHostFunction {
    pub fn new(
        name: String,
        function: *mut c_void,
        args: Option<Vec<WamrType>>,
        return_type: Option<WamrType>,
    ) -> Self {
        WamrHostFunction {
            name: name.to_string(),
            function,
            args,
            return_type,
        }
    }

    pub fn signature(&self) -> String {
        let mut signature = String::new();
        signature.push('(');
        if let Some(args) = &self.args {
            for arg in args {
                signature.push_str(&arg.to_string());
            }
        }
        signature.push(')');
        if let Some(ret_type) = &self.return_type {
            signature.push_str(&ret_type.to_string());
        }
        signature
    }
}

impl WamrRuntime {
    pub fn new(host_gas_check_fn: HostGasCheckFn, host_functions: Vec<WamrHostFunction>) -> Self {
        let mut c_strings: Vec<std::ffi::CString> = vec![];
        let mut native_symbols: Vec<wamr::NativeSymbol> = host_functions
            .iter()
            .map(|host_fn| {
                c_strings.push(std::ffi::CString::new(host_fn.name.clone()).unwrap());
                c_strings.push(std::ffi::CString::new(host_fn.signature()).unwrap());
                let symbol = c_strings[c_strings.len() - 2].as_ptr();
                let signature = c_strings[c_strings.len() - 1].as_ptr();

                wamr::NativeSymbol {
                    symbol,
                    func_ptr: host_fn.function,
                    signature,
                    ..Default::default()
                }
            })
            .collect();

        native_symbols.push(wamr::NativeSymbol {
            symbol: c"host_gas_check".as_ptr(),
            func_ptr: host_gas_check_fn as *mut c_void,
            signature: c"(I)".as_ptr(),
            ..Default::default()
        });

        let runtime = WamrRuntime {
            native_symbols,
            heap: Vec::with_capacity(RUNTIME_HEAP_SIZE),
            _c_strings: c_strings,
        };

        let mut init_args = wamr::RuntimeInitArgs {
            mem_alloc_type: wamr::mem_alloc_type_t_Alloc_With_Pool,
            running_mode: wamr::RunningMode_Mode_Interp,
            mem_alloc_option: wamr::MemAllocOption {
                pool: wamr::MemAllocOption__bindgen_ty_1 {
                    heap_buf: runtime.heap.as_ptr() as *mut c_void,
                    heap_size: RUNTIME_HEAP_SIZE as u32,
                },
            },
            ..Default::default()
        };

        if !unsafe_wamr_fns::wasm_runtime_full_init(&mut init_args as *mut wamr::RuntimeInitArgs) {
            panic!("Failed to initialize WAMR runtime");
        }

        if !unsafe_wamr_fns::wasm_runtime_register_natives(
            c"env".as_ptr(),
            runtime.native_symbols.as_ptr() as *mut wamr::NativeSymbol,
            runtime.native_symbols.len() as u32,
        ) {
            panic!("Failed to register native symbols");
        }

        runtime
    }
}
