#[allow(warnings)]
pub mod wamr {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod compiler;
pub mod program;
pub mod runtime;
mod unsafe_wamr_fns;

pub type Result<T> = color_eyre::Result<T>;

pub const ERROR_BUFFER_SIZE: usize = 128;
