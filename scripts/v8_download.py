#!/usr/bin/env python3

import os
import gclient
import argparse
import patch_build
from utils import NonFallibleDict, git_fetch


def main(crate_dir, out_dir, checkout, host_os=None, host_cpu=None):
    os.makedirs(out_dir, exist_ok=True)

    patch_build.restore(out_dir)
    git_fetch("https://github.com/denoland/v8", "11.8-lkgr-denoland", out_dir)

    git_fetch(
        "https://github.com/denoland/chromium_build",
        "20230426_rustyv8",
        os.path.join(out_dir, "build"),
    )

    deps = gclient.load_deps(os.path.join(out_dir, "DEPS"))
    vars = NonFallibleDict(deps["vars"])
    vars["host_os"] = host_os
    vars["host_cpu"] = host_cpu
    for c in checkout:
        vars["checkout_" + c] = True

    gclient.fetch_deps(
        out_dir,
        deps["deps"],
        [
            "base/trace_event/common",
            "build",
            "buildtools",
            "buildtools/linux64",
            "buildtools/mac",
            "buildtools/win",
            "third_party/abseil-cpp",
            "third_party/android_platform",
            "third_party/android_toolchain/ndk",
            "third_party/catapult",
            "third_party/colorama/src",
            "third_party/cpu_features/src",
            "third_party/googletest/src",
            "third_party/icu",
            "third_party/jinja2",
            "third_party/libc++/src",
            "third_party/libc++abi/src",
            "third_party/libunwind/src",
            "third_party/markupsafe",
            "third_party/ninja",
            "third_party/requests",
            "third_party/zlib",
            "tools/clang",
        ],
        vars,
    )

    gclient.run_hooks(
        out_dir,
        deps["hooks"],
        [
            "clang",
            "lastchange",
            "mac_toolchain",
            "win_toolchain",
            "sysroot_arm",
            "sysroot_arm64",
            "sysroot_x86",
            "sysroot_x64",
        ],
        vars,
    )

    patch_build.patch(crate_dir, out_dir)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("crate_dir")
    parser.add_argument("out_dir")
    parser.add_argument("checkout", nargs="*", default=[])
    parser.add_argument("--host-os")
    parser.add_argument("--host-cpu")
    args = parser.parse_args()
    main(
        args.crate_dir,
        args.out_dir,
        args.checkout,
        host_os=args.host_os,
        host_cpu=args.host_cpu,
    )
