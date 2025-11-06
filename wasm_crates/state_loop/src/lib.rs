#![no_std]

use algokit::{GlobalVar, get_global_uint, get_global_var_uint, set_global_uint};

const KEY: &[u8] = b"counter";

#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    let app_id = get_global_var_uint(GlobalVar::AppID);

   
    while get_global_uint(app_id, KEY) < 46 {
        let mut value = get_global_uint(app_id, KEY);
        value += 1;
        set_global_uint(app_id, KEY, value);
    }

    0
}
