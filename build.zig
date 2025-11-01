const std = @import("std");

fn build_wasm(b: *std.Build, comptime name: []const u8) void {
    const wasm_target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
    });

    const wasm_optimize = std.builtin.OptimizeMode.ReleaseSmall;

    const wasm_mod = b.addModule(name, .{
        .root_source_file = b.path("wasm-src/" ++ name ++ ".zig"),
        .target = wasm_target,
        .optimize = wasm_optimize,
    });

    const wasm_progrram = b.addExecutable(.{
        .name = name,
        .root_module = wasm_mod,
    });
    wasm_progrram.import_memory = true;
    wasm_progrram.entry = .disabled;
    wasm_progrram.rdynamic = true;

    b.installArtifact(wasm_progrram);
}

pub fn build(b: *std.Build) void {
    build_wasm(b, "program");
    build_wasm(b, "avm");
    build_wasm(b, "avm_complex");
}
