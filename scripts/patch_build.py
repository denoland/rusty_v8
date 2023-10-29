#!/usr/bin/env python3

import os
import sys
from utils import system


def restore(out_dir):
    system(["git", "restore", "BUILD.gn"], cwd=out_dir, check=False)
    rusty_v8 = os.path.join(out_dir, "rusty_v8")
    if os.path.exists(rusty_v8) or os.path.islink(rusty_v8):
        os.unlink(rusty_v8)


def patch(crate_dir, out_dir):
    gpath = os.path.join(out_dir, "build/config/gclient_args.gni")
    os.makedirs(os.path.dirname(gpath), exist_ok=True)
    open(gpath, "a").close()

    build_gn = None
    with open(os.path.join(crate_dir, "BUILD.gn"), "r") as f:
        build_gn = f.read()

    with open(os.path.join(out_dir, "BUILD.gn"), "a") as f:
        f.write("\n# === rusty_v8 ===\n")
        f.write(build_gn)

    try:
        os.symlink(
            os.path.abspath(os.path.join(crate_dir, "bindings")),
            os.path.abspath(os.path.join(out_dir, "rusty_v8")),
            target_is_directory=True,
        )
    except Exception as e:
        if os.name == "nt":
            print('[*] Enable "Developer Mode" in Windows settings')
        raise e


if __name__ == "__main__":
    if sys.argv[1] == "restore":
        restore(sys.argv[2])
    elif sys.argv[1] == "patch":
        patch(sys.argv[2], sys.argv[3])
    else:
        print("Unknown command")
        exit(1)
