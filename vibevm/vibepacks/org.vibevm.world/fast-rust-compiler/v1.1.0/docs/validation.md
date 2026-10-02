# Package validation — 2026-10-02

This validates instruction delivery, not a complete workstation provisioning
run or a measured compiler speedup.

- `vibe check --path <package> --offline --json`: zero errors and warnings.
- XML specifications and TOML manifest/templates parsed; guide links resolve.
- All fenced Bash examples passed `bash -n`; provisioning examples were not run.
- A disposable consumer installed the flow through ordinary offline `vibe install`.
  The portable rules appeared in generated STATIC. The Linux fragment appeared
  only as a dynamic INDEX entry with `when = "os:linux"`.
- Explicit source descriptors preserved Markdown skills/guides and configuration
  templates byte-for-byte in an XML-lane consumer. Without these descriptors,
  the first probe converted SKILL.md into SKILL.xml and the promised skill path
  was absent; the descriptor correction closed that concrete failure.
- Ordinary `vibe skill install` projected both named skills for Codex. Projected
  directories contain skill files, so their instructions resolve guides through
  the selected workspace's exact locked package slot.
- The host installed the flow through the same ordinary materialization path.
  Existing dependency hashes and fields were preserved; their local source URLs
  were refreshed after checkout relocation. No system setup hooks exist.
- Repository Cargo configuration retained its portable xtask alias and stable
  toolchain. An isolated offline stable Cargo metadata check read all 41 workspace
  members, each declaring Rust 1.93, without local accelerator configuration.

Full apt/rustup/Cargo provisioning and removal, native macOS/Windows execution,
compiler compatibility and performance remain checks for the explicitly selected
workstation/profile. OS predicates are delivery evidence, not native build proof.
