#!/usr/bin/env python3
import os
import sys
import json

log_file = os.getenv('CC_HOOK_COMPDB_LOG_FILE')

if log_file is None:
    log_file = 'cc_hook.txt'

# todo keep hook log out of tree
args = sys.argv[1:]

wd = os.getcwd()
with open(log_file, 'a') as f:
    f.write(json.dumps({
        'wd': wd,
        'args': args,
    }))
    f.write('\n')

cc = '/usr/bin/gcc'
os.execve(cc, [cc] + args, os.environ)