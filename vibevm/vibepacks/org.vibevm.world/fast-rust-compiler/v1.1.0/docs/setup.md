# Set up fast Rust development on Linux

Use the [setup skill](../vibevm/vibespecs/skills/fast-rust-setup/SKILL.md) after
an explicit setup request. Read the [flow contract](../vibevm/vibespecs/flows/fast-rust-compiler/FAST-RUST-COMPILER.xml)
and [removal guide](remove.md). Execute the concrete authorized plan; do not ask again for permission already supplied. Installing the flow itself is inert.

## 1. Prepare and record the baseline

Select the project, workspace ancestor, Cargo install root and state home from actual owner intent. Inspect Linux distribution/package manager, CPU, available
memory/space, target triple, tools and their versions. Existing rustup and the initial working Rust toolchain are prerequisites, not setup-owned installations.
If either is missing, report that prerequisite and its concrete pending step.

Inspect `CARGO_HOME`, `CARGO_INSTALL_ROOT`, `RUSTUP_TOOLCHAIN`, `RUSTC_WRAPPER`, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_*` and target-specific
variables without logging credentials. Inventory `.cargo/config.toml` and extensionless `.cargo/config` from the project through every ancestor and Cargo
home; inspect `rust-toolchain.toml`/`rust-toolchain`, project profiles, aliases and directory overrides. These read-only commands help establish the baseline:

```bash
rustup show
rustup override list
rustup toolchain list
rustc -vV
cargo -V
cargo install --list
command -v clang mold sccache cargo-nextest
clang --version
mold --version
sccache --version
cargo nextest --version
git -C "$project_root" worktree list --porcelain
git -C "$project_root" rev-parse --path-format=absolute --git-common-dir
findmnt -T "$project_root"
df -h "$project_root"
```

A missing optional executable is an inventory result, not proof of setup failure.
Discover the primary checkout from the first `worktree` record. An explicit warm source override must share the project's Git common directory. Do not infer it
from cwd or a branch name. Verify effective toolchains in each selected directory.

Before any changes, create a unique private receipt directory (reuse the matching receipt on reruns), for example:

```bash
state_home="${state_home:-$HOME/.vibe/fast-rust-compiler}"
mkdir -p "$state_home"
receipt_dir=$(mktemp -d "$state_home/setup.XXXXXXXX")
chmod 700 "$receipt_dir"
```

Persist the concrete plan, scope, prior tool presence/versions, package origins, exact override paths and prior channels/absence, original modified TOML key
values/absence, selected profile and applied values. Record commands, real exits
and effects after every step. Back up safe configuration only; credential-bearing files need scoped key restoration without secret copies. Preserve comments,
unknown keys and aliases. Never blindly replace a file with a template.

Existing manual acceleration without a prior receipt is adopted observation: record what exists now, identify any newly authorized changes, and never invent
historical ownership. Reruns apply only missing or explicitly changed steps.

## 2. Install missing tools

The recommended baseline is stable/LLVM on a supported GNU/Linux target. On Debian/Ubuntu with apt, install only the missing packages selected in the plan:

```bash
sudo -n apt-get update
sudo -n apt-get install -y clang mold
```

For other distributions, inspect available clang/mold packages and the native package manager's documentation; package names/availability differ. Do not
invent an equivalent command. Record a missing package or privilege barrier as an exact pending step, continue independent authorized work, and preserve partial
receipts. Inspect effects before retrying an operation with an unknown outcome.
Use noninteractive sudo to expose a missing privilege grant rather than wait for
a password in an unattended tool call. Record the actual apt transaction,
including newly installed dependencies; do not classify pre-existing packages
as setup-owned. Keep the transaction's proposed scope within the prepared plan.

Use existing tools rather than reinstalling them. For missing Cargo tools, set `install_root` to the selected Cargo install root and ensure its `bin` is on the
current shell's PATH. Persist PATH changes only within the authorized scope.

```bash
# Run outside any accelerated workspace ancestor, before applying its profile.
# If a home/environment config already requires experimental compiler flags,
# prepare a neutral temporary Cargo configuration/home for these tool installs,
# retaining the selected install root and safe reusable registry caches.
cargo +stable install --locked --root "$install_root" sccache
cargo +stable install --locked --root "$install_root" cargo-nextest
export PATH="$install_root/bin:$PATH"
```

If pinning nextest is desired, resolve an actually available version from the primary installation source first, record it, and use the separate command:

```bash
cargo +stable install --locked --root "$install_root" --version "$nextest_version" cargo-nextest
```

Installing these external tools does not authorize a project release build.

## 3. Merge selected local configuration

Prefer `$workspace_root/.cargo/config.toml` in an untracked ancestor outside the repository. Home-wide settings are optional owner opt-in. Review
[Cargo profiles](../templates/cargo-home.toml) and [LLVM target settings](../templates/linux-llvm.toml), then merge selected keys
into the chosen scope with a TOML-aware edit preserving unrelated content:

```toml
[build]
rustc-wrapper = "sccache"
[profile.dev]
debug = "line-tables-only"
split-debuginfo = "unpacked"
[profile.dev.package."*"]
debug = false
opt-level = 1
[profile.dev.build-override]
opt-level = 3
[profile.test]
debug = "line-tables-only"
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Replace the example target table with the observed supported GNU/Linux triple. Do not apply it to all Unix targets. Cargo scalars/profile keys and arrays merge
differently; arrays concatenate across configuration levels. Consolidate owned linker/thread flags in one scope instead of duplicating them in home and ancestor
configs. Inspect environment overrides and closer project settings after editing.
Propose removal of existing repository-local accelerator keys within authorized project scope while preserving portable settings and unrelated content.

Select build jobs from observed resources. Keep workspace incremental compilation unless measurement justifies changing it. Optimized dependencies/build scripts
can increase initial compilation cost while improving repeated execution.

## 4. Optional explicit nightly/Cranelift profile

Use this only when selected by the owner. `nightly_channel` may be `nightly` or an actually verified date-pinned nightly. Record whether the channel/component
already existed; verify target support and actual component availability.

```bash
rustup toolchain install "$nightly_channel"
rustup component add rustc-codegen-cranelift-preview --toolchain "$nightly_channel"
rustup override set "$nightly_channel" --path "$project_root"
```

Record and apply exact overrides separately for every selected worktree when a closer repository toolchain pin would select stable. Never change that pin or the
global rustup default. For an explicitly selected stable directory override use `rustup override set stable --path "$project_root"`, recording its prior state.
If the workspace ancestor itself is explicitly selected for nightly, also record
and set its exact override; a nested project override is a separate record, not
a substitute. A parent-only override can lose to a closer stable toolchain file.
Merge [Cranelift settings](../templates/linux-cranelift.toml) locally:

```toml
[unstable]
codegen-backend = true
[profile.dev]
codegen-backend = "cranelift"
```

Optional `"-Zthreads=8"` is an example of a measured eight-thread profile, not a fixed default; add the tuned flag to the one selected target `rustflags` array
alongside linker flags. Nightly backend, panic/unwind, SIMD and assembly support
changes. Report a failing crate and exact error; do not silently change backend,
weaken tests or claim compatibility. LLVM remains a deliberate selected profile.

## 5. Optional worktree target seed

Every worktree retains a private target; never share `CARGO_TARGET_DIR` or link
another worktree's target. Inspect source and destination mounts/free space using
`findmnt -T` and `df -h`. Windows/NTFS skips warm copying. Ordinary ext4 does not imply cloning support. XFS reflink can make initial clones cheap, but metadata
and future writes still consume resources; its name does not prove clone support. Never repartition, reformat, remount or change filesystem features here.

With no source writer active, probe the actual selected mount pair using tiny files. Variables below are exact verified paths; `new_worktree` already exists.

```bash
reflink_available=false
if probe_source=$(mktemp "$warm_source/.reflink-probe.XXXXXXXX"); then
  if probe_dest=$(mktemp -d "$new_worktree/.reflink-probe.XXXXXXXX"); then
    printf '%s\n' 'clone probe' > "$probe_source"
    if cp --reflink=always "$probe_source" "$probe_dest/probe"; then
      reflink_available=true
    fi
    rm -rf -- "$probe_dest"
  fi
  rm -f -- "$probe_source"
fi
# Record reflink_available and the probe's actual result, not the cleanup exit.
```

After a successful probe and sufficient space, use a unique staging directory.
No live source build, destination writer or existing destination target is allowed.
Serialize publication with worktree users; this recipe assumes that verified state.

```bash
if [ "$reflink_available" = true ] && [ -d "$warm_source/target" ] &&
   [ ! -e "$new_worktree/target" ] && [ ! -L "$new_worktree/target" ]; then
  if stage=$(mktemp -d "$new_worktree/.target-seed.XXXXXXXX"); then
    if cp -a --reflink=always "$warm_source/target" "$stage/target"; then
      if [ ! -e "$new_worktree/target" ] && [ ! -L "$new_worktree/target" ]; then
        mv -T -- "$stage/target" "$new_worktree/target"
      fi
    fi
    rm -rf -- "$stage"
  fi
fi
```

Record copy/publication exits and preserve existing targets. A failed probe/copy means discard only this attempt and build normally; never use ordinary copy.

## 6. Verify and report observed results

Verify effective toolchains from project/worktree directories with `rustup show active-toolchain` and `rustc -vV`. Use a disposable representative
fixture in the chosen configuration scope to verify compilation and clang/mold linking. Measure actual cold/warm elapsed times and capture
`sccache --show-stats` before/after; retain versions, hit/miss/noncacheable deltas
and command exits. Do not clean a project for this measurement. sccache does not cache incremental compilation or linking crate kinds; cache presence alone is
not a speed result.

Run affected `cargo check -p <crate>` and targeted `cargo nextest run -p <crate> <filter>` when nextest exists; otherwise use
`cargo test -p <crate>` with the affected target/filter. Verify a meaningful nonzero case count. Run affected Clippy and the required final completion gate.
Do not implicitly run project release builds, `cargo clean` or `cargo update`. Wait for Cargo locks; do not kill processes or remove lock files.

Report actual scope, timings/counters, partial failures, exact pending privilege steps, receipt path and removal command. Never claim untested native hosts.

Primary references (access date: 2026-10-02): [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html),
[rustup overrides](https://rust-lang.github.io/rustup/overrides.html), [Cranelift](https://github.com/rust-lang/rustc_codegen_cranelift),
[sccache](https://github.com/mozilla/sccache), [nextest source installation](https://nexte.st/docs/installation/from-source/),
[mold](https://github.com/rui314/mold).
