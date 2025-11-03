#![no_std]

use algokit::{GlobalVar, get_global_uint, get_global_var_uint, set_global_uint};

const KEY: &[u8] = b"foo";

#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    let app_id = get_global_var_uint(GlobalVar::AppID);

    let mut value = get_global_uint(app_id, KEY);
    if value != 0 {
        algokit::panic();
    }

    set_global_uint(app_id, KEY, 7);

    value = get_global_uint(app_id, KEY);

    if value != 7 {
        algokit::panic();
    }

    set_global_uint(app_id, KEY, 0);

    0
}
