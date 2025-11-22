use std::thread;

use crossbeam_channel::{Receiver, Sender, bounded};

use crate::{
    program::Program,
    runtime::{WamrHostFunction, WamrRuntime},
};

pub enum ProgramMessage {
    NewProgram {
        program_bytes: Vec<u8>,
        program_sender: Sender<Result<Program, String>>,
    },
    CallProgram {
        program_receiver: Receiver<Result<Program, String>>,
        result_sender: Sender<Result<u64, String>>,
    },
}

pub struct RuntimeThread {
    program_message_sender: crossbeam_channel::Sender<ProgramMessage>,
}

impl RuntimeThread {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host_functions: Vec<WamrHostFunction>,
        runtime_heap_size: usize,
        stack_size: u32,
        app_heap_size: usize,
        max_pages: u32,
        message_buffer_size: usize,
        instruction_count_limit: i32,
    ) -> Self {
        let (prog_sender, prog_receiver) = bounded::<ProgramMessage>(message_buffer_size);

        thread::spawn(move || {
            let _runtime = WamrRuntime::new(host_functions, runtime_heap_size)
                .expect("Failed to create WamrRuntime");
            let error_buf = &mut [0i8; crate::ERROR_BUFFER_SIZE];

            while let Ok(program_message) = prog_receiver.recv() {
                match program_message {
                    ProgramMessage::NewProgram {
                        mut program_bytes,
                        program_sender,
                    } => {
                        let result = match Program::new(
                            &mut program_bytes,
                            error_buf,
                            app_heap_size,
                            stack_size,
                            max_pages,
                            instruction_count_limit,
                        ) {
                            Ok(program) => {
                                if error_buf[0] != 0 {
                                    let err_msg =
                                        unsafe { std::ffi::CStr::from_ptr(error_buf.as_ptr()) }
                                            .to_string_lossy()
                                            .into_owned();
                                    error_buf.fill(0);
                                    Err(format!("Error buffer not empty: {}", err_msg))
                                } else {
                                    Ok(program)
                                }
                            }
                            Err(e) => Err(format!("Failed to create Program: {}", e)),
                        };

                        if program_sender.send(result).is_err() {
                            // Receiver dropped, nothing we can do
                            break;
                        }
                    }
                    ProgramMessage::CallProgram {
                        program_receiver,
                        result_sender,
                    } => {
                        let result = match program_receiver.recv() {
                            Ok(Ok(program)) => program
                                .call()
                                .map_err(|e| format!("Failed to call program: {}", e)),
                            Ok(Err(e)) => Err(e),
                            Err(e) => Err(format!("Failed to receive program for calling: {}", e)),
                        };

                        if result_sender.send(result).is_err() {
                            // Receiver dropped, nothing we can do
                            break;
                        }
                    }
                }
            }
        });

        Self {
            program_message_sender: prog_sender,
        }
    }

    pub fn init_program(
        &self,
        program_bytes: Vec<u8>,
    ) -> Result<Receiver<Result<Program, String>>, String> {
        let (sender, receiver) = crossbeam_channel::bounded::<Result<Program, String>>(1);

        self.program_message_sender
            .send(ProgramMessage::NewProgram {
                program_bytes,
                program_sender: sender,
            })
            .map_err(|e| format!("Failed to send program bytes: {}", e))?;

        Ok(receiver)
    }

    pub fn call_intialized_program(
        &self,
        program_receiver: Receiver<Result<Program, String>>,
    ) -> Result<u64, String> {
        let (result_sender, result_receiver) = bounded::<Result<u64, String>>(1);

        self.program_message_sender
            .send(ProgramMessage::CallProgram {
                program_receiver,
                result_sender,
            })
            .map_err(|e| format!("Failed to send call program message: {}", e))?;

        result_receiver
            .recv()
            .map_err(|e| format!("Failed to receive program result: {}", e))?
    }

    pub fn call_program(&self, program_bytes: Vec<u8>) -> Result<u64, String> {
        let program_receiver = self.init_program(program_bytes)?;
        self.call_intialized_program(program_receiver)
    }
}
