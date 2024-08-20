Str = str
def Var(name):
    return vars[name]
with open('./v8/DEPS') as f:
    exec(f.read())

import subprocess

def process(name, dep):
    if name == 'build' or name == 'third_party/icu':
        # We have our own fork of this
        return

    url = dep if isinstance(dep, str) else dep['url']
    rev = url.split('@')[1]
    print(name, rev)
    subprocess.run(['git', 'fetch', 'origin'], cwd=name)
    subprocess.run(['git', 'checkout', rev], cwd=name)

failed = False

with open('.gitmodules') as f:
    for line in f.readlines():
        if line.startswith('['):
            name = line.split(" ")[1][1:-3]
            if name in deps:
                try:
                    process(name, deps[name])
                except:
                    failed = True

if failed:
    import sys
    sys.exit(1)

