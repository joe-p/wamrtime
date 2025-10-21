const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Create the main executable
    const exe = b.addExecutable(.{
        .name = "wamr-zig",
        .root_module = b.createModule(.{
            .target = target,
            .optimize = optimize,
            .root_source_file = b.path("src/main.zig"),
        }),
    });

    // Add WAMR include directories
    const wamr_root = "wasm-micro-runtime";
    exe.addIncludePath(b.path(wamr_root ++ "/core/iwasm/include"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/iwasm/interpreter"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/iwasm/aot"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/iwasm/libraries/libc-builtin"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/iwasm/common"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/shared/include"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/shared/platform/include"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/shared/platform/linux"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/shared/platform/common/posix"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/shared/mem-alloc"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/shared/utils"));
    exe.addIncludePath(b.path(wamr_root ++ "/core/shared/utils/uncommon"));
    exe.addIncludePath(b.path("src"));

    // Link with the pre-built vmlib (you'll need to build this with CMake first)
    // Assuming the CMake build puts the library in a build directory
    exe.addLibraryPath(b.path("build"));
    exe.linkSystemLibrary("vmlib");

    // Link with system libraries
    exe.linkLibC();
    exe.linkSystemLibrary("m");
    exe.linkSystemLibrary("dl");
    exe.linkSystemLibrary("pthread");

    // On Linux, also link rt
    if (exe.root_module.resolved_target.?.result.os.tag == .linux) {
        exe.linkSystemLibrary("rt");
    }

    // Install the executable
    b.installArtifact(exe);

    // Create a run step
    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());

    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);

    const exe_check = b.addExecutable(.{ .name = exe.name, .root_module = exe.root_module });

    const check = b.step("check", "Check if foo compiles");
    check.dependOn(&exe_check.step);
}
