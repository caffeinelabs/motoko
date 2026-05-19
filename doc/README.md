# Building the documentation locally

You can build the basic documentation locally as follows:

```
make
python3 -m http.server --directory html
# now open http://0.0.0.0:8000/motoko.html
```

CI pushes these docs for latest master to
[https://hydra.dfinity.systems/job/dfinity-ci-build/motoko/docs/latest/download/1/overview-slides.html](https://hydra.dfinity.systems/job/dfinity-ci-build/motoko/docs/latest/download/1/overview-slides.html).

## Published documentation

The canonical Motoko language documentation is published at
[https://docs.internetcomputer.org/languages/motoko/](https://docs.internetcomputer.org/languages/motoko/)
and synced from `doc/md/` in this repository.

The documentation uses [Starlight](https://starlight.astro.build/) on the consumer site.
Code fences use `no-repl` to indicate static syntax-highlighted blocks.

## Code fence conventions

| Fence | Meaning |
|---|---|
| ```` ```motoko no-repl ```` | Static syntax-highlighted Motoko code (most common) |
| ```` ```motoko file=<motokoExamples>/foo.mo ```` | Embed a `.mo` example file from `doc/md/examples/` |
| ```` ```md file=<motokoRoot>/Changelog.md ```` | Inline the root `Changelog.md` as rendered prose |

`<motokoExamples>` and `<motokoRoot>` are placeholders resolved by the consumer site's build plugin.
