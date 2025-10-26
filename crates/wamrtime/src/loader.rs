use std::path::PathBuf;

use crate::compiler::Compiler;
#[allow(deprecated)]
use sha2::digest::generic_array::GenericArray;
use sha2::{Digest, Sha512_256, digest::consts::U32};

#[allow(deprecated)]
type HashOutput = GenericArray<u8, U32>;
fn fmt_hash(hash: HashOutput) -> String {
    hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

pub trait AotLoader {
    fn aot_from_hash(&self, wasm_hash: HashOutput) -> Option<Vec<u8>>;
    fn aot_from_wasm(&self, wasm_bytes: &[u8]) -> Vec<u8>;
}

pub struct FsLoader<'compiler> {
    compiler: Compiler<'compiler>,
    aot_dir: PathBuf,
}

impl<'compiler> FsLoader<'compiler> {
    pub fn new(compiler: Compiler<'compiler>, aot_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&aot_dir).expect("Failed to create AOT directory");
        FsLoader { compiler, aot_dir }
    }
}

impl AotLoader for FsLoader<'_> {
    fn aot_from_hash(&self, wasm_hash: HashOutput) -> Option<Vec<u8>> {
        let aot_path = self.aot_dir.join(fmt_hash(wasm_hash));
        if aot_path.exists() {
            Some(std::fs::read(aot_path).expect("Failed to read AOT file"))
        } else {
            None
        }
    }

    fn aot_from_wasm(&self, wasm_bytes: &[u8]) -> Vec<u8> {
        let mut hasher = Sha512_256::new();
        hasher.update(wasm_bytes);
        let wasm_hash = hasher.finalize();

        if let Some(aot_bytes) = self.aot_from_hash(wasm_hash) {
            println!("AOT file found for hash {}", fmt_hash(wasm_hash));
            return aot_bytes;
        }

        println!(
            "AOT file not found for hash {}, compiling...",
            fmt_hash(wasm_hash)
        );
        let mut wasm_bytes_mut = wasm_bytes.to_vec();
        let aot_bytes = self.compiler.compile_wasm(&mut wasm_bytes_mut);

        let aot_path = self.aot_dir.join(fmt_hash(wasm_hash));
        std::fs::write(&aot_path, &aot_bytes).expect("Failed to write AOT file");
        println!("AOT file written to {:?}", aot_path);

        aot_bytes
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::runtime::WamrRuntime;

    extern "C" fn host_gas_check_impl(_exec_env: *mut std::ffi::c_void, _requested_gas: i64) {
        // No-op for testing
    }

    #[test]
    fn test_fs_loader() {
        let runtime = WamrRuntime::new(host_gas_check_impl, vec![]);
        let compiler = Compiler::new(&runtime);
        let loader = FsLoader::new(compiler, PathBuf::from("./test_aot_cache"));

        let wasm_bytes =
            std::fs::read("../../zig-out/bin/program.wasm").expect("Failed to read WASM file");

        let aot_bytes_first = loader.aot_from_wasm(&wasm_bytes);
        let aot_bytes_second = loader.aot_from_wasm(&wasm_bytes);

        assert_eq!(aot_bytes_first, aot_bytes_second);
    }
}
