CURR_DIR=$PWD
WAMR_DIR=${PWD}/wasm-micro-runtime
OUT_DIR=${PWD}/out

${WAMR_DIR}/wamr-compiler/build/wamrc-2.4.3 --size-level=3 --format=aot --cpu=apple-m4 -o ${OUT_DIR}/wasm-apps/testapp.aot ${OUT_DIR}/wasm-apps/testapp.wasm 

