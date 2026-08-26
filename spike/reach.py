#!/usr/bin/env python3
# Reachability from the start function to a target import in a wasm-objdump -d dump.
# Usage: reach.py <dis-file> <start-func-index> <target-name-substring>
import re, sys
from collections import defaultdict, deque

dis, start_idx, target = sys.argv[1], int(sys.argv[2]), sys.argv[3]
calls = defaultdict(set)   # caller idx -> set of (callee idx, callee name)
names = {}
cur = None
func_re = re.compile(r'^[0-9a-f]+ func\[(\d+)\](?: <([^>]*)>)?:')
call_re = re.compile(r'\| call (\d+)(?: <([^>]*)>)?')
ind_re = re.compile(r'\| call_indirect')
indirect_callers = set()
for line in open(dis, errors='replace'):
    m = func_re.match(line)
    if m:
        cur = int(m.group(1)); names[cur] = m.group(2) or f'f{cur}'
        continue
    if cur is None: continue
    m = call_re.search(line)
    if m:
        callee = int(m.group(1)); cname = m.group(2) or f'f{callee}'
        names.setdefault(callee, cname)
        calls[cur].add(callee)
    elif ind_re.search(line):
        indirect_callers.add(cur)

targets = {i for i, n in names.items() if target in n}
# BFS from start, record parent for path reconstruction
parent = {start_idx: None}
q = deque([start_idx])
found = None
while q:
    u = q.popleft()
    if u in targets:
        found = u; break
    for v in sorted(calls.get(u, ())):
        if v not in parent:
            parent[v] = u; q.append(v)
if found is None:
    print(f"UNREACHABLE from func[{start_idx}] (direct calls only; "
          f"{len(indirect_callers & set(parent))} reachable funcs use call_indirect)")
else:
    path = []
    n = found
    while n is not None:
        path.append(f"func[{n}] <{names.get(n)}>")
        n = parent[n]
    print("PATH (start -> target):")
    for p in reversed(path):
        print("  " + p)
