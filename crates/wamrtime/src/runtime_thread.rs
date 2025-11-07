use std::thread;

use crossbeam_channel::bounded;

use crate::{
    program::Program,
    runtime::{HostGasCheckFn, WamrHostFunction, WamrRuntime},
};

pub struct RuntimeThread {
    pub program_sender: crossbeam_channel::Sender<Vec<u8>>,
    pub result_receiver: crossbeam_channel::Receiver<u64>,
}

impl RuntimeThread {
    pub fn new(gas_check_fn: HostGasCheckFn, host_fns: Vec<WamrHostFunction>) -> Self {
        let (prog_sender, prog_receiver) = bounded::<Vec<u8>>(1);
        let (result_sender, result_receiver) = bounded::<u64>(1);

        thread::spawn(move || {
            let runtime = WamrRuntime::new(gas_check_fn, host_fns.clone())
                .expect("Failed to create WamrRuntime");

            while let Ok(program_bytes) = prog_receiver.recv() {
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

                let result = program.call().expect("Failed to call program");
                result_sender
                    .send(result)
                    .expect("Failed to send program result");
            }
        });
        Self {
            program_sender: prog_sender,
            result_receiver,
        }
    }

    pub fn call_program(&self, program_bytes: Vec<u8>) -> u64 {
        self.program_sender
            .send(program_bytes)
            .expect("Failed to send program bytes");
        self.result_receiver
            .recv()
            .expect("Failed to receive program result")
    }
}
