#!/bin/bash

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WAMR_DIR=${ROOT}/wasm-micro-runtime
set -ex

cd ${ROOT}

echo "Building WAMR vmlib with CMake..."

# Create build directory if it doesn't exist
mkdir -p build
cd build

# Configure and build with CMake to get the vmlib
cmake .. -DCMAKE_BUILD_TYPE=Release
make vmlib

cd ${WAMR_DIR}/wamr-compiler
bash build_llvm.sh
mkdir -p build
cd build
cmake .. -DWAMR_BUILD_PLATFORM=darwin
make

cd ${ROOT}

# for each directory in wasm_crates, build the wasm module
for dir in wasm_crates/*/; do
    (cd "$dir" && cargo build --profile wasm_small --target wasm32-unknown-unknown)
done

cargo build --workspace --exclude avm_complex --exclude avm_blank_key --exclude state_loop

cargo run -p wamrtime-avm-bindgen -- /Users/joe/git/algorand/go-algorand/wamrtime/crates/wamrtime_avm/src/lib.rs /Users/joe/git/algorand/go-algorand/data/transactions/logic/eval.go
 
