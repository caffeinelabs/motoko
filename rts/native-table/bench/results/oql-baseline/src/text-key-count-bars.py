"""Writes text-key-count-bars.tsv from the text-key count records (text-key-counts.md, "Per-key bars").

Run in the folder above src/ after `tar -xJf raw-text-key-counts.tar.xz`:
    python3 src/text-key-count-bars.py > text-key-count-bars.tsv
For each key class and counted value: the lowest of OQL's four count paths (Table and IndexedMap, JSON and
flat key) under the lower of moc 1.14.0 and 1.16.1, and the IndexedMap's flat-key figure under the lower one.
"""
import json
import sys

RUNS = [
    ('nat', 'natidx-{c}/moc{c}nat-s0-100000'),
    ('text17', 'textidx-{c}/moc{c}-s0-100000'),
    ('paths', 'textidx-{c}/moc{c}-s1-100000'),
    ('text1999-1e4', 'textidx-{c}/moc{c}-s2-10000'),
    ('text1999-1e5', 'textidx-{c}/moc{c}-s2-100000'),
]
COUNT_PATHS = ['tJson', 'tFlat', 'mJson', 'mFlat']
SWEEPS = {'sweep even': 'even', 'sweep odd (absent)': 'odd'}


def pick(s):
    return (s * 7919) % 100000


out = sys.stdout
out.write('# OQL count bars per key: class, set, sweep position (-1: a planted key), value, rows holding it, bar '
          '(the lowest count path, lower compiler), its path and compiler, the IndexedMap flat-key figure\n')
out.write('class\tset\tpos\tvalue\tcount\tbar\tpath\tmoc\tindexedmap_flat\n')
even_pos = {pick(s) & ~1: s for s in range(300)}
odd_pos = {pick(1000 + s) | 1: s for s in range(300)}
for cls, pattern in RUNS:
    recs = {}
    for c in ('1140', '1161'):
        with open('raw/' + pattern.format(c=c) + '.queries.jsonl') as f:
            for line in f:
                r = json.loads(line)
                recs.setdefault((r['set'], r['u']), []).append((c, r))
    for (st, u), rs in recs.items():
        if len(rs) != 2 or len({r['value'] for _, r in rs}) != 1:
            raise SystemExit(f'{cls} {st} {u}: not one answer from both compilers')
        bar, path, moc = min((r[p], p, c) for c, r in rs for p in COUNT_PATHS)
        flat = min(r['mFlat'] for _, r in rs)
        kind = SWEEPS.get(st)
        pos = even_pos[u] if kind == 'even' else odd_pos[u] if kind == 'odd' else -1
        name = kind or st.replace(' ', '_')
        out.write(f"{cls}\t{name}\t{pos}\t{u}\t{rs[0][1]['value']}\t{bar}\t{path}\t{moc}\t{flat}\n")
