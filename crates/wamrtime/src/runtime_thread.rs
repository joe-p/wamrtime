use std::thread;

use crossbeam_channel::{Receiver, Sender, bounded};

use crate::{
    program::Program,
    runtime::{HostGasCheckFn, WamrHostFunction, WamrRuntime},
};

pub enum ProgramMessage {
    NewProgram {
        program_bytes: Vec<u8>,
        program_sender: Sender<Program>,
    },
    CallProgram {
        program_receiver: Receiver<Program>,
        result_sender: Sender<u64>,
    },
}

pub struct RuntimeThread {
    program_message_sender: crossbeam_channel::Sender<ProgramMessage>,
}

impl RuntimeThread {
    pub fn new(
        gas_check_fn: HostGasCheckFn,
        host_fns: Vec<WamrHostFunction>,
        runtime_heap_size: usize,
        stack_size: u32,
        app_heap_size: usize,
        max_pages: u32,
    ) -> Self {
        // We bound it with 512 because we can have 256 programs total, with each program having
        // two messages (init and call). It is unlikely we'll ever get close to this limit, but it
        // seems preferable to an unbounded channel.
        let (prog_sender, prog_receiver) = bounded::<ProgramMessage>(512);

        thread::spawn(move || {
            let _runtime = WamrRuntime::new(gas_check_fn, host_fns.clone(), runtime_heap_size)
                .expect("Failed to create WamrRuntime");

            while let Ok(program_message) = prog_receiver.recv() {
                match program_message {
                    ProgramMessage::NewProgram {
                        program_bytes,
                        program_sender,
                    } => {
                        let err_buf = &mut [0i8; crate::ERROR_BUFFER_SIZE];
                        let program = Program::new(
                            &mut program_bytes.clone(),
                            err_buf,
                            app_heap_size,
                            stack_size,
                            max_pages,
                        )
                        .expect("Failed to create Program");

                        if err_buf[0] != 0 {
                            let err_msg = unsafe { std::ffi::CStr::from_ptr(err_buf.as_ptr()) }
                                .to_string_lossy()
                                .into_owned();
                            panic!("Error buffer not empty: {}", err_msg);
                        }

                        program_sender
                            .send(program)
                            .expect("Failed to send initialized program");
                    }
                    ProgramMessage::CallProgram {
                        program_receiver,
                        result_sender,
                    } => {
                        let program = program_receiver
                            .recv()
                            .expect("Failed to receive program for calling");

                        let result = program.call().expect("Failed to call program");

                        result_sender
                            .send(result)
                            .expect("Failed to send program result");
                    }
                }
            }
        });

        Self {
            program_message_sender: prog_sender,
        }
    }

    pub fn init_program(&self, program_bytes: Vec<u8>) -> Receiver<Program> {
        let (sender, receiver) = crossbeam_channel::bounded::<Program>(1);

        self.program_message_sender
            .send(ProgramMessage::NewProgram {
                program_bytes,
                program_sender: sender,
            })
            .expect("Failed to send program bytes");

        receiver
    }

    pub fn call_intialized_program(&self, program_receiver: Receiver<Program>) -> u64 {
        let (result_sender, result_receiver) = bounded::<u64>(1);

        self.program_message_sender
            .send(ProgramMessage::CallProgram {
                program_receiver,
                result_sender,
            })
            .expect("Failed to send call program message");

        result_receiver
            .recv()
            .expect("Failed to receive program result")
    }

    pub fn call_program(&self, program_bytes: Vec<u8>) -> u64 {
        let program_receiver = self.init_program(program_bytes);
        self.call_intialized_program(program_receiver)
    }
}
