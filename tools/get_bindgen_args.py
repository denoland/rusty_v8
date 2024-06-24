import argparse
import json
import os

parser = argparse.ArgumentParser(description='Generate args for bindgen')
parser.add_argument('--gn-out', help='GN out directory')
args = parser.parse_args()

with open(os.path.join(args.gn_out, 'project.json')) as project_json:
    project = json.load(project_json)

target = project['targets']['//v8:v8_headers']

assert '//v8:cppgc_headers' in target['deps']

args = []

for define in target['defines']:
    args.append(f'-D{define}')

print('\0'.join(args), end="")
