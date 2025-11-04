#![no_std]

extern crate alloc;

use algokit::{GlobalBytes, GlobalUint};
use alloc::vec::Vec;

const GLOBAL_UINT_VALUE: GlobalUint = GlobalUint::new(b"foo");
const GLOBAL_BYTES_VALUE: GlobalBytes<Vec<u8>> = GlobalBytes::new(b"foo");

#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    let mut value = GLOBAL_UINT_VALUE.get();
    if value != 0 {
        algokit::panic();
    }

    GLOBAL_UINT_VALUE.set(7);

    value = GLOBAL_UINT_VALUE.get();

    if value != 7 {
        algokit::panic();
    }

    GLOBAL_UINT_VALUE.set(0);

    GLOBAL_BYTES_VALUE.set_raw_bytes(b"hello AVM!");

    let retrieved_value = GLOBAL_BYTES_VALUE.get();

    if retrieved_value.as_slice() != b"hello AVM!" {
        algokit::panic();
    }

    0
}
