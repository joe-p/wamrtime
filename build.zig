const std = @import("std");

pub fn build(b: *std.Build) void {
    const wasm_target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
    });

    const wasm_optimize = std.builtin.OptimizeMode.ReleaseSmall;

    const wasm_mod = b.addModule("program", .{
        .root_source_file = b.path("wasm-src/wasm.zig"),
        .target = wasm_target,
        .optimize = wasm_optimize,
    });

    const wasm_progrram = b.addExecutable(.{
        .name = "program",
        .root_module = wasm_mod,
    });
    wasm_progrram.import_memory = true;
    wasm_progrram.entry = .disabled;
    wasm_progrram.rdynamic = true;

    b.installArtifact(wasm_progrram);
}
