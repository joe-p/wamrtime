const WAMR_ROOT: &str = "wasm-micro-runtime";

fn add_include_path(builder: bindgen::Builder, path: &str) -> bindgen::Builder {
    builder.clang_arg(format!("-I{}/{}", WAMR_ROOT, path))
}

fn main() {
    println!("cargo:rerun-if-changed={}/core", WAMR_ROOT);

    let mut builder = bindgen::Builder::default()
        .header(format!("{}/core/iwasm/include/wasm_export.h", WAMR_ROOT));

    builder = add_include_path(builder, "core/iwasm/include");
    builder = add_include_path(builder, "core/iwasm/interpreter");
    builder = add_include_path(builder, "core/iwasm/aot");
    builder = add_include_path(builder, "core/iwasm/libraries/libc-builtin");
    builder = add_include_path(builder, "core/iwasm/common");
    builder = add_include_path(builder, "core/shared/include");
    builder = add_include_path(builder, "core/shared/platform/include");
    builder = add_include_path(builder, "core/shared/platform/linux");
    builder = add_include_path(builder, "core/shared/platform/common/posix");
    builder = add_include_path(builder, "core/shared/mem-alloc");
    builder = add_include_path(builder, "core/shared/utils");
    builder = add_include_path(builder, "core/shared/utils/uncommon");

    println!("cargo:rustc-link-search=native=build");
    println!("cargo:rustc-link-lib=static=vmlib");

    println!("cargo:rustc-link-lib=dylib=c");
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=dl");
    println!("cargo:rustc-link-lib=dylib=pthread");

    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=dylib=rt");

    let bindings = builder
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .derive_default(true)
        .trust_clang_mangling(false)
        .layout_tests(false)
        .generate_comments(false)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    std::process::Command::new("zig")
        .args([
            "build-exe",
            "wasm-src/wasm.zig",
            "-target",
            "wasm32-freestanding",
            "-O",
            "ReleaseSmall",
            "--import-memory",
            "--no-entry",
            "-rdynamic",
            "-femit-bin=zig-out/bin/program.wasm",
        ])
        .status()
        .expect("Failed to build WASM module");

    println!("cargo:rerun-if-changed=wasm-src/wasm.zig");
}
