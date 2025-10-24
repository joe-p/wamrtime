package main

/*
#cgo LDFLAGS: ${SRCDIR}/target/debug/libwamrtime.a -ldl -lpthread -lm
#include <stdint.h>
#include <stdlib.h>

typedef void (*HostFunction)(void* ctx);

// These are provided by Go via //export (below).
extern void goHostFunction(void* ctx);

// Helper to get a function pointer with correct type in C
static inline HostFunction getGoHostFunction() { return (HostFunction)goHostFunction; }

// Rust functions we link to:
void set_host_function(HostFunction cb, void* ctx);
void test_run();
*/
import "C"

import (
	"fmt"
	"runtime"
	"runtime/cgo"
	"unsafe"
)

// The Go function we actually want to run when Rust calls back.
type Handler func(code int32, msg []byte)

func main() {
	// If you care about strict same-thread execution for callbacks, you may lock.
	// This ensures Rust's synchronous callback happens on the calling OS thread.
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	// Create a Go handler (could be a closure capturing state).
	h := func() {
		fmt.Printf("Go handler")
	}

	// Wrap it in a cgo.Handle so we can pass an opaque pointer to Rust.
	handle := cgo.NewHandle(h)
	defer handle.Delete() // Delete when Rust will no longer call back with this ctx.

	// “Store and trigger” example:
	C.set_host_function(C.getGoHostFunction(), unsafe.Pointer(handle))

	C.test_run()
}

//export goHostFunction
func goHostFunction(ctx unsafe.Pointer) {
	// Recover the Go handler from the handle
	// h := cgo.Handle(ctx).Value().(Handler)

	fmt.Println("goHostFunction called from Rust")
}
