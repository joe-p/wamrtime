extern fn hello() void;
extern fn ret_1337() u64;

export fn program() u64 {
    return ret_1337();
}
