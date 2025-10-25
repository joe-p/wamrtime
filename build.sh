#!/bin/bash

ROOT=${PWD}
WAMR_DIR=${PWD}/wasm-micro-runtime
OUT_DIR=${PWD}/zig-out/bin
set -ex

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

zig build

${WAMR_DIR}/wamr-compiler/build/wamrc-2.4.3 --size-level=3 --format=aot --cpu=apple-m4 -o ${OUT_DIR}/program.aot ${OUT_DIR}/program.wasm

cargo t
