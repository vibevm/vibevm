# CONTINUE — cold-resume checkpoint

_Written 2026-07-21 (end of the terminal-product extraction session). The two-phase plan from this
session landed: Phase 1 (packages move to vibevm-term) was already done entering the session; Phase 2a
(host-side tear-down) and Phase 2b (version-manager port) landed this session. `spec/WAL.md` is the
canonical living state and supersedes this snapshot if they diverge._

## TL;DR

The **vibeterm / vibeframe / vibe-launcher extraction** is complete in two repos:

1. **`vibevm` (this repo, host)** — Phase 2a, the host-side tear-down. The terminal products moved out;
   `vibe self install` builds the `vibe` binary only. Terminal apps resolve through
   `$VIBEVM_<APP>` → packaged `<instance>/<app>/` (back-compat) → `PATH`, with an **in-place fallback**
   for `vibe tree` when no desktop terminal is resolvable. **10 commits** on `main` this session:
   the vvm install-pipeline refactor, the term/tree resolver, the vibe-launcher crate removal, the
   tools-gate drop, the PROP-019/PROP-042 doc rewrites, the specmap regen, the moved-source drop,
   the WAL checkpoint, and two clippy fixups the floor caught.
2. **`vibevm-term` (the sibling products repo at `C:/Users/olegc/git/v/vibevm-term/`)** — Phase 2b, the
   version-manager port. The full `vibe self` verb set ported Rust→TS as a twin
   (`spec://term-common/PROP-vvm`), cross-platform placement (Win .lnk + Linux .desktop + macOS .app +
   rename-aside), and per-product self CLIs (`bin/self.mjs` + `scripts/install.mjs` + PROP-self-install
   for vibeterm/vibeframe/launcher). **6 commits** there this session: the vvm port + tests, the two
   normative PROPs, the vibeterm/vibeframe self CLIs, the launcher self CLI, the WAL checkpoint.

**Both repos: floor-green, NOT pushed.** `vibevm` is **N ahead of origin/main**; `vibevm-term` has no
upstream configured (local repo). The owner's standing push route is `cargo xtask mirror` for vibevm.

**No blocker.** The next session picks up at the **push** decision (§Push), then optional real-build
verification of `<product> self install` against a real `~/opt` (the version-manager is exercised by a
fake-builder smoke today; the real Electron-packager / cargo-build paths are spec'd but not yet run
end-to-end on the products repo).

## Where work stands

| Repo | Branch | Ahead of origin | Working tree | Floor |
|------|--------|-----------------|--------------|-------|
| vibevm (host) | `main` | `git log --oneline origin/main..HEAD` (10 commits this session) | clean | `cargo check`, `cargo clippy --workspace --all-targets`, `cargo test -p vibe-cli --test vvm` (4/4), `self-check.sh` all green |
| vibevm-term (products) | `main` | no upstream configured | clean | `node --test` (53/53 in common) green |

Run `git -C C:/Users/olegc/git/v/vibevm log --oneline -12` and
`git -C C:/Users/olegc/git/v/vibevm-term log --oneline -8` to see the exact commits.

## Active blocker — none

The floor is green on both sides. The only open item is the **push decision** — see §Push.

## Exact next-steps recipe

1. **Verify the floor one more time on vibevm (host):**
   ```sh
   cd C:/Users/olegc/git/v/vibevm
   bash tools/self-check.sh        # the full floor; green per this checkpoint
   ```
2. **Decide on the push** (see §Push) — the owner's standing route is `cargo xtask mirror`.
3. **(Optional) Real-build verify the version-manager end-to-end** — the smoke today uses a fake
   builder. A real `vibeterm self install` requires `npm install` (Electron + node-pty) in
   `vibevm-term/.../vibeterm/v0.1.0/app/`, which needs network. Skip if offline.
4. **(Optional) vibevm-term push** — the repo has no upstream; configure one (`git remote add origin
   <url>`) if it should mirror, then `git push -u origin main`.

## Push

- **vibevm (host):** the owner's standing push route is `cargo xtask mirror` (mirrors to BOTH gitverse
  + github, NOT a bare `git push origin`). The extraction commits are routine per Rule 4 — no history
  rewrites, no force-push, no large blobs, no CI/secrets. The owner's go is the gate (held by
  convention until the owner says push; this session was not given that go).
- **vibevm-term:** no upstream configured today. If/when the owner wants to publish it, configure a
  remote and push.

## Non-obvious findings this session

- **Phase 2a was mostly done entering the session** — the working tree already carried the install /
  term / tree / doctor / tools / Cargo.toml / conform.toml edits and the deletions, uncommitted. The
  gap was **N.2 (PROP-042 §5 rewrite)** — the spec still described the dead `apps/<app>` walk-up tier
  and the silent vibeframe→vibeterm fallback. Fixed in commit `5b0cca1`.
- **The conform gate caught a Phase-2a miss** — `conform.toml` still listed `vibe-launcher` in
  `gated_crates` after the crate's directory was deleted; the `every_crate_is_gated_or_exempt` test
  failed. Fixed in `52f4e58`. (The env_root entry for vibe-launcher was updated in `5b0cca1`; the
  gated_crates line was the matching edit I missed.)
- **Two new clippy lints fired on the new code** — `unnecessary_lazy_evaluations` (`.then(|| dir)` →
  `.then_some(dir)`) and `collapsible_if` (nested `if let` + `if` → let-chain). Both in the new
  `via_path` resolver in `term.rs`. Fixed in `fb3f8e4` and `8b623f4`.
- **mtime in the vvm manifest had to be milliseconds, not nanoseconds** — JS's `Number` is IEEE-754
  and overflows `Number.isSafeInteger` at nanosecond mtime (≈1.78e18 > 2^53 ≈ 9.0e15). The TS port's
  manifest stores `mtime_ms` (integer, floored); the Rust twin's `.vvm-manifest.toml` carries
  `mtime_nanos` (i64). Both compare equal-on-equal-API (PROP-019 §2.15), so the cross-floor invariant
  holds — but a tool reading both manifests MUST account for the unit difference. Documented in
  `vibevm-term/.../common/v0.1.0/vvm/placer.mjs`.
- **The TS port has zero runtime deps** — no TOML library, no semver library. `vvm/toml.mjs` is a
  schema-bound mini TOML for `state.toml` + `.vvm-manifest.toml`; `selectorParse` uses a permissive
  semver regex. Matches the existing `args.mjs` / `packaging.mjs` style; keeps the product's dep graph
  empty.
- **Product `bin/self.mjs` uses a RELATIVE import to term-common** — the product packages have no
  `node_modules` at `bin/` (the formal `tool:org.vibevm.term/term-common` edge is resolved by `vibe
  install` at install time, not by Node). The relative import (`../../../common/v0.1.0/vvm/cli.mjs`)
  keeps the entry script self-contained.

## Repository map

### vibevm (this repo, host)

Top-level layout post-extraction:

- `crates/` — the Rust workspace (vibe-cli, vibe-core, vibe-install, vibe-registry, vibe-resolver,
  vibe-settings, vibe-actions, vibe-graph, vibe-index, vibe-llm, vibe-mcp, vibe-check, vibe-publish,
  vibe-spec, vibe-wire, vibe-workspace). **`crates/vibe-launcher/` is GONE** (moved to vibevm-term).
- `apps/` — **empty** (the vibeterm/vibeframe Electron apps moved to vibevm-term).
- `spec/` — the normative contracts. `spec/modules/vibeterm/`, `spec/modules/vibeframe/`,
  `spec/modules/vibe-launcher/` are GONE (moved). `spec/modules/vibe-cli/PROP-042-aiui-observation.md`
  stays but its §5 now describes the PATH resolver tiers (not the in-tree walk-up).
- `tools/` — `self-check.sh` (the floor driver), `first-run.sh`. Both dropped the vibeterm gates.
- `xtask/` — the dev tooling (`cargo xtask specmap`, `cargo xtask conform`, `cargo xtask mirror`).
- `packages/org.vibevm.fractality/` — the fractality launcher (separate specspace; its own boot).
- `assets/icons/` — only generic + `vibetree-*` variants remain (the per-product icons moved).

### vibevm-term (the sibling products repo)

- `org.vibevm.term/` — the product group:
  - `common/v0.1.0/` — term-common: shared TS helpers (`src/args.mjs`, `src/keymap.mjs`,
    `src/packaging.mjs`) + the ported version-manager (`vvm/*.mjs`) + normative wire contracts
    (`spec/modules/term-common/PROP-*.md`).
  - `vibeterm/v0.1.0/` — the complex terminal workspace (Electron + SolidJS). `app/` is the Electron
    app; `bin/self.mjs` is its version-manager entry.
  - `vibeframe/v0.1.0/` — the simple terminal frame. Same shape as vibeterm.
  - `launcher/v0.1.0/` — the GUI launchers (vibetree/vibeterm/vibeframe), Rust crate. `bin/self.mjs`
    uses `LauncherBuilder` (cargo), not the Electron packager.

## Architectural / policy decisions still in force

- **Hybrid model.** vibevm = open-source core (package manager, vibe CLI, vibe tree, version manager);
  vibevm-term = closed GUI products (vibeterm, vibeframe, term-common, vibe-launcher). Seamless
  integration — `vibe.exe` calls the apps when they're on `PATH`.
- **PATH is the integration surface.** Terminal apps resolve through `$VIBEVM_<APP>` → packaged
  `<instance>/<app>/` (back-compat) → `PATH`. No in-tree coupling.
- **In-place fallback for `vibe tree`.** A box without a desktop terminal product installed still
  gets a working tree — `vibe tree -t` runs the console TUI in place (subprocess), no desktop window.
- **Per-product version-manager.** Each product (`vibeterm`, `vibeframe`, `launcher`) carries its own
  version-manager twin of `vibe self`. State under `~/opt/<product>/`; shared `~/opt/bin/` shim dir.
- **Cross-floor twin contract.** The TS port MUST hold the same invariants the host Rust holds
  (PROP-019). A drift is a bug, not a divergence to tolerate. The host Rust stays authoritative.

## Recent commit chain (vibevm host, last ~14)

```
8b623f4 fix(term): collapse the via_path if-let (clippy collapsible_if)
fb3f8e4 fix(term): use then_some in via_path (clippy unnecessary_lazy_evaluations)
dee6c4e docs(wal): checkpoint — vibeterm/vibeframe/launcher extraction Phase 2a
acbc020 chore(specmap): drop the moved terminal/launcher anchors
7e46d84 chore(extract): drop the moved terminal/launcher sources
5b0cca1 docs(specs): apps and launchers are external products
220c661 chore(tools): drop the vibeterm gates from self-check and first-run
e220089 refactor(workspace): remove the vibe-launcher crate
b95efdb refactor(term): resolve terminal apps via PATH with an in-place fallback
4221a20 refactor(vvm): stop packaging the terminal products and launchers
52f4e58 fix(conform): drop vibe-launcher from gated_crates
84f47de fix(vvm): retry the instance rename through a transient scanner lock
b49f67d chore(discipline): refresh health snapshot after the local-registry fix
6dbf6ba fix(registry): narrow the local-directory backend to the file: scheme only
```

## Quick-start

```sh
# vibevm (host) — floor (from repo root)
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask conform check
cargo xtask specmap            # regenerate; --check for strict CI drift
cargo run -q -p vibe-cli -- check
bash tools/self-check.sh       # the full floor driver

# vibevm (host) — push BOTH mirrors (NOT `git push origin`)
cargo xtask mirror
cargo xtask mirror --check

# vibevm-term (products) — the common floor
cd C:/Users/olegc/git/v/vibevm-term/org.vibevm.term/common/v0.1.0
npm test                       # 53 tests (12 vvm + 41 helpers)

# vibevm-term — a product's CLI (help is the no-op smoke)
node ../vibeterm/v0.1.0/bin/self.mjs help
```
