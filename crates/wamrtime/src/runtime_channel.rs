use std::thread;

use crossbeam_channel::bounded;

use crate::{
    program::Program,
    runtime::{HostGasCheckFn, WamrHostFunction, WamrRuntime},
};

pub struct RuntimeChannel {
    pub program_sender: crossbeam_channel::Sender<Vec<u8>>,
}

impl RuntimeChannel {
    pub fn new(gas_check_fn: HostGasCheckFn, host_fns: Vec<WamrHostFunction>) -> Self {
        let (sender, receiver) = bounded::<Vec<u8>>(1);

        thread::spawn(move || {
            let runtime = WamrRuntime::new(gas_check_fn, host_fns.clone())
                .expect("Failed to create WamrRuntime");

            while let Ok(program_bytes) = receiver.recv() {
                let err_buf = &mut [0i8; crate::ERROR_BUFFER_SIZE];
                let program = Program::new(
                    &mut program_bytes.clone(),
                    err_buf,
                    crate::STACK_SIZE as usize,
                    &runtime,
                )
                .expect("Failed to create Program");

                if err_buf[0] != 0 {
                    let err_msg = unsafe { std::ffi::CStr::from_ptr(err_buf.as_ptr()) }
                        .to_string_lossy()
                        .into_owned();
                    panic!("Error buffer not empty: {}", err_msg);
                }

                program.call().expect("Failed to call program");
            }
        });
        Self {
            program_sender: sender,
        }
    }
}
