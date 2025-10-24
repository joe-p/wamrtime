use std::ffi::c_void;

#[allow(warnings)]
mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

fn main() {
    let aot_bytes = std::fs::read("zig-out/bin/program.aot").expect("Failed to read AOT file");

    unsafe {
        let t = wamr::get_package_type(aot_bytes.as_ptr(), aot_bytes.len() as u32);
        println!("Package type: {}", t);
    }

    unsafe {
        const HEAP_SIZE: usize = 1024 * 1024 * 2;
        let heap_buf: Vec<u8> = vec![0; HEAP_SIZE];

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

        wamr::wasm_runtime_full_init(&mut init_args as *mut wamr::RuntimeInitArgs);
    }
}
