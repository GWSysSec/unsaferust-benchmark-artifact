#!/usr/bin/env python3
"""Keep the source needed for this x86_64 rustc build."""

import re
import shutil
import sys
from pathlib import Path


def replace_exact(path: Path, old: str, new: str = "") -> None:
    text = path.read_text()
    if text.count(old) != 1:
        raise RuntimeError(f"expected one occurrence in {path}: {old!r}")
    path.write_text(text.replace(old, new))


def remove_blocks(path: Path, marker: str, expected: int) -> None:
    text = path.read_text()
    pattern = re.compile(
        rf"(?m)^[ \t]*{re.escape(marker)} BEGIN\n.*?^[ \t]*"
        rf"{re.escape(marker)} END\n",
        re.DOTALL,
    )
    result, count = pattern.subn("", text)
    if count != expected:
        raise RuntimeError(f"expected {expected} blocks in {path}, found {count}")
    path.write_text(result)


def main(root: Path) -> None:
    project = root / "src/llvm-project"
    llvm = project / "llvm"
    for relative in (
        "lib/SVF",
        "lib/Transforms/SVFAnalysis",
        "include/llvm/Transforms/SVFAnalysis",
        "test/Analysis/SVF",
    ):
        path = llvm / relative
        if not path.is_dir():
            raise RuntimeError(f"expected source directory: {path}")
        shutil.rmtree(path)

    # This custom note describes the removed passes and is not build input.
    note = llvm / "lib/Passes/passorder.md"
    if not note.is_file():
        raise RuntimeError(f"expected pass order note: {note}")
    note.unlink()

    replace_exact(llvm / "lib/CMakeLists.txt", "add_subdirectory(SVF)\n")
    remove_blocks(llvm / "lib/Transforms/CMakeLists.txt", "# UNSAFE-SVF", 1)
    remove_blocks(llvm / "lib/Passes/CMakeLists.txt", "# UNSAFE-SVF", 1)
    remove_blocks(llvm / "lib/Passes/PassBuilder.cpp", "// UNSAFE-SVF", 1)
    pipelines = llvm / "lib/Passes/PassBuilderPipelines.cpp"
    remove_blocks(pipelines, "// UNSAFE-SVF", 5)
    replace_exact(pipelines, "  // Removed duplicate UnsafeHeapInstrumentation\n\n")
    registry = llvm / "lib/Passes/PassRegistry.def"
    replace_exact(registry, 'MODULE_ANALYSIS("unsafe-heap-alloc-analysis", UnsafeHeapAllocAnalysis())\n')
    replace_exact(registry, 'MODULE_PASS("runtime-alias", RuntimeAliasPass())\n')
    replace_exact(registry, 'MODULE_PASS("unsafe-heap-instrumentation", UnsafeHeapInstrumentation())\n')
    replace_exact(
        llvm / "cmake/modules/LLVMConfig.cmake.in",
        "# SVF embeds an in-tree Z3 build. Load its exported targets so the\n"
        "# z3::libz3 dependency referenced by LLVMSvfCore/LLVMSvfLLVM resolves.\n"
        'set(_svf_z3_targets "@CMAKE_BINARY_DIR@/lib/SVF/z3/Z3Targets.cmake")\n'
        'if(NOT TARGET z3::libz3 AND EXISTS "${_svf_z3_targets}")\n'
        '  include("${_svf_z3_targets}")\n'
        "endif()\n"
        "unset(_svf_z3_targets)\n\n",
    )

    # Rust bootstrap enables neither LLVM's optional Z3 solver nor Clang. Keep
    # the small set of LLVM projects rustc may use, without their test suites.
    for relative in (
        "bolt", "clang", "clang-tools-extra", "cross-project-tests",
        "flang", "libc", "libclc", "libcxx", "libcxxabi", "lldb",
        "llvm-libgcc", "mlir", "openmp", "polly", "pstl",
    ):
        path = project / relative
        if not path.is_dir():
            raise RuntimeError(f"expected LLVM project directory: {path}")
        shutil.rmtree(path)
    for relative in ("test", "unittests", "docs", "examples", "benchmarks", "utils/gn"):
        path = llvm / relative
        if not path.is_dir():
            raise RuntimeError(f"expected LLVM non-build directory: {path}")
        shutil.rmtree(path)
    for relative in (
        "cmake/modules/FindZ3.cmake",
        "lib/Support/Z3Solver.cpp",
        "utils/convert-constraint-log-to-z3.py",
    ):
        path = llvm / relative
        if not path.is_file():
            raise RuntimeError(f"expected optional Z3 source: {path}")
        path.unlink()
    replace_exact(
        llvm / "CMakeLists.txt",
        'set(LLVM_Z3_INSTALL_DIR "" CACHE STRING "Install directory of the Z3 solver.")\n'
        "\n"
        "option(LLVM_ENABLE_Z3_SOLVER\n"
        '  "Enable Support for the Z3 constraint solver in LLVM."\n'
        "  ${LLVM_ENABLE_Z3_SOLVER_DEFAULT}\n"
        ")\n"
        "\n"
        "if (LLVM_ENABLE_Z3_SOLVER)\n"
        "  find_package(Z3 4.7.1)\n"
        "\n"
        "  if (LLVM_Z3_INSTALL_DIR)\n"
        "    if (NOT Z3_FOUND)\n"
        '      message(FATAL_ERROR "Z3 >= 4.7.1 has not been found in LLVM_Z3_INSTALL_DIR: ${LLVM_Z3_INSTALL_DIR}.")\n'
        "    endif()\n"
        "  endif()\n"
        "\n"
        "  if (NOT Z3_FOUND)\n"
        '    message(FATAL_ERROR "LLVM_ENABLE_Z3_SOLVER cannot be enabled when Z3 is not available.")\n'
        "  endif()\n"
        "\n"
        "  set(LLVM_WITH_Z3 1)\n"
        "endif()\n"
        "\n"
        'set(LLVM_ENABLE_Z3_SOLVER_DEFAULT "${Z3_FOUND}")\n',
        "set(LLVM_WITH_Z3 0)\n",
    )
    support_cmake = llvm / "lib/Support/CMakeLists.txt"
    replace_exact(
        support_cmake,
        "# Link Z3 if the user wants to build it.\n"
        "if(LLVM_WITH_Z3)\n"
        "  set(system_libs ${system_libs} ${Z3_LIBRARIES})\n"
        "endif()\n\n",
    )
    replace_exact(support_cmake, "  Z3Solver.cpp\n")
    replace_exact(
        support_cmake,
        "if(LLVM_WITH_Z3)\n"
        "  target_include_directories(LLVMSupport SYSTEM\n"
        "    PRIVATE\n"
        "    ${Z3_INCLUDE_DIR}\n"
        "    )\n"
        "endif()\n",
    )
    replace_exact(
        llvm / "include/llvm/Support/SMTAPI.h",
        "/// Convenience method to create and Z3Solver object\n"
        "SMTSolverRef CreateZ3Solver();\n",
    )

    forbidden = re.compile(
        r"UNSAFE-SVF|SVFAnalysis|LLVMSvf|RuntimeAliasPass|"
        r"UnsafeHeapAllocAnalysis|UnsafeHeapInstrumentation|lib/SVF"
    )
    for relative in (
        "lib/Passes/PassBuilder.cpp",
        "lib/Passes/PassBuilderPipelines.cpp",
        "lib/Passes/PassRegistry.def",
        "lib/Passes/CMakeLists.txt",
        "lib/Transforms/CMakeLists.txt",
        "lib/CMakeLists.txt",
        "cmake/modules/LLVMConfig.cmake.in",
    ):
        path = llvm / relative
        if forbidden.search(path.read_text()):
            raise RuntimeError(f"SVF reference remains in {path}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: prune_compiler_source.py EXPORTED_RUSTC_DIRECTORY")
    main(Path(sys.argv[1]))
