package main

/*
#cgo LDFLAGS: ${SRCDIR}/target/debug/libwamrtime_avm.a ${SRCDIR}/target/debug/libwamrtime.a -L/opt/homebrew/opt/zstd/lib -lc++ -lz -lzstd -ldl -lpthread -lm
#include <stdint.h>
#include <stdlib.h>

// The function exposed by the Rust library to run the test.
void test_run();

// Types and definitions for AVM functions.

typedef uint64_t (*AvmGetGlobalUintFn)(void* exec_env, uint64_t app, const uint8_t* key_ptr, uint32_t key_len);

extern uint64_t goGetGlobalUint(void* exec_env, uint64_t app, uint8_t* key_ptr, uint32_t key_len);

static inline AvmGetGlobalUintFn getGoGetGlobalUint() {
	return (AvmGetGlobalUintFn)goGetGlobalUint;
}

extern void goSetGlobalUint(void* exec_env, uint64_t app, uint8_t* key_ptr, uint32_t key_len, uint64_t value);

typedef void (*AvmSetGlobalUintFn)(void* exec_env, uint64_t app, const uint8_t* key_ptr, uint32_t key_len, uint64_t value);

static inline AvmSetGlobalUintFn getGoSetGlobalUint() {
	return (AvmSetGlobalUintFn)goSetGlobalUint;
}

// The function used to initialize the WAMR runtime with Go callbacks.
void avm_init(void* ctx, AvmGetGlobalUintFn get_global_uint_impl, AvmSetGlobalUintFn set_global_uint_impl);
*/
import "C"

import (
	"runtime"
	"sync"
	"unsafe"
)

var globalState = struct {
	sync.Mutex
	data map[string]uint64
}{
	data: make(map[string]uint64),
}

func main() {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	C.avm_init(nil, C.getGoGetGlobalUint(), C.getGoSetGlobalUint())
	C.test_run()
}

//export goGetGlobalUint
func goGetGlobalUint(execEnv unsafe.Pointer, app uint64, keyPtr *C.uint8_t, keyLen C.uint32_t) uint64 {
	key := C.GoBytes(unsafe.Pointer(keyPtr), C.int(keyLen))

	globalState.Lock()
	value := globalState.data[string(key)]
	globalState.Unlock()

	return value
}

//export goSetGlobalUint
func goSetGlobalUint(execEnv unsafe.Pointer, app uint64, keyPtr *C.uint8_t, keyLen C.uint32_t, value uint64) {
	key := C.GoBytes(unsafe.Pointer(keyPtr), C.int(keyLen))

	globalState.Lock()
	globalState.data[string(key)] = value
	globalState.Unlock()
}
