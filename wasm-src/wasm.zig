extern fn hello() void;

export fn program() u64 {
    hello();
    return 1;
}
