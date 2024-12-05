#!/usr/bin/env python
# Copyright (c) 2012 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.
"""This script is used to download prebuilt gn/ninja binaries."""

import platform
import json
import argparse
import os
import sys
import zipfile
import tempfile
import time
import http.client
from v8_deps import Var
from urllib.error import HTTPError, URLError
from stat import ST_MODE
from urllib.request import urlopen
from urllib.parse import urlparse


def get_platform():
    system = platform.system().lower()
    if system == 'darwin':
        system = 'mac'
    machine = platform.machine().lower()
    if machine == 'x86_64':
        machine = 'amd64'
    elif machine == 'aarch64':
        machine = 'arm64'

    return f'{system}-{machine}'


PLATFORM = get_platform()
is_windows = PLATFORM.startswith('windows')

RESOLVE_URL = 'https://chrome-infra-packages.appspot.com/_ah/api/repo/v1/instance/resolve?package_name={}&version={}'
INSTANCE_URL = 'https://chrome-infra-packages.appspot.com/_ah/api/repo/v1/instance?package_name={}&instance_id={}'

NINJA_VERSION = Var('ninja_version')
GN_VERSION = Var('gn_version')

NINJA_PACKAGE = f'infra/3pp/tools/ninja/{PLATFORM}'
GN_PACKAGE = f'gn/gn/{PLATFORM}'


def DownloadUrl(url, output_file):
    """Download url into output_file."""
    CHUNK_SIZE = 4096
    num_retries = 3
    retry_wait_s = 5  # Doubled at each retry.

    while True:
        try:
            sys.stdout.write('Downloading %s...' % url)
            sys.stdout.flush()
            response = urlopen(url)
            bytes_done = 0
            while True:
                chunk = response.read(CHUNK_SIZE)
                if not chunk:
                    break
                output_file.write(chunk)
                bytes_done += len(chunk)
            if bytes_done == 0:
                raise URLError("empty response")
            print(' Done.')
            return
        except URLError as e:
            sys.stdout.write('\n')
            print(e)
            if num_retries == 0 or isinstance(e, HTTPError) and e.code == 404:
                raise e
            num_retries -= 1
            print('Retrying in %d s ...' % retry_wait_s)
            sys.stdout.flush()
            time.sleep(retry_wait_s)
            retry_wait_s *= 2


def EnsureDirExists(path):
    if not os.path.exists(path):
        os.makedirs(path)


def DownloadAndUnpack(url, output_dir):
    """Download an archive from url and extract into output_dir."""
    with tempfile.TemporaryFile() as f:
        DownloadUrl(url, f)
        f.seek(0)
        EnsureDirExists(output_dir)
        with zipfile.ZipFile(f, 'r') as z:
            z.extractall(path=output_dir)
            if not is_windows:
                for info in z.infolist():
                    if info.is_dir():
                        continue
                    file = os.path.join(output_dir, info.filename)
                    hi = info.external_attr >> 16
                    if hi:
                        mode = os.stat(file)[ST_MODE]
                        mode |= hi
                        os.chmod(file, mode)


def DownloadCIPD(package, tag, output_dir):
    def get(url):
        parsed = urlparse(url)
        conn = http.client.HTTPSConnection(parsed.netloc)
        conn.request("GET", parsed.path + (f'?{parsed.query}' if parsed.query else ''), headers={"Host": parsed.netloc})
        response = conn.getresponse()
        if response.status != 200:
            raise Exception(f'GET {url} returned {response.status} {response.reason}')
        data = response.read().decode()
        return json.loads(data)

    resolved = get(RESOLVE_URL.format(package, tag))
    instance_id = resolved['instance_id']
    instance = get(INSTANCE_URL.format(package, instance_id))
    DownloadAndUnpack(instance['fetch_url'], output_dir)


def main():
    parser = argparse.ArgumentParser(description='Download ninja/gn binaries.')
    parser.add_argument('--dir', help='Where to extract the package.')
    args = parser.parse_args()

    output_dir = os.path.abspath(args.dir)

    DownloadCIPD(GN_PACKAGE, GN_VERSION, os.path.join(output_dir, 'gn'))
    DownloadCIPD(NINJA_PACKAGE, NINJA_VERSION, os.path.join(output_dir, 'ninja'))



if __name__ == '__main__':
    sys.exit(main())
