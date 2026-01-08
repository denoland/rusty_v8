from v8_deps import deps
from download_file import DownloadUrl
import platform
import os
import tempfile
import tarfile
import sys

DIR = 'third_party/rust-toolchain'
SENTINEL = f'{DIR}/.rusty_v8_version'

host_os = platform.system().lower()
if host_os == "darwin":
    host_os = "mac"
elif host_os == "windows":
    host_os = "win"

host_cpu = platform.machine().lower()
if host_cpu == "x86_64":
    host_cpu = "x64"
elif host_cpu == "aarch64":
    host_cpu = "arm64"

eval_globals = {
    'host_os': host_os,
    'host_cpu': host_cpu,
}

dep = deps[DIR]
obj = next(obj for obj in dep['objects'] if eval(obj['condition'], eval_globals))
bucket = dep['bucket']
name = obj['object_name']
url = f'https://storage.googleapis.com/{bucket}/{name}'


def EnsureDirExists(path):
    if not os.path.exists(path):
        os.makedirs(path)


def DownloadAndUnpack(url, output_dir):
    """Download an archive from url and extract into output_dir."""
    with tempfile.TemporaryFile() as f:
        DownloadUrl(url, f)
        f.seek(0)
        EnsureDirExists(output_dir)
        with tarfile.open(mode='r:xz', fileobj=f) as z:
            z.extractall(path=output_dir)

try:
    with open(SENTINEL, 'r') as f:
        if f.read() == url:
            print(f'{DIR}: already downloaded')
            sys.exit()
except FileNotFoundError:
    pass

DownloadAndUnpack(url, DIR)

with open(SENTINEL, 'w') as f:
    f.write(url)
