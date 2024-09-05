Str = str
def Var(name):
    if name == 'rbe_instance':
        return 'projects/rbe-chromium-untrusted/instances/default_instance'
    return vars[name]
with open('./v8/DEPS') as f:
    exec(f.read())

