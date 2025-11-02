use std::path::PathBuf;

use crate::{compiler::Compiler, ERROR_BUFFER_SIZE, Result};
use color_eyre::eyre::Context;
#[allow(deprecated)]
use sha2::digest::generic_array::GenericArray;
use sha2::{digest::consts::U32, Digest, Sha512_256};

#[allow(deprecated)]
type HashOutput = GenericArray<u8, U32>;
fn fmt_hash(hash: &HashOutput) -> String {
    hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

pub trait AotLoader {
    fn aot_from_hash(&self, wasm_hash: HashOutput) -> Result<Option<Vec<u8>>>;
    fn aot_from_wasm(&self, wasm_bytes: &[u8]) -> Result<Vec<u8>>;
}

pub struct FsLoader<'compiler> {
    compiler: Compiler<'compiler>,
    aot_dir: PathBuf,
}

impl<'compiler> FsLoader<'compiler> {
    pub fn new(compiler: Compiler<'compiler>, aot_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&aot_dir)
            .with_context(|| format!("Failed to create AOT directory at {:?}", &aot_dir))?;
        Ok(FsLoader { compiler, aot_dir })
    }
}

impl AotLoader for FsLoader<'_> {
    fn aot_from_hash(&self, wasm_hash: HashOutput) -> Result<Option<Vec<u8>>> {
        let hash_str = fmt_hash(&wasm_hash);
        let aot_path = self.aot_dir.join(&hash_str);
        if aot_path.exists() {
            let bytes = std::fs::read(&aot_path)
                .with_context(|| format!("Failed to read AOT file at {:?}", &aot_path))?;
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    fn aot_from_wasm(&self, wasm_bytes: &[u8]) -> Result<Vec<u8>> {
        let mut hasher = Sha512_256::new();
        hasher.update(wasm_bytes);
        let wasm_hash = hasher.finalize();
        let hash_str = fmt_hash(&wasm_hash);

        if let Some(aot_bytes) = self.aot_from_hash(wasm_hash.clone())? {
            println!("AOT file found for hash {}", hash_str);
            return Ok(aot_bytes);
        }

        println!(
            "AOT file not found for hash {}, compiling...",
            hash_str
        );
        let mut wasm_bytes_mut = wasm_bytes.to_vec();
        let mut err_buf = vec![0i8; ERROR_BUFFER_SIZE];
        let aot_bytes = self
            .compiler
            .compile_wasm(&mut wasm_bytes_mut, &mut err_buf)?;

        let aot_path = self.aot_dir.join(&hash_str);
        std::fs::write(&aot_path, &aot_bytes)
            .with_context(|| format!("Failed to write AOT file at {:?}", &aot_path))?;
        println!("AOT file written to {:?}", aot_path);

        Ok(aot_bytes)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{runtime::WamrRuntime, Result};
    use color_eyre::eyre::Context;

    extern "C" fn host_gas_check_impl(_exec_env: *mut std::ffi::c_void, _requested_gas: i64) {
        // No-op for testing
    }

    #[test]
    fn test_fs_loader() -> Result<()> {
        let runtime = WamrRuntime::new(host_gas_check_impl, vec![])?;
        let compiler = Compiler::new(&runtime);
        let loader = FsLoader::new(compiler, PathBuf::from("./test_aot_cache"))?;

        let wasm_bytes = std::fs::read("../../zig-out/bin/program.wasm")
            .with_context(|| "Failed to read WASM file".to_string())?;

        let aot_bytes_first = loader.aot_from_wasm(&wasm_bytes)?;
        let aot_bytes_second = loader.aot_from_wasm(&wasm_bytes)?;

        assert_eq!(aot_bytes_first, aot_bytes_second);
        Ok(())
    }
}
