# jtd-codegen — install target; the recipe is a package

The `jtd-codegen` binary is dropped into this directory (`jtd-codegen`, or
`jtd-codegen.exe` on Windows) and is never committed — `tools/.gitignore`
keeps every toolchain binary out of git.

The tool's canonical home — the pinned upstream version, the per-platform
install commands, and the use notes — is the package
**`tool:org.vibevm.ai-native/jtd-codegen`**:
[`vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md`](../../packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md).
Install per that recipe; do not restate the version pin here — one pin,
one home.

In this repository the binary is driven by `cargo xtask codegen` /
`cargo xtask check-codegen`: the wire contracts in `schemas/` at the repo
root generate into `crates/vibe-wire/src/generated/`, and the specmap
schema inside the `core-ai-native` package generates into that package's
engine crate (`core-ai-native-specmap`).
