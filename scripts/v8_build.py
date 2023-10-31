import os
import sys
import shutil
import argparse
import v8_download
from utils import system


def find_cc_wrapper():
    return (
        os.environ.get("SCCACHE")
        or shutil.which("sccache")
        or os.environ.get("CCACHE")
        or shutil.which("ccache")
        or None
    )


def find_ninja(root):
    return (
        os.environ.get("NINJA")
        or shutil.which(os.path.join(root, "third_party/ninja/ninja"))
        or shutil.which("ninja")
        or None
    )


def find_gn(root, host_os):
    is_depot_tools = "depot_tools" in os.environ["PATH"]
    if host_os == "linux":
        host_os = "linux64"
    return (
        os.environ.get("GN")
        or shutil.which(os.path.join(root, "buildtools", host_os, "gn"))
        or (not is_depot_tools and shutil.which("gn"))
        or None
    )


def s(value):
    if type(value) is bool:
        return str(value).lower()
    else:
        return str(value)


def q(value):
    return f'"{value}"'


def default_args():
    return {
        "clang_use_chrome_plugins": False,
        "is_component_build": False,
        "linux_use_bundled_binutils": False,
        "use_dummy_lastchange": True,
        "use_sysroot": False,
        "win_crt_flavor_agnostic": True,
        # Minimize size of debuginfo in distributed static library.
        "line_tables_only": True,
        "no_inline_line_tables": True,
        "symbol_level": 1,
        "use_debug_fission": False,
        "v8_enable_sandbox": False,
        "v8_enable_snapshot_compression": False,
        "v8_enable_javascript_promise_hooks": True,
        "v8_promise_internal_field_count": 1,
        "v8_use_external_startup_data": False,
        "v8_use_snapshot": True,
        # We prefer embedders to bring their own compression
        "v8_use_zlib": False,
        "v8_enable_snapshot_compression": False,
        # Disable handle zapping for performance
        "v8_enable_handle_zapping": False,
        # Ensure allocation of typed arrays and arraybuffers always goes through
        # the embedder's ArrayBufferAllocator, otherwise small buffers get moved
        # around by the garbage collector but embedders normally want them to have
        # fixed addresses.
        "v8_typed_array_max_size_in_heap": 0,
        # Enabling the shared read-only heap comes with a restriction that all
        # isolates running at the same time must be created from the same snapshot.
        # This is problematic for Deno, which has separate "runtime" and "typescript
        # compiler" snapshots, and sometimes uses them both at the same time.
        "v8_enable_shared_ro_heap": False,
        # V8 11.6 hardcoded an assumption in `mksnapshot` that shared RO heap
        # is enabled. In our case it's disabled so without this flag we can't
        # compile.
        "v8_enable_verify_heap": False,
        # V8 introduced a bug in 11.1 that causes the External Pointer Table to never
        # be cleaned which causes resource exhaustion. Disabling pointer compression
        # makes sure that the EPT is not used.
        # https:#bugs.chromium.org/p/v8/issues/detail?id=13640&q=garbage%20collection&can=2
        "v8_enable_pointer_compression": False,
        # Maglev *should* be supported when pointer compression is disabled as per
        # https:#chromium-review.googlesource.com/c/v8/v8/+/4753150, but it still
        # fails to compile.
        "v8_enable_maglev": False,
        # Enable Deno-specific extra bindings
        "deno_enable_extras": True,
    }


def build_v8(
    crate_root,
    root,
    gn_root,
    host_os,
    host_cpu,
    target_os,
    target_cpu,
    is_debug,
    use_custom_libcxx,
):
    gn_args = default_args()
    gn_args["is_debug"] = is_debug
    gn_args["target_cpu"] = q(target_cpu)
    gn_args["v8_target_cpu"] = q(target_cpu)

    cc_wrapper = find_cc_wrapper()
    if cc_wrapper:
        gn_args["cc_wrapper"] = q(cc_wrapper)

    if not use_custom_libcxx:
        gn_args["use_custom_libcxx"] = False

    if host_cpu != target_cpu:
        gn_args["use_sysroot"] = True

    if host_os != target_os:
        gn_args["target_os"] = q(target_os)

    if "DISABLE_CLANG" in os.environ:
        gn_args["is_clang"] = False
        gn_args["line_tables_only"] = False
    elif "CLANG_BASE_PATH" in os.environ:
        gn_args["clang_base_path"] = q(os.environ["CLANG_BASE_PATH"])
        gn_args["clang_use_chrome_plugins"] = False
        gn_args["treat_warnings_as_errors"] = False

    for arg in (os.environ.get("GN_ARGS") or "").split():
        k, v = arg.split("=", 1)
        gn_args[k] = v

    gn_args = [f"{k}={s(v)}" for k, v in gn_args.items()]
    gn_args = " ".join(gn_args)
    print(gn_args)

    v8_download.main(
        crate_root,
        root,
        [host_os, host_cpu, target_os, target_cpu],
        host_os=host_os,
        host_cpu=host_cpu,
    )

    gn_r = f"--root={root}"
    gn_se = f"--script-executable={sys.executable}"

    system([find_gn(root, host_os), gn_r, gn_se, "gen", gn_root, f"--args={gn_args}"])
    if "PRINT_GN_ARGS" in os.environ:
        system([find_gn(root, host_os), gn_r, gn_se, "args", gn_root, "--list"])

    system([find_ninja(root), "-C", gn_root, "rusty_v8"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--crate-root", required=True)
    parser.add_argument("--root", required=True)
    parser.add_argument("--gn-root", required=True)
    parser.add_argument("--host-os", required=True)
    parser.add_argument("--host-cpu", required=True)
    parser.add_argument("--target-os", required=True)
    parser.add_argument("--target-cpu", required=True)
    parser.add_argument("--is-debug", type=bool, default=False)
    parser.add_argument("--use-custom-libcxx", type=bool, default=True)
    args = parser.parse_args()
    build_v8(
        args.crate_root,
        args.root,
        args.gn_root,
        args.host_os,
        args.host_cpu,
        args.target_os,
        args.target_cpu,
        args.is_debug,
        args.use_custom_libcxx,
    )
