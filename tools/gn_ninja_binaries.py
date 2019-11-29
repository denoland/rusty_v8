#!/usr/bin/env python
# Copyright (c) 2012 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.
"""This script is used to download prebuilt gn/ninja binaries."""

# TODO: Running stand-alone won't work on Windows due to the dia dll copying.

from __future__ import division
from __future__ import print_function
import argparse
import os
import shutil
import stat
import sys
import tarfile
import tempfile
import time

try:
    from urllib2 import HTTPError, URLError, urlopen
except ImportError:  # For Py3 compatibility
    from urllib.error import HTTPError, URLError
    from urllib.request import urlopen

import zipfile

URL = "https://s3.amazonaws.com/deno.land/gn_ninja_binaries.tar.gz"
THIS_DIR = os.path.abspath(os.path.dirname(__file__))
DIR = None


def RmTree(dir):
    """Delete dir."""

    def ChmodAndRetry(func, path, _):
        # Subversion can leave read-only files around.
        if not os.access(path, os.W_OK):
            os.chmod(path, stat.S_IWUSR)
            return func(path)
        raise

    shutil.rmtree(dir, onerror=ChmodAndRetry)


def DownloadUrl(url, output_file):
    """Download url into output_file."""
    CHUNK_SIZE = 4096
    TOTAL_DOTS = 10
    num_retries = 3
    retry_wait_s = 5  # Doubled at each retry.

    while True:
        try:
            sys.stdout.write('Downloading %s ' % url)
            sys.stdout.flush()
            response = urlopen(url)
            total_size = int(response.info().get('Content-Length').strip())
            bytes_done = 0
            dots_printed = 0
            while True:
                chunk = response.read(CHUNK_SIZE)
                if not chunk:
                    break
                output_file.write(chunk)
                bytes_done += len(chunk)
                num_dots = TOTAL_DOTS * bytes_done // total_size
                sys.stdout.write('.' * (num_dots - dots_printed))
                sys.stdout.flush()
                dots_printed = num_dots
            if bytes_done != total_size:
                raise URLError(
                    "only got %d of %d bytes" % (bytes_done, total_size))
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


def DownloadAndUnpack(url, output_dir, path_prefixes=None):
    """Download an archive from url and extract into output_dir. If path_prefixes
     is not None, only extract files whose paths within the archive start with
     any prefix in path_prefixes."""
    with tempfile.TemporaryFile() as f:
        DownloadUrl(url, f)
        f.seek(0)
        EnsureDirExists(output_dir)
        if url.endswith('.zip'):
            assert path_prefixes is None
            zipfile.ZipFile(f).extractall(path=output_dir)
        else:
            t = tarfile.open(mode='r:gz', fileobj=f)
            members = None
            if path_prefixes is not None:
                members = [
                    m for m in t.getmembers() if any(
                        m.name.startswith(p) for p in path_prefixes)
                ]
            t.extractall(path=output_dir, members=members)


def Update():
    if os.path.exists(DIR):
        RmTree(DIR)
    try:
        DownloadAndUnpack(URL, DIR, None)
    except URLError:
        print('Failed to download prebuilt ninja/gn binaries %s' % URL)
        print('Exiting.')
        sys.exit(1)

    return 0


def main():
    parser = argparse.ArgumentParser(description='Download ninja/gn binaries.')
    parser.add_argument('--dir', help='Where to extract the package.')
    args = parser.parse_args()

    if args.dir:
        global DIR
        DIR = os.path.abspath(args.dir)

    return Update()


if __name__ == '__main__':
    sys.exit(main())
