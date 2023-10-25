#!/usr/bin/env python3

import os
import sys
import shutil
import argparse
import platform
import dumb_cipd
from common import system, git_fetch

cipd_bin = None
if "USE_CIPD" in os.environ:
    cipd_bin = shutil.which("cipd")


def load_deps(path):
    locs = {}
    locs["Var"] = lambda value: locs["vars"][value]
    locs["Str"] = lambda value: value

    with open(path, "r") as f:
        exec(f.read(), None, locs)

    for path, spec in locs["deps"].items():
        if type(spec) is str:
            spec = {"url": spec}
            locs["deps"][path] = spec
        if "url" in spec:
            url, ref = spec["url"].split("@")
            spec["url"] = url
            spec["ref"] = ref

    return locs


def default_env():
    env = {
        # unneeded
        "build_with_chromium": False,
        "check_v8_header_includes": False,
        "checkout_clang_coverage_tools": False,
        "checkout_clang_tidy": False,
        "checkout_fuchsia": False,
        "checkout_fuchsia_no_hooks": False,
        "checkout_fuchsia_product_bundles": False,
        "checkout_instrumented_libraries": False,
        "checkout_ittapi": False,
        "checkout_v8_builtins_pgo_profiles": False,
        "download_gcmole": False,
        "download_jsfunfuzz": False,
        "download_prebuilt_bazel": False,
        # host_os
        "checkout_android": False,
        "checkout_linux": False,
        "checkout_mac": False,
        "checkout_win": False,
        # host_cpu
        "checkout_arm": False,
        "checkout_arm64": False,
        "checkout_x86": False,
        "checkout_x64": False,
    }

    host_os = "unknown"
    if sys.platform.startswith("linux"):
        host_os = "linux"
    elif sys.platform.startswith("win32"):
        host_os = "win"
    elif sys.platform.startswith("darwin"):
        host_os = "mac"
    env["host_os"] = host_os

    host_cpu = "unknown"
    machine = platform.machine().lower()
    if machine == "x86_64" or machine == "amd64":
        host_cpu = "x64"
    elif machine == "aarch64" or machine.startswith("arm64"):
        host_cpu = "arm64"
    env["host_cpu"] = host_cpu
    return env


def check_condition(obj, env):
    if not env or not "condition" in obj:
        return True

    try:
        return eval(obj["condition"], None, env)
    except Exception as e:
        print("[*] Failed to check condition", e)
        return False


def run_hook(hook, cwd, env=None):
    print("\n[*] Running", hook, "\n")
    if not check_condition(hook, env):
        print("[*] Condition didn't match")
        return

    action = hook["action"]
    if "python3" in action[0]:
        action[0] = sys.executable

    return system(action, cwd)


def fetch(dep, destination, env=None):
    print("\n[*] Installing", dep, "\n")
    if not check_condition(dep, env):
        print("[*] Condition didn't match")
        return

    if "dep_type" in dep and dep["dep_type"] == "cipd":
        cipd_fetch(dep, destination)
    elif "url" in dep:
        git_fetch(dep["url"], dep["ref"], destination)


def cipd_real_fetch(dep, destination):
    ensure = (
        "\n".join(
            [
                package["package"] + " " + package["version"]
                for package in dep["packages"]
            ]
        )
        .replace("{{", "{")
        .replace("}}", "}")
    )
    print("[*] CIPD ensure file:\n", ensure)
    ensure_path = os.path.join(destination, ".cipd_ensure")
    with open(ensure_path, "w") as f:
        f.write(ensure)

    system([cipd_bin, "ensure", "-root", destination, "-ensure-file", ensure_path])


def cipd_dumb_fetch(dep, destination):
    for pkg in dep["packages"]:
        print("[*] Downloading CIPD package:", pkg)
        dumb_cipd.download_package(
            pkg["package"].replace("{{", "{").replace("}}", "}"),
            pkg["version"],
            destination,
        )


def cipd_fetch(dep, destination):
    os.makedirs(destination, exist_ok=True)
    if cipd_bin:
        return cipd_real_fetch(dep, destination)
    else:
        return cipd_dumb_fetch(dep, destination)


def run_hooks(out_dir, hooks, names, env=None):
    for hook in hooks:
        if not hook["name"] in names:
            continue
        run_hook(hook, out_dir, env)


def fetch_deps(out_dir, deps, paths, env=None):
    for path in paths:
        fetch(deps[path], os.path.join(out_dir, path), env)


def main(
    out_dir, deps_path, hooks=[], deps=[], checkout=[], host_os=None, host_cpu=None
):
    print("[*] Use real CIPD:", bool(cipd_bin))
    DEPS = load_deps(deps_path)

    env = default_env()
    for name in checkout:
        env["checkout_" + name] = True
    if host_os:
        env["host_os"] = host_os
    if host_cpu:
        env["host_cpu"] = host_cpu

    fetch_deps(out_dir, DEPS["deps"], deps, env)
    run_hooks(out_dir, DEPS["hooks"], hooks, env)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("-i", "--deps-path", required=True)
    parser.add_argument("-c", "--checkout", nargs="*", default=[])
    parser.add_argument("-h", "--hook", nargs="*", default=[])
    parser.add_argument("-d", "--dep", nargs="*", default=[])
    parser.add_argument("--host-os")
    parser.add_argument("--host-cpu")
    args = parser.parse_args()
    main(
        args.out_dir,
        args.deps_path,
        hooks=args.hook,
        deps=args.dep,
        checkout=args.checkout,
        host_os=args.host_os,
        host_cpu=args.host_cpu,
    )
