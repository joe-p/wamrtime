#![no_std]
use algokit::{ActiveAvm, GlobalVar, program_entry};

const KEY: &[u8] = b"foo";

#[program_entry]
fn blank_key(avm: ActiveAvm) -> Result<(), ()> {
    let app_id = avm.get_global_var_uint(GlobalVar::AppID);

    let mut value = avm.get_global_uint(app_id, KEY);
    if value != 0 {
        return Err(());
    }

    avm.set_global_uint(app_id, KEY, 7);

    value = avm.get_global_uint(app_id, KEY);

    if value != 7 {
        return Err(());
    }

    avm.set_global_uint(app_id, KEY, 0);

    Ok(())
}
