const WAMR_ROOT: &str = "wasm-micro-runtime";

fn add_include_path(builder: bindgen::Builder, path: &str) -> bindgen::Builder {
    builder.clang_arg(format!("-I{}/{}", WAMR_ROOT, path))
}

fn main() {
    println!("cargo:rerun-if-changed={}/core", WAMR_ROOT);

    let mut builder = bindgen::Builder::default()
        .header(format!("{}/core/iwasm/include/wasm_export.h", WAMR_ROOT))
        .header(format!("{}/core/iwasm/include/aot_export.h", WAMR_ROOT));

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

    let llvm_build = format!("{}/core/deps/llvm/build", WAMR_ROOT);
    if std::path::Path::new(&llvm_build).exists() {
        println!("cargo:rustc-link-search=native={}/lib", llvm_build);

        let llvm_libs = [
            "LLVMHipStdPar",
            "LLVMPasses",
            "LLVMAArch64AsmParser",
            "LLVMAArch64CodeGen",
            "LLVMAArch64Desc",
            "LLVMAArch64Disassembler",
            "LLVMAArch64Info",
            "LLVMAArch64Utils",
            "LLVMARMAsmParser",
            "LLVMARMCodeGen",
            "LLVMARMDesc",
            "LLVMARMDisassembler",
            "LLVMARMInfo",
            "LLVMARMUtils",
            "LLVMMipsAsmParser",
            "LLVMMipsCodeGen",
            "LLVMMipsDesc",
            "LLVMMipsDisassembler",
            "LLVMMipsInfo",
            "LLVMRISCVAsmParser",
            "LLVMRISCVCodeGen",
            "LLVMRISCVDesc",
            "LLVMRISCVDisassembler",
            "LLVMRISCVInfo",
            "LLVMRISCVTargetMCA",
            "LLVMX86AsmParser",
            "LLVMX86CodeGen",
            "LLVMX86Desc",
            "LLVMX86Disassembler",
            "LLVMX86Info",
            "LLVMX86TargetMCA",
            "LLVMAggressiveInstCombine",
            "LLVMAnalysis",
            "LLVMAsmParser",
            "LLVMAsmPrinter",
            "LLVMBinaryFormat",
            "LLVMBitReader",
            "LLVMBitstreamReader",
            "LLVMBitWriter",
            "LLVMCFGuard",
            "LLVMCodeGen",
            "LLVMCodeGenTypes",
            "LLVMCore",
            "LLVMCoroutines",
            "LLVMCoverage",
            "LLVMDebugInfoCodeView",
            "LLVMDebugInfoDWARF",
            "LLVMDebugInfoMSF",
            "LLVMDebugInfoPDB",
            "LLVMDemangle",
            "LLVMDlltoolDriver",
            "LLVMExecutionEngine",
            "LLVMExtensions",
            "LLVMFileCheck",
            "LLVMFrontendOpenACC",
            "LLVMFrontendOffloading",
            "LLVMFrontendOpenMP",
            "LLVMGlobalISel",
            "LLVMIRPrinter",
            "LLVMIRReader",
            "LLVMInstCombine",
            "LLVMInstrumentation",
            "LLVMInterfaceStub",
            "LLVMInterpreter",
            "LLVMJITLink",
            "LLVMLTO",
            "LLVMLineEditor",
            "LLVMLinker",
            "LLVMMC",
            "LLVMMCA",
            "LLVMMCDisassembler",
            "LLVMMCJIT",
            "LLVMMCParser",
            "LLVMMIRParser",
            "LLVMObjCARCOpts",
            "LLVMObjCopy",
            "LLVMObject",
            "LLVMObjectYAML",
            "LLVMOption",
            "LLVMOrcJIT",
            "LLVMOrcShared",
            "LLVMOrcTargetProcess",
            "LLVMProfileData",
            "LLVMRemarks",
            "LLVMRuntimeDyld",
            "LLVMScalarOpts",
            "LLVMSelectionDAG",
            "LLVMSupport",
            "LLVMSymbolize",
            "LLVMTableGen",
            "LLVMTableGenCommon",
            "LLVMTableGenGlobalISel",
            "LLVMTarget",
            "LLVMTargetParser",
            "LLVMTextAPI",
            "LLVMTransformUtils",
            "LLVMVectorize",
            "LLVMWindowsDriver",
            "LLVMWindowsManifest",
            "LLVMipo",
        ];

        for lib in llvm_libs {
            println!("cargo:rustc-link-lib=static={}", lib);
        }

        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
        }
    }

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    }

    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=zstd");
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
        .generate_comments(true)
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
