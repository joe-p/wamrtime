use std::thread;

use crossbeam_channel::{Receiver, Sender, bounded};

use crate::{
    program::Program,
    runtime::{HostGasCheckFn, WamrHostFunction, WamrRuntime},
    wamr,
};

pub struct InitMessage {
    program_bytes: Vec<u8>,
    program_sender: Sender<Program>,
}

pub struct CallMessage {
    program_receiver: Receiver<Program>,
    result_sender: Sender<u64>,
}

pub struct RuntimeThread {
    init_message_sender: crossbeam_channel::Sender<InitMessage>,
    call_message_sender: crossbeam_channel::Sender<CallMessage>,
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
        let (init_sender, init_receiver) = bounded::<InitMessage>(256);
        let (call_sender, call_receiver) = bounded::<CallMessage>(256);

        // Spawn initialization thread
        thread::spawn(move || {
            let _runtime = WamrRuntime::new(gas_check_fn, host_fns.clone(), runtime_heap_size)
                .expect("Failed to create WamrRuntime");

            while let Ok(init_message) = init_receiver.recv() {
                let err_buf = &mut [0i8; crate::ERROR_BUFFER_SIZE];
                let compiler = crate::compiler::Compiler::new();

                let mut compiled_bytes = compiler
                    .compile_wasm(&mut init_message.program_bytes.clone())
                    .expect("Failed to compile WASM bytes");

                let program = Program::new(
                    &mut compiled_bytes,
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

                init_message
                    .program_sender
                    .send(program)
                    .expect("Failed to send initialized program");
            }
        });

        // Spawn calling thread
        thread::spawn(move || {
            unsafe { wamr::wasm_runtime_init_thread_env() };
            while let Ok(call_message) = call_receiver.recv() {
                let program = call_message
                    .program_receiver
                    .recv()
                    .expect("Failed to receive program for calling");

                let result = program.call().expect("Failed to call program");

                call_message
                    .result_sender
                    .send(result)
                    .expect("Failed to send program result");
            }
            unsafe { wamr::wasm_runtime_destroy_thread_env() };
        });

        Self {
            init_message_sender: init_sender,
            call_message_sender: call_sender,
        }
    }

    pub fn init_program(&self, program_bytes: Vec<u8>) -> Receiver<Program> {
        let (sender, receiver) = crossbeam_channel::bounded::<Program>(1);

        self.init_message_sender
            .send(InitMessage {
                program_bytes,
                program_sender: sender,
            })
            .expect("Failed to send program bytes");

        receiver
    }

    pub fn call_intialized_program(&self, program_receiver: Receiver<Program>) -> u64 {
        let (result_sender, result_receiver) = bounded::<u64>(1);

        self.call_message_sender
            .send(CallMessage {
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
