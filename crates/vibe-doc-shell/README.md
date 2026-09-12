# vibe-doc-shell

The documentation reader's shell — one type over three sources.

A page a person reads is the **island** (rendered by `vibe-doc` out of the
documentation package) inside the **shell** (a build of the site package:
head, styles, navigation, behaviour). This crate owns the second half, and
the only question it answers is *where are the bytes*:

- **embedded** — compiled in through `include_dir` behind the
  `embedded-shell` feature, which a release build turns on;
- **store** — downloaded with explicit consent by `vibe doc shell install`
  into `<install root>/vibevm/doc-shell/<sha256>/`;
- **fallback** — the bare shell in `src/fallback.rs`: typography, no
  scripts, still readable.

`Shell::provenance()` is what `vibe doc serve --print-shell` reports.

## Building the embedded shell

```
cargo xtask embed-doc-shell          # pnpm build:embedded, copy, re-pin
cargo build -p vibe-cli --features vibe-doc-shell/embedded-shell
```

Without the first step the second refuses, because neither `include_dir`
nor `rust-embed` can tell an empty directory from a full one and a release
binary that embedded nothing would only be found by a reader.

`shell/` is a build product and is not committed. `doc-shell.lock` is: it
pins the web package's coordinate, its version and the digest of the shell
built from it, and `vibe doc shell status` compares that pin against a
fresh measurement of the bytes the running binary is carrying.

Norm: PROP-057 `##SHELL-XTASK-EMBED`, `##SHELL-RELEASE-BUILD`,
`##SHELL-INSTALL-COMMAND`, `##SHELL-SERVE-SOURCES`, `##SHELL-PIN`,
`##SHELL-RELATIVE`.
