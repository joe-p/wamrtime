#[allow(warnings)]
pub mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod compiler;
pub mod program;
pub mod runtime;
pub mod runtime_channel;
mod unsafe_wamr_fns;

pub type Result<T> = color_eyre::Result<T>;

pub const ERROR_BUFFER_SIZE: usize = 128;

const KB: usize = 1024;

/// The size of the heap that each WAMR program gets
const MAX_PROGRAM_SIZE: usize = 64 * KB;

/// The maximum number of WAMR programs that can be called per outer call
const MAX_WAMR_PROGRAM_REFERENCES: usize = 256;

/// The total runtime heap size needed to support all WAMR possible programs
const RUNTIME_HEAP_SIZE: usize = MAX_PROGRAM_SIZE * MAX_WAMR_PROGRAM_REFERENCES;

const STACK_SIZE: u32 = 64 * KB as u32;
