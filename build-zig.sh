#!/bin/bash

# Build script for the Zig version of the WAMR playground

set -e

echo "Building WAMR vmlib with CMake..."

# Create build directory if it doesn't exist
mkdir -p build
cd build

# Configure and build with CMake to get the vmlib
cmake .. -DCMAKE_BUILD_TYPE=Release
make vmlib

# cd ..
#
# echo ""
# echo "Building Zig executable..."
#
# # Build the Zig executable
# zig build
#
# echo ""
# echo "Compiling app to WASM..."
#
# CURR_DIR=$PWD
# WAMR_DIR=${PWD}/wasm-micro-runtime
# OUT_DIR=${PWD}/out
#
# WASM_APPS=${PWD}/wasm-apps
#
# cd ${WASM_APPS}
#
# for i in `ls *.c`
# do
# APP_SRC="$i"
# OUT_FILE=${i%.*}.wasm
#
# # use WAMR SDK to build out the .wasm binary
# clang     \
#         --target=wasm32 -O0 -z stack-size=4096 -Wl,--initial-memory=65536 \
#         --sysroot=${WAMR_DIR}/wamr-sdk/app/libc-builtin-sysroot  \
#         -Wl,--allow-undefined-file=${WAMR_DIR}/wamr-sdk/app/libc-builtin-sysroot/share/defined-symbols.txt \
#         -Wl,--strip-all,--no-entry -nostdlib \
#         -Wl,--export=generate_float \
#         -Wl,--export=float_to_string \
#         -Wl,--export=calculate\
#         -Wl,--export=mul7\
#         -Wl,--allow-undefined \
#         -o ${OUT_DIR}/wasm-apps/${OUT_FILE} ${APP_SRC}
#
#
# if [ -f ${OUT_DIR}/wasm-apps/${OUT_FILE} ]; then
#         echo "build ${OUT_FILE} success"
# else
#         echo "build ${OUT_FILE} fail"
# fi
# done
#
# echo ""
#
# echo "AOT Compiling WASM bytecode..."
#
# ${WAMR_DIR}/wamr-compiler/build/wamrc-2.3.1 --size-level=3 --format=aot --cpu=apple-m4 -o ${OUT_DIR}/wasm-apps/testapp.aot ${OUT_DIR}/wasm-apps/testapp.wasm 
#
# wasm-dis ${OUT_DIR}/wasm-apps/*.wasm

