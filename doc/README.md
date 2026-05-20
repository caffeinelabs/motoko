# Motoko documentation

Source for the Motoko language documentation at
[https://docs.internetcomputer.org/languages/motoko/](https://docs.internetcomputer.org/languages/motoko/).

Content lives in `doc/md/` and is synced to the consumer site via a plain rsync; no post-processing pipeline needed.

## Previewing locally

`doc/site/` is a self-contained Starlight site that renders `doc/md/` with the same navigation and ICP brand styling as the consumer site.

```bash
make preview
# open http://localhost:4321
```

To validate the build (catches broken file embeds and config errors):

```bash
make build
```

## Code fence conventions

| Fence | Meaning |
|---|---|
| ```` ```motoko no-repl ```` | Static syntax-highlighted Motoko code (most common) |
| ```` ```motoko file=<motokoExamples>/foo.mo ```` | Embed a `.mo` example file from `doc/md/examples/` |
| ```` ```motoko file=<motokoExamples>/foo.mo#L10-L20 ```` | Embed specific lines from an example file |
| ```` ```md file=<motokoRoot>/Changelog.md ```` | Inline the root `Changelog.md` as rendered prose |

`<motokoExamples>` resolves to `doc/md/examples/`.
`<motokoRoot>` resolves to the repository root (where `Changelog.md` lives).

Both placeholders are resolved by `doc/site/plugins/remark-include-file.mjs` locally and by the consumer site's equivalent plugin at build time.

