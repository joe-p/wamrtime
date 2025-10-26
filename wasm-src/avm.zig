const std = @import("std");

extern fn avm_get_global_uint(_app: u64, key_ptr: u32, len: u32) u64;
extern fn avm_set_global_uint(_app: u64, key_ptr: u32, len: u32, value: u64) void;

// TestBlankKey:
// app_global_get
// int 0
// ==
// assert
//
// byte ""
// int 7
// app_global_put
//
// byte ""
// app_global_get
// int 7
// ==
export fn program() u64 {
    const app_id: u64 = 42;
    const key_slice = "";
    const key_ptr: u32 = @intFromPtr(key_slice.ptr);

    var value = avm_get_global_uint(app_id, key_ptr, key_slice.len);
    if (value != 0) {
        @panic("expected initial global uint to be 0");
    }

    avm_set_global_uint(app_id, key_ptr, key_slice.len, 7);

    value = avm_get_global_uint(app_id, key_ptr, key_slice.len);
    if (value != 7) {
        @panic("expected global uint to be 7 after setting it");
    }

    return 0;
}
