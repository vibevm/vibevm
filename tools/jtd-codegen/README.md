# jtd-codegen — vendored locally

`jtd-codegen` generates strictly-typed, language-specific code from JTD
([JSON Type Definition, RFC 8927](https://www.rfc-editor.org/rfc/rfc8927))
schemas. We use it to derive Rust types from the project's JTD schemas: the wire
contracts in [`schemas/`](../../schemas/) at the repo root generate into
[`crates/vibe-wire/src/generated/`](../../crates/vibe-wire/src/generated/),
and the specmap schema inside the `core-ai-native` package generates into
that package's own engine crate (`core-ai-native-specmap`).

Per [PROP-000 §16](../../spec/common/PROP-000.md#jtd) JTD is the source of
truth for every wire contract in the project; per [PROP-000 §15](../../spec/common/PROP-000.md#dep-weight) we
do not minimise tooling weight. Pick the right binary, vendor it
locally, run it from the project's `cargo xtask codegen` task.

## Pinned version

`jtd-codegen 0.4.1` — the most recent stable release at the time of
writing. Bump by editing this README; CI is the place to assert
schemas don't drift, not to enforce a particular generator version.

Upstream: <https://github.com/jsontypedef/json-typedef-codegen>

## Install

Drop the appropriate platform binary at `tools/jtd-codegen/jtd-codegen`
(or `jtd-codegen.exe` on Windows). The `tools/` directory's
`.gitignore` keeps the binary out of git — only this `README.md`
travels with the repo.

### Windows

```sh
# From repo root, in PowerShell or Git Bash:
curl -LO https://github.com/jsontypedef/json-typedef-codegen/releases/download/v0.4.1/x86_64-pc-windows-gnu.zip
unzip -d tools/jtd-codegen x86_64-pc-windows-gnu.zip
rm x86_64-pc-windows-gnu.zip
```

(Upstream ships a `gnu` build, not `msvc` — the static binary works on all Windows hosts the project targets.)

### macOS

```sh
# Apple Silicon:
curl -L https://github.com/jsontypedef/json-typedef-codegen/releases/download/v0.4.1/aarch64-apple-darwin.tar.gz \
  | tar -xz -C tools/jtd-codegen
# Intel:
# curl -L .../v0.4.1/x86_64-apple-darwin.tar.gz | tar -xz -C tools/jtd-codegen
```

### Linux

```sh
curl -L https://github.com/jsontypedef/json-typedef-codegen/releases/download/v0.4.1/x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz -C tools/jtd-codegen
```

### Verify

```sh
tools/jtd-codegen/jtd-codegen --version    # prints "jtd-codegen 0.4.1"
```

## Use

```sh
cargo xtask codegen           # regenerate every Rust target from schemas/
cargo xtask check-codegen     # regenerate then assert no diff (CI uses this)
```

The xtask preflights the binary location and emits an actionable error
if it is missing — pointing at this file.
