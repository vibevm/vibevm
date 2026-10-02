---
name: fast-rust-uninstall
description: Undo receipt-owned fast Rust workstation settings and newly installed tools after an explicit owner request, preserving shared installations, user work and later configuration edits.
---

# Remove fast Rust setup

Use this skill when the owner explicitly asks to remove or undo fast Rust setup,
including “убери настройку fast-rust”. Removing a VibeVM flow alone never runs
system cleanup. Resolve the selected project's workspace lockfile, match the
exact `org.vibevm.world/fast-rust-compiler` entry and verify that version's
materialized slot under the workspace's `vibevm/vibedeps/`. A projected skill
does not contain the entire package; do not resolve guides relative to it.
If the package is already absent, use an owner-bound retained copy and receipt,
or report the missing removal instructions before dependent actions. Read its
`vibevm/vibespecs/flows/fast-rust-compiler/FAST-RUST-COMPILER.xml`,
`docs/remove.md`, and the matching user-local setup receipt.

1. Resolve the exact configuration scope and receipts. Inspect current tools,
   overrides, relevant config keys and active builds. If no baseline predates
   existing settings, report unknown ownership and ask for the specific removal
   scope. Never invent a clean machine before-state.
2. Prepare the precise reverse plan. Compare current values with recorded applied
   values; preserve later unrelated edits. Resolve meaningful conflicting drift
   before overwriting a key. Remove only owned configuration values/blocks or
   restore their recorded previous values, preserving aliases and comments.
3. Remove wrapper/linker/backend references before tools. Restore each recorded
   directory rustup override, including nested worktrees, to its actual prior
   channel or absence. Do not switch the global toolchain default.
4. Run the concrete `docs/remove.md` commands only for tools/components installed
   by this setup and now confirmed unused elsewhere. Existing sccache, nextest,
   clang, mold and nightly installations remain shared. Stop a cache service only
   when owned and no active work needs it. Do not kill builds or delete lock files.
5. Do not run apt autoremove, rustup self uninstall, blanket cache removal,
   cargo clean, filesystem changes or worktree/source deletion. Local caches and
   receipts remain unless separately selected by the owner for removal.
6. Record real exits and any partial/pending steps. Verify portable repository
   config, restored effective toolchains and absence of dangling wrapper/linker
   references. Retain a completion receipt; report remaining shared tools honestly.
7. If the owner also asks to remove the instruction package, use ordinary
   `vibe uninstall` / `vibe skill uninstall` for the package and its projections,
   after preserving the system-removal guide and receipts. No hand-written edits
   to generated boot lanes or unrelated agent instructions.

An explicit removal command authorizes the concrete receipt-owned reverse plan;
do not request the same permission again. A privilege/input/drift barrier stops
only its affected step with a precise report.
