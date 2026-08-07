from v8_deps import deps
import subprocess
import sys


def run_git(args, cwd=None):
    subprocess.run(['git', *args], cwd=cwd, check=True)


def process(name, dep):
    url = dep if isinstance(dep, str) else dep['url']
    rev = url.rsplit('@', 1)[1]
    print(name, rev)

    run_git(['fetch', 'origin'], cwd=name)
    if name == 'build':
        # The build submodule carries rusty_v8-specific patches, so it cannot
        # be checked out directly at Chromium's revision. Its current commit
        # must instead contain that revision in its ancestry.
        result = subprocess.run(
            ['git', 'merge-base', '--is-ancestor', rev, 'HEAD'],
            cwd=name,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f'build does not contain Chromium revision {rev}; '
                'rebase denoland/chromium_build onto it first'
            )
        return

    run_git(['checkout', '--detach', rev], cwd=name)

failed = []
names = []

with open('.gitmodules') as f:
    for line in f.readlines():
        if line.startswith('['):
            name = line.split(" ")[1][1:-3]
            if name in deps:
                try:
                    if name != 'build':
                        run_git([
                            'submodule',
                            'update',
                            '--init',
                            '--checkout',
                            '--',
                            name,
                        ])
                    process(name, deps[name])
                    names.append(name)
                except (
                    OSError,
                    RuntimeError,
                    subprocess.CalledProcessError,
                ) as error:
                    print(f'Failed to update {name}: {error}', file=sys.stderr)
                    failed.append(name)

if failed:
    print(f'Failed dependencies: {", ".join(failed)}', file=sys.stderr)
    sys.exit(1)

print(','.join(names))
