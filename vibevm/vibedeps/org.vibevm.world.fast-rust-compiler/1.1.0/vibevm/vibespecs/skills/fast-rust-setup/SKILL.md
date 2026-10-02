---
name: fast-rust-setup
description: Configure a Linux workstation and selected project for fast Rust development when the owner explicitly requests setup; install tools, apply local profiles and verify them without changing portable repository requirements.
---

# Fast Rust setup

Use this skill for an explicit owner request to set up fast Rust development,
including “настрой компьютер и проект для быстрой Rust-разработки”. Installing
the flow itself never fires this procedure. An execution request means do the
authorized work, not merely display commands.

Resolve the selected project's workspace lockfile and its exact package entry
for `org.vibevm.world/fast-rust-compiler`. Locate that version's materialized
slot under the workspace's `vibevm/vibedeps/` and verify its manifest identity.
A projected agent skill directory contains the skill, not the entire package;
do not resolve guides by walking relative to the projected skill. If the flow
is missing, materialize it with ordinary VibeVM installation before setup.
Read its `vibevm/vibespecs/flows/fast-rust-compiler/FAST-RUST-COMPILER.xml`,
`docs/setup.md`, `docs/remove.md`, and the three `templates/*.toml` files.
Their package-relative paths survive installation; do not assume a host checkout.

1. Confirm Linux and the selected project, then inventory rustup/tool versions,
   package manager, Cargo home/install root, configuration hierarchy, directory
   overrides and environment overrides. Discover primary worktree through Git.
   Preserve an explicitly selected same-repository warm source as a local override.
2. Inspect source/destination mounts, filesystem type, free space, CPU and memory.
   Probe actual clone support with tiny disposable files. XFS is not sufficient
   evidence by itself; ordinary ext4 usually cannot take this path; skip warm
   copying on Windows. Never provision or change a filesystem for this procedure.
3. Prepare a concrete local plan and baseline receipt under
   `~/.vibe/fast-rust-compiler/` or the owner's selected state home, privately.
   Record original tool presence/versions, modified key values or absence, exact
   override paths/channels, selected profile and owned configuration scope.
   Do not persist credentials. Missing roots/profile intent are questions, not
   permission requests for setup already authorized by the owner.
4. Prefer the stable/LLVM Linux profile unless the owner selected the optional
   nightly/Cranelift profile. The latter requires actual component availability
   and target checks. Select build parallelism from measured resources; do not
   impose fixed thread counts or disable incremental builds to claim cache hits.
5. Execute the selected steps in `docs/setup.md`: supported package-manager
   installation of clang/mold, stable Cargo installation of sccache/nextest,
   optional nightly/component installation and exact directory overrides,
   reviewed local TOML edits. Use existing tools rather than reinstalling them.
   Do not commit machine settings or change the global default toolchain.
   Ask only when a required input, scope conflict or actual privilege barrier
   prevents a concrete step; stop affected work at that barrier and retain receipts.
6. Persist each command's real exit and applied effects before moving on. On
   partial failure, inspect effects before retrying; never duplicate an unknown
   package-manager/configuration operation. Preserve aliases, unknown keys,
   comments and unrelated settings. Cargo rustflags arrays merge: consolidate
   owned flags rather than append duplicate linker or thread switches.
7. Verify active toolchains in the selected project/worktrees, compiler/linker
   behavior on a disposable fixture, targeted nonzero tests, cache statistics
   and cold/warm timings with exact versions. A cache install alone is not speed
   evidence. Backend failures retain crate/error and require an explicit profile
   decision; do not silently switch settings or weaken tests.
8. Install/project the flow's agent rules and skills only through ordinary VibeVM
   package/skill commands when needed; preserve user-local machine selections.
   Conclude with observed setup scope, limitations, receipt path and the natural
   removal command “убери настройку fast-rust”. Never claim unrun native hosts.

This skill supplies instructions to the executing agent. It adds no automatic
apt hook, privileged service, system-control backend or repository compiler CLI.
