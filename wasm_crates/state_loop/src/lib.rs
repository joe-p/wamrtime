#![no_std]

use algokit::{ActiveAvm, GlobalVar, program_entry};

const KEY: &[u8] = b"counter";

#[program_entry]
fn state_loop(avm: ActiveAvm) -> u64 {
    let app_id = avm.get_global_var_uint(GlobalVar::AppID);

    while avm.get_global_uint(app_id, KEY) < 46 {
        let mut value = avm.get_global_uint(app_id, KEY);
        value += 1;
        avm.set_global_uint(app_id, KEY, value);
    }

    0
}
