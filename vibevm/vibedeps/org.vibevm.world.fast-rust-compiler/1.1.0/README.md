# fast-rust-compiler 1.1.0

A VibeVM flow for portable Rust worktree discipline and explicitly requested
Linux workstation acceleration. Installing the flow materializes instructions;
it never installs software, changes Cargo settings or runs system cleanup.

Install from a selected local package registry, then project the declared skills
using ordinary VibeVM commands (no system setup runs during either step):

```bash
vibe install flow:org.vibevm.world/fast-rust-compiler@1.1.0 --registry "$registry_root" --offline
vibe skill install --skill fast-rust-setup --skill fast-rust-uninstall --scope project
```

This source package is not a claim of public registry publication. Projected
skills resolve guides from the selected workspace's locked materialized slot.

Ask an agent with the installed skills to **“set up fast Rust development”**
(or **“настрой компьютер и проект для быстрой Rust-разработки”**). The agent
inventories the selected scope, records a local baseline, executes authorized
steps and verifies their actual effects. To undo those effects, request
**“remove fast Rust setup”** or **“убери настройку fast-rust”**.

- [Setup recipe](docs/setup.md): stable/LLVM baseline, optional nightly/Cranelift,
  local configuration, filesystem probes and verification.
- [Removal recipe](docs/remove.md): restore receipt-owned settings before
  removing newly installed, unused tools.
- [Setup skill](vibevm/vibespecs/skills/fast-rust-setup/SKILL.md) and
  [removal skill](vibevm/vibespecs/skills/fast-rust-uninstall/SKILL.md).
- [Authoritative flow contract](vibevm/vibespecs/flows/fast-rust-compiler/FAST-RUST-COMPILER.xml).
- Templates: [Cargo profiles](templates/cargo-home.toml),
  [GNU/Linux LLVM](templates/linux-llvm.toml),
  [optional Cranelift](templates/linux-cranelift.toml).

The default development path uses stable Rust, clang/mold, sccache and nextest
where supported. Local settings preferably live in a selected untracked ancestor
of the project. Home-wide settings require explicit opt-in. Preserve the
repository's portable configuration and toolchain pin; a closer toolchain file
may require an exact directory override for the project and each worktree.

Every worktree keeps its own `target/`. An optional, proven filesystem reflink
can seed a new target; failed clones never fall back to ordinary full copies.
Use affected `cargo check`, targeted tests with a meaningful nonzero case count,
and affected Clippy checks. Preserve the required final completion gate.
Project release builds, cleaning and dependency updates require explicit requests.

Setup receipts live privately under the selected user state home, normally
`~/.vibe/fast-rust-compiler/`. Existing installations remain shared observations.
Removal preserves user caches, source, worktrees and unrelated settings. Removing
this instruction package alone does not undo machine configuration.

Package: `org.vibevm.world/fast-rust-compiler@1.1.0`; license: UPL-1.0.
