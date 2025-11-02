use crate::program::Program;
use crate::runtime::WamrRuntime;
use crate::{APP_HEAP_SIZE, ERROR_BUFFER_SIZE, Result};
use color_eyre::eyre::eyre;
use std::sync::{Arc, Mutex};
use std::thread;

const MAX_PROGRAMS: usize = 256;

type ProgramArray = [Option<Program>; MAX_PROGRAMS];

struct SharedEvaluatorState {
    programs: [ProgramArray; 3],
    program_lens: [usize; 3],
}

pub struct Evaluator<'runtime> {
    state: Arc<Mutex<SharedEvaluatorState>>,
    current_idx: usize,
    init_thread: Option<thread::JoinHandle<Result<()>>>,
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
        mut aot_bytes_vec: Vec<Vec<u8>>,
    ) -> Result<()> {
        let prev_idx = (current_idx + 2) % 3;
        let next_idx = (current_idx + 1) % 3;

        {
            let mut state_guard = state
                .lock()
                .map_err(|err| eyre!("Failed to lock evaluator state: {err}"))?;
            for idx in 0..state_guard.program_lens[prev_idx] {
                state_guard.programs[prev_idx][idx] = None;
            }
            state_guard.program_lens[prev_idx] = 0;
        }

        let len = aot_bytes_vec.len();
        if len > MAX_PROGRAMS {
            return Err(eyre!(
                "AOT byte vector exceeds MAX_PROGRAMS ({MAX_PROGRAMS}), got {len}"
            ));
        }

        const INIT: Option<Program> = None;
        let mut new_programs: [Option<Program>; MAX_PROGRAMS] = [INIT; MAX_PROGRAMS];

        for (i, aot_bytes) in aot_bytes_vec.iter_mut().enumerate() {
            let mut err_buf = [0i8; ERROR_BUFFER_SIZE];
            let program = Program::new(aot_bytes, &mut err_buf, APP_HEAP_SIZE)?;
            new_programs[i] = Some(program);
        }

        {
            let mut state_guard = state
                .lock()
                .map_err(|err| eyre!("Failed to lock evaluator state: {err}"))?;
            for (idx, program) in new_programs.into_iter().enumerate() {
                state_guard.programs[next_idx][idx] = program;
            }
            state_guard.program_lens[next_idx] = len;
        }

        Ok(())
    }

    // NOTE: We need ownership of aot_bytes_vec because WAMR may modify it. We'll let the
    // caller worry about whether they need to clone it, but in most real-world cases they won't
    pub fn next_round(&mut self, aot_bytes_vec: Vec<Vec<u8>>) -> Result<()> {
        if let Some(thread) = self.init_thread.take() {
            let join_result = thread.join().map_err(|_| eyre!("Thread join failed"))?;
            join_result?;
        }

        self.current_idx = (self.current_idx + 1) % 3;

        let state = Arc::clone(&self.state);
        let current_idx = self.current_idx;

        self.init_thread = Some(thread::spawn(move || {
            Self::init_next(state, current_idx, aot_bytes_vec)
        }));

        Ok(())
    }

    pub fn wait_for_init(&mut self) -> Result<()> {
        if let Some(thread) = self.init_thread.take() {
            let join_result = thread.join().map_err(|_| eyre!("Thread join failed"))?;
            join_result?;
        }
        Ok(())
    }

    pub fn call_program(&self, program_idx: usize) -> Result<u64> {
        let state_guard = self
            .state
            .lock()
            .map_err(|err| eyre!("Failed to lock evaluator state: {err}"))?;

        let program_entry = state_guard.programs[self.current_idx]
            .get(program_idx)
            .ok_or_else(|| eyre!("Program index {program_idx} out of range"))?;

        if let Some(program) = program_entry {
            program.call()
        } else {
            Err(eyre!("Program at index {program_idx} not found"))
        }
    }
}
