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
}
