#!/usr/bin/env python
# Copyright (c) 2012 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

from __future__ import print_function
import argparse
import os
import sys

try:
    from urllib2 import HTTPError, URLError, urlopen
except ImportError:  # For Py3 compatibility
    from urllib.error import HTTPError, URLError
    from urllib.request import urlopen

def DownloadUrl(url, output_file):
    """Download url into output_file."""
    CHUNK_SIZE = 4096
    TOTAL_DOTS = 10
    num_retries = 3
    retry_wait_s = 5  # Doubled at each retry.

    while True:
        try:
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
            print('Done.')
            return
        except URLError as e:
            sys.stdout.write('\n')
            print(e)
            if num_retries == 0 or isinstance(e, HTTPError) and e.code == 404:
                raise e
            num_retries -= 1
            print('Retrying in %d s ...' % retry_wait_s)
            sys.stdout.flush()
            retry_wait_s *= 2


def main():
    parser = argparse.ArgumentParser(description='Download librusty_v8.a')
    parser.add_argument('--filename', help='where to put binary')
    parser.add_argument('--url', help='what url to download')
    args = parser.parse_args()
    print("url", args.url)
    print("filename", args.filename)
    with open(args.filename, "wb") as f:
        DownloadUrl(args.url, f)
    return 0


if __name__ == '__main__':
    sys.exit(main())
