"""
This script sets up re_client sort of like https://chromium.googlesource.com/chromium/src/+/main/docs/linux/build_instructions.md#use-reclient

You will need to set these gn args:
```
use_remoteexec=true
reclient_cfg_dir="../../buildtools/reclient_cfgs/linux"
cc_wrapper=""
```

and set these env vars:
```
NINJA=autoninja
```
"""

from v8_deps import Var, hooks

import subprocess
import os

def run(name):
    hook = next(h for h in hooks if h['name'] == name)
    print(subprocess.run(hook['action']))

run('configure_reclient_cfgs')
run('configure_siso')

rbe_version = Var('reclient_version')

ensure_file = f'''
$ParanoidMode CheckPresence
@Subdir buildtools/reclient
infra/rbe/client/linux-amd64 {rbe_version}
'''
print(ensure_file)
with open("./cipd.ensure", "w") as f:
    f.write(ensure_file)
print(subprocess.run(['cipd', 'ensure', '-root', '.', '-ensure-file', 'cipd.ensure']))
os.remove('./cipd.ensure')
