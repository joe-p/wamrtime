use crate::ERROR_BUFFER_SIZE;
use crate::program::Program;
use crate::runtime::WamrRuntime;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

const MAX_PROGRAMS: usize = 256;

type ProgramArray = [Option<Program>; MAX_PROGRAMS];

struct SharedEvaluatorState {
    programs: [ProgramArray; 3],
    program_lens: [usize; 3],
}

pub struct Evaluator<'runtime> {
    state: Arc<Mutex<SharedEvaluatorState>>,
    current_idx: usize,
    init_thread: Option<thread::JoinHandle<Result<(), String>>>,
    _runtime: &'runtime WamrRuntime,
}

impl Drop for Evaluator<'_> {
    fn drop(&mut self) {
        if let Some(thread) = self.init_thread.take() {
            thread.join().ok();
        }
    }
}

impl<'runtime> Evaluator<'runtime> {
    pub fn new(runtime: &'runtime WamrRuntime) -> Self {
        const INIT: Option<Program> = None;
        Evaluator {
            state: Arc::new(Mutex::new(SharedEvaluatorState {
                programs: [
                    [INIT; MAX_PROGRAMS],
                    [INIT; MAX_PROGRAMS],
                    [INIT; MAX_PROGRAMS],
                ],
                program_lens: [0, 0, 0],
            })),
            current_idx: 0,
            init_thread: None,
            _runtime: runtime,
        }
    }

    fn init_next(
        state: Arc<Mutex<SharedEvaluatorState>>,
        current_idx: usize,
        aot_bytes_vec: Vec<Vec<u8>>,
    ) -> Result<(), String> {
        let prev_idx = (current_idx + 2) % 3;
        let next_idx = (current_idx + 1) % 3;

        {
            let mut state_guard = state.lock().unwrap();
            for idx in 0..state_guard.program_lens[prev_idx] {
                state_guard.programs[prev_idx][idx] = None;
            }
        }

        let len = aot_bytes_vec.len();
        let mut new_programs = Vec::new();
        for mut aot_bytes in aot_bytes_vec {
            let mut err_buf = [0i8; ERROR_BUFFER_SIZE];
            let program = Program::new(&mut aot_bytes, &mut err_buf);
            new_programs.push(program);
        }

        {
            let mut state_guard = state.lock().unwrap();
            for (idx, program) in new_programs.into_iter().enumerate() {
                state_guard.programs[next_idx][idx] = Some(program);
            }
            state_guard.program_lens[next_idx] = len;
        }

        Ok(())
    }

    // NOTE: We need ownership of aot_bytes_vec because WAMR may modify it. We'll let the
    // caller worry about whether they need to clone it, but in most real-world cases they won't
    pub fn next_round(&mut self, aot_bytes_vec: Vec<Vec<u8>>) -> Result<(), String> {
        if let Some(thread) = self.init_thread.take() {
            thread
                .join()
                .map_err(|_| "Thread join failed".to_string())??;
        }

        self.current_idx = (self.current_idx + 1) % 3;

        let state = Arc::clone(&self.state);
        let current_idx = self.current_idx;

        self.init_thread = Some(thread::spawn(move || {
            Self::init_next(state, current_idx, aot_bytes_vec)
        }));

        // TODO: Get programs, release guard, then execute to allow better parallelism with init
        let state_guard = self.state.lock().unwrap();
        for idx in 0..state_guard.program_lens[self.current_idx] {
            if let Some(program) = &state_guard.programs[self.current_idx][idx] {
                // let start = Instant::now();
                program.call();
                // let duration = start.elapsed();
                // println!(
                //     "Program {} executed in {} ns with return value {}",
                //     idx,
                //     duration.as_nanos(),
                //     res
                // );
            }
        }

        Ok(())
    }

    pub fn wait_for_init(&mut self) -> Result<(), String> {
        if let Some(thread) = self.init_thread.take() {
            thread
                .join()
                .map_err(|_| "Thread join failed".to_string())??;
        }
        Ok(())
    }

    // TODO: Completely rework the evaluator so that it has a hashmap of aot hash -> Program
    pub fn _test_only_call_next(&self, program_idx: usize) -> Result<u64, String> {
        let state_guard = self.state.lock().unwrap();

        if let Some(program) = &state_guard.programs[(self.current_idx + 1) % 3][program_idx] {
            let res = program.call();
            Ok(res)
        } else {
            Err(format!("Program at index {} not found", program_idx))
        }
    }
}
