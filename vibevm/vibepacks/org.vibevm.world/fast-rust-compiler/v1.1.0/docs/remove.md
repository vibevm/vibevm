# Remove fast Rust setup

After an explicit removal request, use the
[removal skill](../vibevm/vibespecs/skills/fast-rust-uninstall/SKILL.md), the
[flow contract](../vibevm/vibespecs/flows/fast-rust-compiler/FAST-RUST-COMPILER.xml)
and the matching local setup receipt. Flow removal alone never runs system
cleanup. Execute the receipt-owned reverse plan without requesting the same
permission again.

## 1. Prepare the exact reverse plan

Resolve the selected project, configuration scopes, Cargo install root, nightly
channel, exact override paths and receipt directory under the selected state home
(normally `~/.vibe/fast-rust-compiler/`). Inspect current tool versions, config
hierarchy/environment, overrides and active builds. Existing installations from
the setup baseline are shared and remain installed.

Compare each current modified key to its recorded applied value and each
current override to its applied channel. Preserve unrelated later edits, aliases,
comments and unknown TOML keys. Conflicting drift needs a specific owner decision
before that value is overwritten; continue independent safe removal steps.

If no receipt predates existing manual setup, report unknown ownership. A current
inventory is not a historical baseline. Obtain the specific removal scope rather
than inventing prior values, tool ownership or an empty previous configuration.

Record the reverse plan before changes. Retain real command exits, applied
restorations, partial failures and pending steps after each operation. Never
include credentials in reports or copy credential-bearing files wholesale.

## 2. Restore configuration before removing tools

Use a TOML-aware scoped edit to restore each original value or remove a key
whose recorded prior state was absence. First undo owned `build.rustc-wrapper`,
target linker/linker flags, optional compiler thread/backend keys and profiles.
Use a full safe backup only when the current file is exactly the applied version
and the restore cannot erase later edits. Do not blindly overwrite whole files.
Remove a newly created file/directory only if its contents remain exclusively
setup-owned; otherwise preserve the file and restore owned keys individually.

Check home, selected ancestor, project and extensionless Cargo configs plus
environment settings. Arrays merge: remove only receipt-owned flags, preserving
other flags and order. Restore recorded PATH/environment changes in the exact
owned scope. Confirm no remaining wrapper/linker/backend reference will require
a tool selected for removal. Do this before uninstalling any executable.

Restore directory rustup overrides individually. If the receipt says the exact
path had no prior override, and its current value matches the applied channel:

```bash
rustup override unset --path "$override_path"
```

If it had a prior channel, restore that exact recorded channel instead:

```bash
rustup override set "$prior_channel" --path "$override_path"
```

Repeat for the recorded project and each worktree path, including nested paths.
Do not blanket-unset overrides or change the global rustup default. Verify the
effective toolchain from each directory with `rustup show active-toolchain` and
`rustc -vV`; closer toolchain files and environment variables affect selection.

## 3. Remove only newly installed, now-unused tools

For every candidate, prove setup installed it and inspect use by other projects,
configuration, processes and receipts. An existing tool, component or toolchain
is shared even if setup used it. Do not infer exclusive ownership from its name.
When unused status is uncertain, leave it installed and report that fact.

If this setup owns the sccache server and no active work needs it, stop it before
uninstalling the owned executable:

```bash
sccache --stop-server
```

A server may be shared across projects. Do not stop it merely because one project
is being restored, and never kill builds or delete Cargo lock files.

For newly installed Cargo tools now confirmed unused, use the recorded install
root (the executable's package name differs from the nextest Cargo subcommand):

```bash
cargo uninstall --root "$install_root" sccache
cargo uninstall --root "$install_root" cargo-nextest
```

Run only the applicable command(s). Preserve installations present in the
baseline, including versions setup observed/adopted. Installation upgrades do not
justify deleting an originally shared tool; restore only an explicitly recorded
and authorized version change using its actual prior version/source.

For an optional component newly installed on an existing or new selected channel,
remove it only after confirming no other project/override needs it:

```bash
rustup component remove rustc-codegen-cranelift-preview --toolchain "$nightly_channel"
```

A selected toolchain may be removed only if setup newly installed it and it is
now unused by overrides, defaults, projects and active work:

```bash
rustup toolchain uninstall "$nightly_channel"
```

Never uninstall pre-existing rustup, an initial/shared stable toolchain or a
pre-existing nightly. Never run `rustup self uninstall`.

For Debian/Ubuntu apt packages newly installed by setup and confirmed unused,
remove only applicable package names from the recorded plan:

```bash
sudo -n apt-get --simulate remove mold clang
# After reviewing the simulated transaction against the receipt and current use:
sudo -n apt-get remove -y mold clang
```

If only one package is owned/unused, pass only that package. Inspect the package
manager's proposed removal transaction and stop if it includes unrelated needed
packages. Review setup-installed dependencies individually as well; remove them
only when receipt-owned installation and current unused status are established.
Other distributions require their documented native removal command; do not
invent one. Do not run `apt autoremove`. A privilege barrier leaves the
exact affected command pending; record it and continue independent authorized
restorations. Inspect effects before retrying a command with an unknown outcome.

## 4. Verify and retain evidence

Check that owned settings are restored, portable repository config remains
usable, exact directory overrides match their baselines and no dangling tool
references remain. Record tool availability/versions and the shared installations
left in place. If a representative fixture or affected project check is required,
use the restored configuration and retain its real exit. Targeted tests require
a meaningful nonzero case count; nextest absence uses `cargo test` with the
appropriate target/filter. Preserve the designated final completion gate.

Do not implicitly release-build, clean or update the project. Preserve user
Cargo/sccache caches, private targets, source and worktrees. Removal is not a
filesystem change, blanket cache cleanup or worktree deletion. Keep receipts and
the completion record unless the owner separately selects their removal.

If also explicitly asked to remove the instruction package or skill projections,
first preserve access to this guide and the receipts, then use the ordinary
`vibe uninstall` / `vibe skill uninstall` commands with identifiers resolved from
the actual installation. Do not hand-edit generated boot lanes or unrelated
agent instructions. Package removal never substitutes for the reverse plan above.

Report restored scope, exact pending/conflicting steps, preserved shared tools
and the completion receipt path. A partial removal is reported as partial.

Primary references (access date: 2026-10-02): [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html),
[rustup overrides](https://rust-lang.github.io/rustup/overrides.html),
[Cranelift](https://github.com/rust-lang/rustc_codegen_cranelift),
[sccache](https://github.com/mozilla/sccache),
[nextest source installation](https://nexte.st/docs/installation/from-source/),
[mold](https://github.com/rui314/mold).
