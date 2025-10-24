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

const MAX_PROGRAMS = 256;

const InitNextContext = struct {
    evaluator: *Evaluator,
    aot_bytes: []const []u8,
    result: ?anyerror!void = null,
};

fn initNextThread(ctx: *InitNextContext) void {
    ctx.result = ctx.evaluator.init_next(ctx.aot_bytes);
}

const DeinitProgramsContext = struct {
    programs: [MAX_PROGRAMS]Program,
    len: usize,
};

fn deinit_programs(ctx: *DeinitProgramsContext) void {
    for (0..ctx.len) |idx| {
        ctx.programs[idx].deinit();
    }
}

const Evaluator = struct {
    heap_buf: []u8,
    error_buf: [ERROR_SIZE]u8,
    init_thread: ?std.Thread = null,
    next_ctx: ?InitNextContext = null,
    programs: struct {
        current: [MAX_PROGRAMS]Program,
        current_len: usize,
        next: [MAX_PROGRAMS]Program,
        next_len: usize,
        prev: [MAX_PROGRAMS]Program,
        prev_len: usize,
    },

    fn init_next(self: *Evaluator, aot_bytes: []const []u8) !void {
        for (0..self.programs.prev_len) |idx| {
            self.programs.prev[idx].deinit();
        }

        for (0..aot_bytes.len) |idx| {
            const aot = aot_bytes[idx];
            const program = try Program.init(aot, &self.error_buf, STACK_SIZE, HEAP_SIZE);
            self.programs.next[idx] = program;
        }
        self.programs.next_len = aot_bytes.len;
    }

    pub fn next_round(self: *Evaluator, aot_bytes: []const []u8) !void {
        const join_start = try std.time.Instant.now();
        if (self.init_thread) |thread| {
            thread.join();
            self.init_thread = null;
        }
        const join_duration_ns = (try std.time.Instant.now()).since(join_start);
        std.debug.print("Join duration: {d} ns\n", .{join_duration_ns});

        const spawn_start = try std.time.Instant.now();

        self.programs.prev = self.programs.current;
        self.programs.prev_len = self.programs.current_len;

        self.programs.current = self.programs.next;
        self.programs.current_len = self.programs.next_len;

        self.next_ctx = InitNextContext{
            .evaluator = self,
            .aot_bytes = aot_bytes,
        };

        self.init_thread = try std.Thread.spawn(.{}, initNextThread, .{&self.next_ctx.?});
        const spawn_duration_ns = (try std.time.Instant.now()).since(spawn_start);
        std.debug.print("Spawn duration: {d} ns\n", .{spawn_duration_ns});

        for (0..self.programs.current_len) |idx| {
            const start = try std.time.Instant.now();
            const res = try self.programs.current[idx].call();
            const duration_ns = (try std.time.Instant.now()).since(start);
            std.debug.print("Program {d} executed in {d} ns with return value {d}\n", .{ idx, duration_ns, res.return_value });
            std.debug.assert(res.return_value == 1337);
        }
    }

    pub fn init(heap_buf: []u8) !Evaluator {
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

        return Evaluator{
            .heap_buf = heap_buf,
            .error_buf = undefined,
            .programs = .{
                .current = undefined,
                .current_len = 0,
                .next = undefined,
                .next_len = 0,
                .prev = undefined,
                .prev_len = 0,
            },
        };
    }

    pub fn deinit(self: *Evaluator) void {
        if (self.init_thread) |thread| {
            thread.join();
            self.init_thread = null;
        }

        defer c.wasm_runtime_destroy();
    }
};

pub fn run_aot() !void {
    var result = ProgramReturn.init();

    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const aot_file = try std.fs.cwd().readFileAlloc(allocator, "zig-out/bin/program.aot", 4096);
    defer allocator.free(aot_file);

    const heap_buf = allocator.alloc(u8, HEAP_SIZE) catch {
        _ = std.fmt.bufPrint(&result.error_message, "Failed to allocate memory for WASM heap.", .{}) catch {};
        return;
    };
    defer allocator.free(heap_buf);

    var eval = try Evaluator.init(heap_buf);
    defer eval.deinit();

    const arr = [_][]u8{
        aot_file,
    };

    for (0..10) |i| {
        std.debug.print("\nIteration {d}:\n", .{i + 1});
        try (&eval).next_round(&arr);
    }

    std.debug.print("\nSleeping for 2 seconds before final iteration...\n", .{});
    std.Thread.sleep(std.time.ns_per_ms * 2000);

    const start = try std.time.Instant.now();
    try (&eval).next_round(&arr);
    const duration_ns = (try std.time.Instant.now()).since(start);
    std.debug.print("Final iteration executed in {d} ns\n", .{duration_ns});

    std.debug.print("All iterations completed successfully.\n", .{});
}

pub fn main() !void {
    try run_aot();
}
