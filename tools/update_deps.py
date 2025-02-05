from v8_deps import deps
import subprocess

def process(name, dep):
    if name == 'build':
        # We have our own fork of this
        return

    url = dep if isinstance(dep, str) else dep['url']
    rev = url.split('@')[1]
    print(name, rev)
    subprocess.run(['git', 'fetch', 'origin'], cwd=name)
    subprocess.run(['git', 'checkout', rev], cwd=name)

failed = False

names = []

with open('.gitmodules') as f:
    for line in f.readlines():
        if line.startswith('['):
            name = line.split(" ")[1][1:-3]
            if name in deps:
                names.append(name)
                try:
                    process(name, deps[name])
                except:
                    failed = True

if failed:
    import sys
    sys.exit(1)

print(','.join(names))
