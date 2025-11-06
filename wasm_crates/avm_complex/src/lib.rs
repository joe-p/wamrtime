#![no_std]

extern crate alloc;

use algokit::{GlobalBytes, GlobalUint, avm_panic};

const GLOBAL_UINT: GlobalUint = GlobalUint::new(b"foo");
const GLOBAL_BYTES: GlobalBytes = GlobalBytes::new(b"foo");

#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    let val = GLOBAL_UINT.get();
    if val != 0 {
        avm_panic();
    }

    GLOBAL_UINT.set(7);
    let new_val = GLOBAL_UINT.get();
    if new_val != 7 {
        avm_panic();
    }

    GLOBAL_BYTES.write(b"hello AVM!");

    let buf = &mut [0u8; 128];
    let retrieved_value = GLOBAL_BYTES.read(buf);

    if retrieved_value != b"hello AVM!".as_slice() {
        avm_panic();
    }

    GLOBAL_UINT.set(0);
    GLOBAL_BYTES.write(&[]);

    0
}
