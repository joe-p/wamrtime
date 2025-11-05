#![no_std]

extern crate alloc;

use algokit::{GlobalBytes, GlobalUint, avm_panic};

const GLOBAL_UINT_VALUE: GlobalUint = GlobalUint::new(b"foo");
const GLOBAL_BYTES_VALUE: GlobalBytes = GlobalBytes::new(b"foo");

#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    let mut value = GLOBAL_UINT_VALUE.get();
    if value != 0 {
        avm_panic();
    }

    GLOBAL_UINT_VALUE.set(7);

    value = GLOBAL_UINT_VALUE.get();

    if value != 7 {
        avm_panic();
    }

    GLOBAL_UINT_VALUE.set(0);

    GLOBAL_BYTES_VALUE.write(b"hello AVM!");

    let buf = &mut [0u8; 128];
    let retrieved_value = GLOBAL_BYTES_VALUE.read(buf);

    if retrieved_value != b"hello AVM!".as_slice() {
        avm_panic();
    }

    0
}
