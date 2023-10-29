#!/usr/bin/env python3

import io
import os
import sys
import argparse
import platform
import urllib.request
from utils import ZipFileWithExecutableBit


def get_cipd_os():
    if sys.platform.startswith("linux"):
        return "linux"
    elif sys.platform.startswith("win32"):
        return "windows"
    elif sys.platform.startswith("darwin"):
        return "mac"
    else:
        return sys.platform


def get_cipd_arch():
    machine = platform.machine().lower()
    if machine == "x86_64" or machine == "amd64":
        return "amd64"
    elif machine == "aarch64" or machine.startswith("arm64"):
        return "arm64"
    else:
        return machine


def replace_placeholders(package_name, cipd_os, cipd_arch):
    cipd_platform = cipd_os + "-" + cipd_arch
    return (
        package_name.replace("{{", "{")
        .replace("}}", "}")
        .replace("${os}", cipd_os)
        .replace("${arch}", cipd_arch)
        .replace("${platform}", cipd_platform)
    )


def package_download_url(name, version):
    return f"https://chrome-infra-packages.appspot.com/dl/{name}/+/{version}"


def download_package(name, version, destination, cipd_os=None, cipd_arch=None):
    version_path = os.path.join(destination, ".cipd_version")

    if os.path.exists(version_path):
        with open(version_path, "r") as file:
            if version == file.read().strip():
                print("[*] Aleady on version", version)
                return

    name = replace_placeholders(
        name, cipd_os or get_cipd_os(), cipd_arch or get_cipd_arch()
    )
    url = package_download_url(name, version)
    print("[*] Download URL", url)

    data = urllib.request.urlopen(url).read()
    with io.BytesIO(data) as b:
        with ZipFileWithExecutableBit(b) as z:
            z.extractall(destination)

    with open(version_path, "w") as file:
        file.write(version)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("name")
    parser.add_argument("version")
    parser.add_argument("destination")
    parser.add_argument("--os")
    parser.add_argument("--arch")
    args = parser.parse_args()
    download_package(
        args.name, args.version, args.destination, cipd_os=args.os, cipd_arch=args.arch
    )
