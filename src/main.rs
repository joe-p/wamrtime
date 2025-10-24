#[allow(warnings)]
mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

fn main() {
    const HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 MB
    let mut heap_buf = vec![0u8; HEAP_SIZE];

    // Manually zero the struct to match Zig's std.mem.zeroes
    let mut init_args = unsafe { std::mem::zeroed::<wamr::RuntimeInitArgs>() };

    init_args.mem_alloc_type = wamr::mem_alloc_type_t_Alloc_With_Pool;
    init_args.mem_alloc_option.pool.heap_buf = heap_buf.as_mut_ptr() as *mut std::ffi::c_void;
    init_args.mem_alloc_option.pool.heap_size = HEAP_SIZE as u32;
    init_args.running_mode = wamr::RunningMode_Mode_Interp;

    unsafe {
        println!("Initializing WASM runtime...");
        let result = wamr::wasm_runtime_full_init(&mut init_args);
        if !result {
            eprintln!("Failed to initialize WASM runtime");
            return;
        }
        println!("WASM runtime initialized successfully");

        println!("Destroying WASM runtime...");
        wamr::wasm_runtime_destroy();
        println!("WASM runtime destroyed");
    }
}
