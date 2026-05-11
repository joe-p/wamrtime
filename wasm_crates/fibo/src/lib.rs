use algokit::{ActiveAvm, program_entry};

fn fibo(n: u64) -> u64 {
    if n <= 1 { n } else { fibo(n - 1) + fibo(n - 2) }
}

#[program_entry]
fn fibo_program(_avm: ActiveAvm) -> u64 {
    fibo(19)
}
