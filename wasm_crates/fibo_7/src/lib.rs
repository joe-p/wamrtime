fn fibo(n: u64) -> u64 {
    if n <= 1 { n } else { fibo(n - 1) + fibo(n - 2) }
}

#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    fibo(7)
}
