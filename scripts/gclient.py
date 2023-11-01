#!/usr/bin/env python3

import os
import sys
import cipd
import argparse
from utils import NonFallibleDict, system, git_fetch


def load_deps(path):
    env = {}
    env["Str"] = lambda value: value
    env["Var"] = lambda value: env["vars"][value]

    with open(path, "r") as f:
        exec(f.read(), None, env)

    for path, spec in env["deps"].items():
        if type(spec) is str:
            spec = {"url": spec}
            env["deps"][path] = spec
        if "url" in spec:
            url, ref = spec["url"].split("@")
            spec["url"] = url
            spec["ref"] = ref

    return env


def check_condition(condition, vars=None):
    if not vars or not condition:
        return True

    if not isinstance(vars, NonFallibleDict):
        vars = NonFallibleDict(vars)

    return eval(condition, None, vars) == True


def cipd_fetch(dep, destination):
    for pkg in dep["packages"]:
        print("[*] Downloading CIPD package:", pkg)
        cipd.download_package(
            pkg["package"],
            pkg["version"],
            destination,
        )


def fetch_dep(dep, destination, vars=None):
    print("\n[*] Installing", dep, "\n")
    if not check_condition(dep.get("condition"), vars):
        print("[*] Condition didn't match")
        return

    if "dep_type" in dep and dep["dep_type"] == "cipd":
        cipd_fetch(dep, destination)
    elif "url" in dep:
        git_fetch(dep["url"], dep["ref"], destination)


def run_hook(hook, cwd=None, vars=None):
    print("\n[*] Running", hook, "\n")
    if not check_condition(hook.get("condition"), vars):
        print("[*] Condition didn't match")
        return

    action = hook["action"]
    if "python3" in action[0]:
        action[0] = sys.executable

    return system(action, cwd=cwd)


def fetch_deps(destination, deps, paths, vars=None):
    for path in paths:
        fetch_dep(deps[path], os.path.join(destination, path), vars)


def run_hooks(destination, hooks, names, vars=None):
    for hook in hooks:
        if not hook["name"] in names:
            continue
        run_hook(hook, destination, vars)


def main(destination, deps_file, host_os, host_cpu, checkout, deps, hooks):
    deps = load_deps(deps_file)
    vars = NonFallibleDict(deps["vars"])
    vars["host_os"] = host_os
    vars["host_cpu"] = host_cpu
    for checkout in checkout:
        vars["checkout_" + checkout] = True
    fetch_deps(destination, deps["deps"], deps, vars)
    run_hooks(destination, deps["hooks"], hooks, vars)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("destination", required=True)
    parser.add_argument("deps-file", required=True)
    parser.add_argument("--host-os", required=True)
    parser.add_argument("--host-cpu", required=True)
    parser.add_argument("-c", "--checkout", nargs="*", default=[])
    parser.add_argument("-d", "--dep", nargs="*", default=[])
    parser.add_argument("-h", "--hook", nargs="*", default=[])
    args = parser.parse_args()
    main(
        args.destination,
        args.deps_file,
        args.host_os,
        args.host_cpu,
        args.checkout,
        args.dep,
        args.hook,
    )
