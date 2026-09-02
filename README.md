# warmtime

wamrtime is an opinionated wrapper around [wasm-micro-runtime (WAMR)](https://github.com/wasm-micro-runtime/wasm-micro-runtime). It was created to determine the feasability of using WASM to evaluate smart contracts on Algorand. The full integration with E2E tests can be seen in [this go-algorand branch](https://github.com/algorand/go-algorand/compare/master...joe-p:go-algorand:spike/wamrtime).

## Goals

- *Execution Speed*: wamrtime is designed to execute contracts as fast as possible. Most of the end-to-end execution time is during the cold startup of the WASM module.
- *Memory footprint*: Since wamrtime is designed to run smart contracts in a blockchain node, each WASM execution has a limited footprint
- *Memory safety*: wamrtime itself reduces the usage of `unsafe` where possible. WASM program executions always have a fresh environment, so the linear memory/stack do not leak between executions

## Architecture

wamrtime executes raw WebAssembly with WAMR's fast interpreter. A dedicated runtime thread owns WAMR and processes program initialization and calls in order:

```text
embedding application
    |
    v
RuntimeThread channel
    |
    +-- InitializeProgram: load WASM -> instantiate -> create execution environment
    |
    +-- CallProgram: invoke exported `program` -> host functions in `env`
```

### Core runtime

The `wamrtime` Rust crate owns the WAMR runtime and exposes small wrappers for its lifecycle. `WamrRuntime` initializes WAMR with an embedding-provided fixed-size allocation pool and registers the host functions supplied by the embedding application, plus `host_malloc` and `host_free` for allocation in a module's linear memory. WAMR's C API is isolated in `unsafe_wamr_fns`, keeping unsafe code at the FFI boundary.

The WAMR build enables the interpreter, fast interpreter, hardware memory and stack bounds checks, reference types, and WAMR instruction metering. It explicitly disables AOT compilation and JIT because their initialization costs do not suit this use case. Each execution environment receives an instruction count limit from its `ProgramConfig`.

### Execution pipeline

A `Program` loads WASM bytes, constrains the module's maximum linear-memory pages, instantiates it with configured stack and managed-heap sizes, creates an execution environment, and resolves the required `program` export. Its destructor destroys the execution environment, instance, and module in reverse order, so each call receives a fresh WASM instance.
`RuntimeThread` serializes all WAMR work onto the thread that created the runtime. Initialization returns a channel receiver for the resulting `Program`; a later call message waits for that receiver and invokes the program. This keeps WAMR objects on their owning thread while allowing callers to queue initialization independently from execution.

` RuntimeThread` is needed to ensure compatibility with Go. There are two main problems with Go interopability without `RuntimeThread`:

1. Go does not allow for configurable stack sizes. If a go routine calls into another language via FFI that language may expect a larger stack size, but has no way to actually increase stack size. This causes a panic when trying to create a WASM execution environment with a stack larger than Go's default stack size

1. WAMR's execution environments are tied to threads. There is no way to control which thread is calling into the Rust code via FFI, thus locking the runtime on the rust-side to one thread removes the need for the Go callsite to think about threads

### AVM embedding example

`wamrtime_avm` demonstrates an Algorand AVM-facing embedding. It declares AVM operations for global state reads, writes, and global values, converts their types to WAMR native-function signatures, and registers them in the `env` module. The embedding initializes the function implementations with `avm_init`, sets the per-evaluation context with `avm_set_ctx`, and calls raw WASM bytes through `avm_call_program`.

The `wasm_crates/` workspace members are small `wasm32-unknown-unknown` contracts that exercise this host boundary. Production contracts must import the registered `env` functions and export `program`.

## End Result

wamrtime execution was considerably faster than the AVM once the initial startup has passed. The main motivation for this work was to enable cryptographic functions in the AVM that were otherwise unavailable. While WASM was faster than AVM, crypto implementations in WASM were still too slow to be viable for usage in a smart contract. Performance aside, the gains of allowing developers to write contracts in WASM and give Algorand developers access to WASM tooling is compelling, but not enough to justify the engineering hours and ecosystem migration to support WASM. For now, Algorand will not support WASM execution but this repo will exist for posterity. I may come back to it every once in awhile to try out a new idea, but this is not longer being actively worked on.
