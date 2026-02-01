import os
import sys
import json

log_file = sys.argv[1]

ls = open(log_file).readlines()
dst = 'compile_commands.json'
db = []
for l in ls:
    it = json.loads(l)
    wd = it['wd']
    args = ['/usr/bin/gcc'] + it['args']
    srcs = []
    for a in args:
        if a.endswith('.c') or a.endswith('.cc') or a.endswith('.cpp'):
            srcs.append(os.path.join(wd, a))
    if len(srcs) > 0:
        db.append({
            'directory': wd,
            'arguments': args,
            'file': srcs[-1],
        })
    else:
        print(f'warning no src {l}')
with open(dst, 'w') as f:
    f.write(json.dumps(db))