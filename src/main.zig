const std = @import("std");
const c = @cImport({
    @cInclude("wasm_export.h");
    @cInclude("platform_common.h");
    @cInclude("time.h");
});

const ERROR_SIZE = 128;

pub const ProgramReturn = struct {
    return_value: u64,
    error_message: [ERROR_SIZE]u8,

    pub fn init() ProgramReturn {
        return ProgramReturn{
            .return_value = 0,
            .error_message = std.mem.zeroes([ERROR_SIZE]u8),
        };
    }
};

pub fn main() void {
    var result = ProgramReturn.init();
    var heap_buf: [512 * 1024]u8 = undefined;
    const heap_size = heap_buf.len;

    // Initialize runtime args
    var init_args = std.mem.zeroes(c.RuntimeInitArgs);
    init_args.mem_alloc_type = c.Alloc_With_Pool;
    init_args.mem_alloc_option.pool.heap_buf = @ptrCast(&heap_buf);

    init_args.mem_alloc_option.pool.heap_size = @intCast(heap_size);
    init_args.running_mode = c.Mode_Interp;
    init_args.native_module_name = "avm";

    if (!c.wasm_runtime_full_init(&init_args)) {
        _ = std.fmt.bufPrint(&result.error_message, "Init runtime environment failed.", .{}) catch {};
        return;
    }
    defer c.wasm_runtime_destroy();

    return;
}
