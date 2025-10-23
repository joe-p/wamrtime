const std = @import("std");
const c = @cImport({
    @cInclude("wasm_export.h");
    @cInclude("platform_common.h");
    @cInclude("time.h");
});

const ERROR_SIZE = 128;
const ITERS = 1000;

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

pub fn ret_1337() u64 {
    return 1337;
}

const native_symbols = [1]c.NativeSymbol{c.NativeSymbol{ .symbol = "ret_1337", .func_ptr = @constCast(&ret_1337), .signature = "()I" }};

const HEAP_SIZE = 2 * 1024 * 1024; // 2 MB
const STACK_SIZE: u32 = 8092;

const Program = struct {
    module: *c.WASMModuleCommon,
    module_inst: *c.WASMModuleInstanceCommon,
    exec_env: *c.WASMExecEnv,
    program_func: c.wasm_function_inst_t,

    pub fn deinit(self: *Program) void {
        c.wasm_runtime_destroy_exec_env(self.exec_env);
        c.wasm_runtime_deinstantiate(self.module_inst);
        c.wasm_runtime_unload(self.module);
        self.* = undefined;
    }

    pub fn init(aot_file: []const u8, error_buf: []u8, stack_size: usize, heap_size: usize) !Program {
        const aot_len = std.math.cast(u32, aot_file.len) orelse return error.InputTooLarge;
        const err_len = std.math.cast(u32, error_buf.len) orelse return error.InputTooLarge;
        const heap_len = std.math.cast(u32, heap_size) orelse return error.InputTooLarge;
        const stack_len = std.math.cast(u32, stack_size) orelse return error.InputTooLarge;

        const mod: ?*c.WASMModuleCommon = c.wasm_runtime_load(@constCast(aot_file.ptr), aot_len, error_buf.ptr, err_len);
        if (mod == null) return error.LoadFailed;
        errdefer c.wasm_runtime_unload(mod.?);

        const inst: ?*c.WASMModuleInstanceCommon = c.wasm_runtime_instantiate(mod.?, stack_len, heap_len, error_buf.ptr, err_len);
        if (inst == null) return error.InstantiateFailed;
        errdefer c.wasm_runtime_deinstantiate(inst.?);

        const env: ?*c.WASMExecEnv = c.wasm_runtime_create_exec_env(inst.?, stack_len);
        if (env == null) return error.ExecEnvCreateFailed;
        errdefer c.wasm_runtime_destroy_exec_env(env.?);

        const name = "program\x00";
        const func: ?c.wasm_function_inst_t = c.wasm_runtime_lookup_function(inst.?, name);
        if (func == null) {
            return error.FunctionNotFound;
        }

        return Program{
            .module = mod.?,
            .module_inst = inst.?,
            .exec_env = env.?,
            .program_func = func.?,
        };
    }

    pub fn call(self: *Program) !ProgramReturn {
        var result = ProgramReturn.init();

        var call_results = [_]c.wasm_val_t{c.wasm_val_t{
            .kind = c.WASM_I64,
            .of = .{ .i64 = 0 },
        }};

        if (!c.wasm_runtime_call_wasm_a(self.exec_env, self.program_func, 1, &call_results, 0, null)) {
            const exception = c.wasm_runtime_get_exception(self.module_inst);
            _ = std.fmt.bufPrint(&result.error_message, "{s}", .{exception}) catch {};
            return result;
        }

        result.return_value = @intCast(call_results[0].of.i64);

        return result;
    }
};

const Evaluator = struct {
    heap_buf: []u8,

    pub fn init(heap_buf: []u8) !void {
        // Initialize runtime args
        var init_args = std.mem.zeroes(c.RuntimeInitArgs);
        init_args.mem_alloc_type = c.Alloc_With_Pool;
        init_args.mem_alloc_option.pool.heap_buf = heap_buf.ptr;

        init_args.mem_alloc_option.pool.heap_size = @intCast(HEAP_SIZE);
        init_args.running_mode = c.Mode_Interp;
        init_args.native_module_name = "avm";

        if (!c.wasm_runtime_full_init(&init_args)) {
            return error.InitRuntimeFailed;
        }
        errdefer c.wasm_runtime_destroy();

        if (!c.wasm_runtime_register_natives("env", @constCast(&native_symbols), 1)) {
            return error.RegisterNativesFailed;
        }
    }

    pub fn deinit() void {
        c.wasm_runtime_destroy();
    }
};

pub fn run_aot() !ProgramReturn {
    var result = ProgramReturn.init();

    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const aot_file = try std.fs.cwd().readFileAlloc(allocator, "zig-out/bin/program.aot", 4096);
    defer allocator.free(aot_file);

    const heap_buf = allocator.alloc(u8, HEAP_SIZE) catch {
        _ = std.fmt.bufPrint(&result.error_message, "Failed to allocate memory for WASM heap.", .{}) catch {};
        return result;
    };
    defer allocator.free(heap_buf);

    try Evaluator.init(heap_buf);

    var error_buf: [ERROR_SIZE]u8 = undefined;

    var start: c.struct_timespec = undefined;
    var end: c.struct_timespec = undefined;
    _ = c.clock_gettime(c.CLOCK_REALTIME, &start);

    var prog = try Program.init(aot_file, error_buf[0..], STACK_SIZE, HEAP_SIZE);
    defer prog.deinit();

    _ = c.clock_gettime(c.CLOCK_REALTIME, &end);
    const elapsed_ns = end.tv_nsec - start.tv_nsec;
    std.debug.print("Load to lookup time: {d} nanoseconds ({d:.6} ms)\n", .{ elapsed_ns, @as(f64, @floatFromInt(elapsed_ns)) / 1e6 });

    // Measure first call time separately
    _ = c.clock_gettime(c.CLOCK_REALTIME, &start);
    result = prog.call() catch |err| {
        _ = std.fmt.bufPrint(&result.error_message, "Error during first call: {s}", .{err}) catch {};
        return result;
    };

    _ = c.clock_gettime(c.CLOCK_REALTIME, &end);
    const first_call_time = end.tv_nsec - start.tv_nsec;
    std.debug.print("First call time: {d} nanoseconds ({d:.6} ms)\n", .{ first_call_time, @as(f64, @floatFromInt(first_call_time)) / 1e6 });

    // Measure subsequent calls time
    _ = c.clock_gettime(c.CLOCK_REALTIME, &start);

    for (0..ITERS) |_| {
        result = prog.call() catch |err| {
            _ = std.fmt.bufPrint(&result.error_message, "Error during subsequent call: {s}", .{err}) catch {};
            return result;
        };
    }

    _ = c.clock_gettime(c.CLOCK_REALTIME, &end);

    const time_per_op = @divTrunc(end.tv_nsec - start.tv_nsec, @as(c_long, ITERS));
    std.debug.print("Subsequent calls time: {d} ns/iter ({d:.6} ms/{d} iters)\n", .{ time_per_op, @as(f64, @floatFromInt(time_per_op)) / 1e6, ITERS });

    return result;
}

pub fn main() !void {
    const result = try run_aot();
    if (result.error_message[0] != 0) {
        std.debug.print("Error: {s}\n", .{result.error_message});
        return;
    }
    std.debug.print("WASM program returned: {d}\n", .{result.return_value});
    std.debug.assert(result.return_value == 1337);
}
