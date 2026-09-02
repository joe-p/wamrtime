#[allow(warnings)]
pub mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod program;
pub mod runtime;
pub mod runtime_thread;
mod unsafe_wamr_fns;

pub type Result<T> = color_eyre::Result<T>;

pub const ERROR_BUFFER_SIZE: usize = 128;
