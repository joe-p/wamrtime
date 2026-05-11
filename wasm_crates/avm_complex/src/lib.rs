#![no_std]

use algokit::{ActiveAvm, GlobalBytes, GlobalUint, program_entry};

#[program_entry]
fn avm_complex(avm: ActiveAvm) -> Result<(), ()> {
    let g_uint: GlobalUint = GlobalUint::new(avm, b"foo");
    let g_bytes: GlobalBytes = GlobalBytes::new(avm, b"foo");

    let val = g_uint.get();
    if val != 0 {
        return Err(());
    }

    g_uint.set(7);
    let new_val = g_uint.get();
    if new_val != 7 {
        return Err(());
    }

    g_bytes.write(b"hello AVM!");

    let buf = &mut [0u8; 128];
    let retrieved_value = g_bytes.read(buf);

    if retrieved_value != b"hello AVM!".as_slice() {
        return Err(());
    }

    g_uint.set(0);
    g_bytes.write(&[]);

    Ok(())
}
