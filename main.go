package main

/*
#cgo LDFLAGS: ${SRCDIR}/target/debug/libwamrtime_avm.a ${SRCDIR}/target/debug/libwamrtime.a -L/opt/homebrew/opt/zstd/lib -lc++ -lz -lzstd -ldl -lpthread -lm
#include <stdint.h>
#include <stdlib.h>

// The function exposed by the Rust library to run the test.
void test_run();

// Types and definitions for the AVM dispatcher.

typedef uint64_t (*AvmDispatcher)(void* ctx, uint64_t function, const uint64_t* args, uint32_t arg_count, uint64_t* ret_ptr);

void set_avm_dispatcher(AvmDispatcher dispatcher, void* ctx);

extern uint64_t goAvmDispatcher(void* ctx, uint64_t function, uint64_t* args, uint32_t arg_count, uint64_t* ret_ptr);

static inline AvmDispatcher getGoDispatcher() {
	return (AvmDispatcher)goAvmDispatcher;
}
*/
import "C"

import (
	"fmt"
	"runtime"
	"sync"
	"unsafe"
)

const (
	AvmFunctionGetGlobalUint = 0
	AvmFunctionSetGlobalUint = 1
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

	C.set_avm_dispatcher(C.getGoDispatcher(), nil)
	C.test_run()
}

//export goAvmDispatcher
func goAvmDispatcher(ctx unsafe.Pointer, function uint64, args *uint64, argCount uint32, retPtr *uint64) uint64 {
	argsSlice := unsafe.Slice(args, argCount)

	switch function {
	case AvmFunctionGetGlobalUint:
		if argCount != 3 {
			panic(fmt.Sprintf("GetGlobalUint expected 3 args, got %d", argCount))
		}
		keyPtr := (*byte)(unsafe.Pointer(uintptr(argsSlice[1])))
		keyLen := int(argsSlice[2])
		key := unsafe.Slice(keyPtr, keyLen)

		globalState.Lock()
		value := globalState.data[string(key)]
		globalState.Unlock()

		retSlice := unsafe.Slice(retPtr, 1)
		retSlice[0] = value
		return 1

	case AvmFunctionSetGlobalUint:
		if argCount != 4 {
			panic(fmt.Sprintf("SetGlobalUint expected 4 args, got %d", argCount))
		}
		keyPtr := (*byte)(unsafe.Pointer(uintptr(argsSlice[1])))
		keyLen := int(argsSlice[2])
		key := unsafe.Slice(keyPtr, keyLen)
		value := argsSlice[3]

		globalState.Lock()
		globalState.data[string(key)] = value
		globalState.Unlock()

		return 0

	default:
		panic(fmt.Sprintf("Unknown function ID: %d", function))
	}
}
