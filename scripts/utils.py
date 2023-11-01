import os
import zipfile
import subprocess


class ZipFileWithExecutableBit(zipfile.ZipFile):
    if os.name == "posix":

        def _extract_member(self, member, targetpath, pwd):
            if not isinstance(member, zipfile.ZipInfo):
                member = self.getinfo(member)
            targetpath = super()._extract_member(member, targetpath, pwd)
            try:
                st = os.stat(targetpath)
                os.chmod(targetpath, st.st_mode | 0o0111)
            except:
                pass
            return targetpath


class NonFallibleDict(dict):
    def __getitem__(self, key):
        try:
            return super().__getitem__(key)
        except KeyError:
            return None


def str2bool(string):
    return string.lower() in ["yes", "true", "t", "y", "1"]


def system(*args, check=True, **kwargs):
    kwargs["check"] = check

    print(f"[*] Executing {', '.join(map(str, args))} with {kwargs}")
    return subprocess.run(*args, **kwargs)


def git_fetch(url, ref, destination):
    # Git refetches everything every time when checking out a single commit
    head_path = os.path.join(destination, ".git/HEAD")
    if os.path.exists(head_path):
        with open(head_path, "r") as file:
            if ref == file.read().strip():
                print("[*] Already on", ref)
                return

    os.makedirs(destination, exist_ok=True)
    system(["git", "init", "-b=main"], cwd=destination)
    try:
        system(["git", "remote", "add", "origin", url], cwd=destination)
    except subprocess.CalledProcessError:
        system(["git", "remote", "set-url", "origin", url], cwd=destination)
    system(["git", "fetch", "--depth=1", "origin", ref], cwd=destination)
    system(["git", "-c", "advice.detachedHead=false", "checkout", ref], cwd=destination)


def load_git_config(path):
    stdout = system(
        ["git", "config", "-f", path, "-l"], capture_output=True, text=True
    ).stdout

    out = {}
    for line in stdout.splitlines():
        [key, value] = line.split("=", 1)
        parts = key.split(".")

        cur = out
        for part in parts[:-1]:
            if not part in cur:
                cur[part] = {}
            cur = cur[part]
        cur[parts[-1]] = value

    return out
