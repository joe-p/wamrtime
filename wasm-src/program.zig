extern fn call_host_function() void;
extern fn ret_1337() u64;

export fn program() u64 {
    call_host_function();
    return 1337;
}
