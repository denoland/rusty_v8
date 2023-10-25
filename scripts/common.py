import os
import subprocess


def system(argv, cwd=None, check=True, *args, **kwargs):
    print("[*] Executing", argv, "CWD:", cwd)
    return subprocess.run(argv, cwd=cwd, check=check, *args, **kwargs)


def git_fetch(url, ref, destination):
    # Git refetches everything every time when checking out a single commit
    head_path = os.path.join(destination, ".git/HEAD")
    if os.path.exists(head_path):
        with open(head_path, "r") as file:
            if ref == file.read().strip():
                print("[*] Already on", ref)
                return

    os.makedirs(destination, exist_ok=True)
    system(["git", "init", "-b=main"], destination)
    system(["git", "config", "advice.detachedHead", "false"], destination)
    try:
        system(["git", "remote", "add", "origin", url], destination)
    except subprocess.CalledProcessError:
        system(["git", "remote", "set-url", "origin", url], destination, check=False)
    system(["git", "fetch", "--depth=1", "origin", ref], destination)
    system(["git", "checkout", ref], destination)
