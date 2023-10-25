#!/usr/bin/env python3

import io
import os
import re
import sys
import stat
import zipfile
import argparse
import platform
import urllib.request


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


def build_download_url(package, version):
    return f"https://chrome-infra-packages.appspot.com/dl/{package}/+/{version}"


def replace_placeholders(value):
    os = None
    if sys.platform.startswith("linux"):
        os = "linux"
    elif sys.platform.startswith("win32"):
        os = "windows"
    elif sys.platform.startswith("darwin"):
        os = "mac"
    else:
        raise "Unknown OS: " + sys.platform

    arch = None
    machine = platform.machine().lower()
    if machine == "x86_64" or machine == "amd64":
        arch = "amd64"
    elif machine == "aarch64" or machine.startswith("arm64"):
        arch = "arm64"
    else:
        raise "Unknown Arch: " + machine

    platform_ = os + "-" + arch

    return (
        value.replace("${os}", os)
        .replace("${arch}", arch)
        .replace("${platform}", platform_)
    )


def download_package(package, version, destination):
    version_path = os.path.join(destination, ".dumb_cipd")
    if os.path.exists(version_path):
        with open(version_path, "r") as file:
            if version == file.read().strip():
                print("[*] Aleady on version", version)
                return

    package = replace_placeholders(package)
    url = build_download_url(package, version)
    print("[*] Download URL", url)
    pkg = urllib.request.urlopen(url).read()
    with io.BytesIO(pkg) as i:
        with ZipFileWithExecutableBit(i) as z:
            z.extractall(destination)

    with open(version_path, "w") as file:
        file.write(version)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("-n", "--name", required=True)
    parser.add_argument("-v", "--version", required=True)
    parser.add_argument("-d", "--destination", required=True)
    args = parser.parse_args()
    download_package(args.name, args.version, args.path)
