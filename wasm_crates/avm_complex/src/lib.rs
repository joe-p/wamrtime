#![no_std]

use algokit::{
    GlobalVar, get_global_bytes, get_global_uint, get_global_var_uint, set_global_bytes,
    set_global_uint,
};

const KEY: &[u8] = b"foo";
const VALUE_BYTES: &[u8] = b"Hello AVM!";

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

    set_global_bytes(app_id, KEY, VALUE_BYTES);

    let retrieved_value = get_global_bytes(app_id, KEY);

    if retrieved_value.as_slice() != VALUE_BYTES {
        algokit::panic();
    }

    0
}
