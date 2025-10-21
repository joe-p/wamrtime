const std = @import("std");
const c = @cImport({
    @cInclude("wasm_export.h");
    @cInclude("platform_common.h");
    @cInclude("time.h");
});

const ERROR_SIZE = 128;
const ITERS = 1;

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

pub fn hello() void {
    std.debug.print("Hello from Zig WAMR host!\n", .{});
}

const native_symbol = [1]c.NativeSymbol{c.NativeSymbol{ .symbol = "hello", .func_ptr = @constCast(&hello), .signature = "()" }};

pub fn main() !void {
    var result = ProgramReturn.init();
    var heap_buf: [512 * 1024]u8 = undefined;
    const heap_size = heap_buf.len;

    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const aot_file = try std.fs.cwd().readFileAlloc(allocator, "zig-out/bin/program.aot", 4096);
    defer allocator.free(aot_file);

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

    if (!c.wasm_runtime_register_natives("env", @constCast(&native_symbol), 1)) {
        _ = std.fmt.bufPrint(&result.error_message, "Register native symbols failed.", .{}) catch {};
        std.debug.print("Register native symbols failed.\n", .{});
        return;
    }

    const package_type = c.get_package_type(aot_file.ptr, @intCast(aot_file.len));
    std.debug.print("Package type for file of size {d}: {d}\n", .{ aot_file.len, package_type });

    const stack_size: u32 = 8092;
    var error_buf: [ERROR_SIZE]u8 = undefined;

    var start: c.struct_timespec = undefined;
    var end: c.struct_timespec = undefined;
    _ = c.clock_gettime(c.CLOCK_REALTIME, &start);

    const module = c.wasm_runtime_load(aot_file.ptr, @intCast(aot_file.len), &error_buf, error_buf.len);
    if (module == null) {
        _ = std.fmt.bufPrint(&result.error_message, "{s}", .{error_buf}) catch {};
        std.debug.print("Error loading module: {s}\n", .{error_buf});

        return;
    }
    defer c.wasm_runtime_unload(module);

    const module_inst = c.wasm_runtime_instantiate(module, stack_size, @intCast(heap_size), &error_buf, error_buf.len);
    if (module_inst == null) {
        _ = std.fmt.bufPrint(&result.error_message, "{s}", .{error_buf}) catch {};
        return;
    }
    defer c.wasm_runtime_deinstantiate(module_inst);

    const exec_env = c.wasm_runtime_create_exec_env(module_inst, stack_size);
    if (exec_env == null) {
        _ = std.fmt.bufPrint(&result.error_message, "Create wasm execution environment failed.", .{}) catch {};
        return;
    }
    defer c.wasm_runtime_destroy_exec_env(exec_env);

    const program_func = c.wasm_runtime_lookup_function(module_inst, "program");
    if (program_func == null) {
        _ = std.fmt.bufPrint(&result.error_message, "The program wasm function is not found.", .{}) catch {};
        return;
    }

    _ = c.clock_gettime(c.CLOCK_REALTIME, &end);
    const elapsed_ns = end.tv_nsec - start.tv_nsec;
    std.debug.print("Load to lookup time: {d} nanoseconds ({d:.6} ms)\n", .{ elapsed_ns, @as(f64, @floatFromInt(elapsed_ns)) / 1e6 });

    // Measure first call time separately
    _ = c.clock_gettime(c.CLOCK_REALTIME, &start);

    var results = [_]c.wasm_val_t{c.wasm_val_t{
        .kind = c.WASM_I64,
        .of = .{ .i64 = 0 },
    }};

    if (!c.wasm_runtime_call_wasm_a(exec_env, program_func, 1, &results, 0, null)) {
        const exception = c.wasm_runtime_get_exception(module_inst);
        _ = std.fmt.bufPrint(&result.error_message, "{s}", .{exception}) catch {};
        return;
    }

    result.return_value = @intCast(results[0].of.i64);

    _ = c.clock_gettime(c.CLOCK_REALTIME, &end);
    const first_call_time = end.tv_nsec - start.tv_nsec;
    std.debug.print("First call time: {d} nanoseconds ({d:.6} ms)\n", .{ first_call_time, @as(f64, @floatFromInt(first_call_time)) / 1e6 });

    // Measure subsequent calls time
    _ = c.clock_gettime(c.CLOCK_REALTIME, &start);

    var i: i32 = 0;
    while (i < ITERS) : (i += 1) {
        results = [_]c.wasm_val_t{c.wasm_val_t{
            .kind = c.WASM_I64,
            .of = .{ .i64 = 0 },
        }};

        if (!c.wasm_runtime_call_wasm_a(exec_env, program_func, 1, &results, 0, null)) {
            const exception = c.wasm_runtime_get_exception(module_inst);
            _ = std.fmt.bufPrint(&result.error_message, "{s}", .{exception}) catch {};
            return;
        }

        result.return_value = @intCast(results[0].of.i64);
    }

    _ = c.clock_gettime(c.CLOCK_REALTIME, &end);

    const time_per_op = @divTrunc(end.tv_nsec - start.tv_nsec, @as(c_long, ITERS));
    std.debug.print("Subsequent calls time: {d} ns/iter ({d:.6} ms/{d} iters)\n", .{ time_per_op, @as(f64, @floatFromInt(time_per_op)) / 1e6, ITERS });

    return;
}
