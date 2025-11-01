const std = @import("std");

extern fn avm_get_global_uint(app: u64, key_ptr: [*]const u8, len: u32) u64;
extern fn avm_set_global_uint(app: u64, key_ptr: [*]const u8, len: u32, value: u64) void;
extern fn avm_get_global_bytes(app: u64, key_ptr: [*]const u8, key_len: u32, dest_ptr: [*]u8, dest_len: u32) i32;
extern fn avm_set_global_bytes(app: u64, key_ptr: [*]const u8, key_len: u32, src_ptr: [*]const u8, src_len: u32) void;

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
    const key_slice = "foo";
    const key_ptr = key_slice.ptr;

    var value = avm_get_global_uint(app_id, key_ptr, key_slice.len);
    if (value != 0) {
        @panic("expected initial global uint to be 0");
    }

    avm_set_global_uint(app_id, key_ptr, key_slice.len, 7);

    value = avm_get_global_uint(app_id, key_ptr, key_slice.len);
    if (value != 7) {
        @panic("expected global uint to be 7 after setting it");
    }

    avm_set_global_uint(app_id, key_ptr, key_slice.len, 0);

    const str_value = "Hello AVM!";
    const value_ptr = str_value.ptr;
    const value_len = str_value.len;

    avm_set_global_bytes(app_id, key_ptr, key_slice.len, value_ptr, value_len);

    var retrieved_value = std.mem.zeroes([value_len]u8);
    const retrieved_value_ptr = &retrieved_value[0];
    const retrieved_value_len = retrieved_value.len;

    const ret_len = avm_get_global_bytes(app_id, key_ptr, key_slice.len, @ptrCast(retrieved_value_ptr), retrieved_value_len);

    if (ret_len != value_len) {
        @panic("expected retrieved byte length to match set byte length");
    }

    // Make sure retrieved bytes match what we set
    for (0..value_len) |i| {
        if (retrieved_value[i] != str_value[i]) {
            @panic("retrieved byte does not match set byte");
        }
    }

    return 0;
}
