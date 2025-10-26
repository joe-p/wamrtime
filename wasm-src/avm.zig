const std = @import("std");

extern fn host_get_global_uint(_app: u64, key_ptr: u32, len: u32) u64;
extern fn host_set_global_uint(_app: u64, key_ptr: u32, len: u32, value: u64) void;

export fn program() u64 {
    const app_id: u64 = 42;
    const key_slice = "counter";

    const key_ptr: u32 = @intFromPtr(key_slice.ptr);
    var counter = host_get_global_uint(app_id, key_ptr, key_slice.len);
    counter += 1;

    host_set_global_uint(app_id, key_ptr, key_slice.len, counter);

    return counter;
}
