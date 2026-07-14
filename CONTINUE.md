# CONTINUE.md — cold-resume checkpoint (2026-07-14)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

The active work is the **cultural-refactor** branch: extracting vibevm's reusable programming-culture
knowledge out of its host spec corpus into **installable packages**, so vibevm becomes a *thin consumer*
of its own practice ecosystem (it now dogfoods the whole `redbook` edition). **40 commits on
`cultural-refactor`, ahead of `main`, not yet pushed.** The corrected (v2) extraction model: every
reusable idea → its own package, content **moved in**; the host section deleted or reduced to a thin
**pointer + project-specific residue**; the package is a real **dependency** (no "loading prose"); the
host says nothing about how the dependency loads. Hierarchical topics become **families** (PROP-028).

**Immediate next:** implement **PROP-034** (transitive links + the static boot-link graph) in code →
connect `redbook` as `inline-transitive` → then **Section D** (per-module STAYS analysis). The plan
lives in `neworder2/concepts.md`.

## Where work stands

- Branch **`cultural-refactor`** @ `3e46162`, working tree **clean**, **40 commits ahead of `main`**,
  **no upstream** (never pushed).
- Gate state at HEAD: `cargo xtask specmap` = **0 suspects, 3 warnings** (all three are the known
  `duplicate-anchor` on the generated `spec/boot/INLINE.md` — backlog **B2**, cosmetic); `vibe check`
  clean; `bash tools/self-check.sh` exit 0.
- Backups: branch `cultural-backup` @ `0eb3202` (the rejected v1 "cite-in-place" model); tag
  `pre-cultural-refactor` @ `8831a14` (the baseline the v2 work reset to).

## Active blocker + the exact unblock

**Connecting `redbook` as `inline-transitive` is blocked on implementing PROP-034.** The vibe tooling
does not yet understand a transitive `link` value, and does not resolve the boot closure as a static
linker (dedup + topological order). Root cause (verified): `crates/vibe-workspace/src/install/bootgen.rs`
resolves `declared_link.or(suggested_link)`, and a **transitive** dependency's `declared_link` reads back
as `None` — so inclusion mode cannot propagate down a subtree.

**Unblock:** implement PROP-034 — the `vibe-core` manifest `link` enum accepts `inline-transitive` /
`static-transitive`, and `bootgen` does effective-mode propagation (the `inline ⊐ static ⊐ dynamic`
lattice, inline-wins-monotone), **dedup** (each package once), **topological sort** (every dependency
before its dependents), and **cycle rejection** at generate time.

## Next-steps recipe

1. **Owner review of PROP-034** (`spec/modules/vibe-workspace/PROP-034-transitive-links-boot-graph.md`) —
   it is the contract the code follows.
2. **Implement PROP-034** (substantial Rust task): manifest schema (`crates/vibe-core/src/manifest/…`)
   + boot resolution (`crates/vibe-workspace/src/install/bootgen.rs`). Add tests for dedup, topo order,
   and cycle rejection.
3. **Connect redbook inline-transitive:** in root `vibe.toml`, set
   `"flow:org.vibevm.world/redbook" = { version = "^0.2.0", link = "inline-transitive" }`; **turn the
   two MCP servers OFF** (see findings); reinstall; verify `spec/boot/INLINE.md` carries the whole
   redbook closure, **deduplicated and topologically ordered**. Drop the interim per-member
   `[boot_snippet].link = "inline"` self-suggestions on the git-practices members (PROP-034 §3).
4. **Section D:** walk the module PROPs (`neworder2/concepts.md` §D list) — confirm each is genuinely
   vibevm machinery (STAYS), add a companion cite where a general practice applies (the PROP-012 /
   PROP-008 shape).

## Non-obvious findings (do not re-learn)

- **Install command:** `./target/debug/vibe.exe install --registry packages --assume-yes`. Build the
  binary first (`cargo build -p vibe-cli`). The **PATH `vibe`** (`~/opt/bin/vibe`) is **stale** — never
  use it. `--registry packages` bypasses the embedded registry (which needs an active VVM install).
- **MCP servers must be OFF before install.** `rust-ai-native-mcp.exe` + `typescript-ai-native-mcp.exe`
  lock their `vibedeps/` slots → install fails with `Access denied (os error 5)`. Verify off, then install.
- **Transitive `link` does not propagate** (the PROP-034 root cause, above). Interim workaround in force:
  the four git-practices members self-suggest `link = "inline"` in their own `[boot_snippet]` so the
  commit rules reach `INLINE.md` today; PROP-034 §3 removes that need.
- **specmap:** run `cargo xtask specmap`, then **`git checkout -- specmap.json`** to keep the parked
  baseline (do not commit the regenerated map). 3 `duplicate-anchor` warnings on `INLINE.md` are
  known/cosmetic (B2). Editorial spec edits that change a `req` unit's content take a
  `spec-editorial: <anchor>` commit trailer.
- **Commits:** heredoc only (`git commit -F - <<'MSG'`), **never** `-m` with backticks (command
  substitution has corrupted messages twice). **No AI-authorship trailers, ever** (Rule 1).
- **Editing:** use Edit/Write only — PowerShell 5.1 corrupts UTF-8-no-BOM round-trips; revert bad edits
  with `git restore`.
- **Trio byte-identity:** `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` must stay byte-identical (self-check's
  `sync-engines` gate) — apply every trio edit to all three.

## What's done (the whole refactor)

- **git-practices family** (PROP-028): renamed members with a `git-` prefix (`git-conventional-commits`,
  `git-atomic-commits`, `git-autonomy`, `git-attribution-policy`); members self-suggest inline → the four
  commit rules land verbatim in `spec/boot/INLINE.md`. Host PROP-000 §12 → stub; the trio's rules →
  a one-line git-practices pointer.
- **redbook dependency:** vibevm depends on the whole `redbook` edition (static) — extracted practices
  reach vibevm through it, no per-flow host entry.
- **Class-A extractions** (host spec thinned to residue + flow cite): source-mirrors (PROP-016),
  health-audit (PROP-013), addressable-specs (PROP-029), spec-genres (`spec/design/README`),
  manual-tests (§14), secrets-hygiene (§20). **Companion cites** (code-verified feature specs kept
  whole): managed-blocks (PROP-012), qualified-naming (PROP-008). **Light remainder:** two-process-model
  + sync-from-code (`00-core.md`), decision-records (design README).
- **operating-modes / mfbt:** PROP-006 reduced to a stub pointing at the `operating-modes` flow (which
  already carries `mfbt-mode.md`) — no duplicate `mfbt` package (single-source).
- **delegation-first:** authored `org.vibevm.fractality/delegation-first` (fractality-opinionated: names
  GLM-5.2, the ~5%-boss / ~95%-worker target, first-level-only — does **not** prescribe fractality's
  internal task distribution; recommends enabling **RLM**; a `#strong-form`). vibevm depends on it
  (static). The trio's Delegation-first block thinned to: pointer → a "Running fractality here"
  operational note → the Rule 1 & 4 binding → the owner-maintained operating-facts ledger.
- **PROP-034** drafted (this session's last deliverable): the transitive-link + static-boot-graph spec.
- **§3 licensing** — settled, left as-is (its EULA text is an owner-frozen historical mention over a
  UPL-licensed tree; the relicense + audit are already done).

## What remains

1. **Implement PROP-034** (code) → connect redbook `inline-transitive` (the blocker above).
2. **Section D** — STAYS analysis of the module PROPs.
3. Backlog (`neworder2/memory/BACKLOG.md`): **B2** (specmap should skip generated boot artifacts —
   the 3 warnings), **B3** (regenerate the fractality nested-project lock post-rename). **B1** was
   promoted to PROP-034; **B4** is done.
4. Deferred polish: a `redbook` edition bump + README refresh once the extraction settles; an owner
   re-read of the real-time-authored `DELEGATION-FIRST-PROTOCOL.md`.

## Repository map

- `crates/vibe-*` — the Rust workspace: `vibe-cli`, `vibe-core` (manifests/graph), `vibe-workspace`
  (install + **bootgen**, where PROP-034 lands), `vibe-registry`, `vibe-resolver`, `vibe-index`,
  `vibe-check`, `vibe-publish`, `vibe-llm`; `xtask/` (specmap, mirror, health).
- `packages/org.vibevm.world/**` — the extracted practice flows (redbook + its members, the git-* family,
  source-mirrors, health-audit, operating-modes, …). `packages/org.vibevm.ai-native/**` — the language
  stacks + discipline. `packages/org.vibevm.fractality/**` — the fractality specspace + `delegation-rules`
  + the new `delegation-first`.
- `spec/` — `boot/` (00-core, 90-user, generated INDEX.md + INLINE.md), `common/` (PROP-000, PROP-006,
  …), `modules/` (per-crate PROP/FEAT incl. PROP-009 + **PROP-034**), `design/` (rationale), `flows/`,
  `discipline/`, `WAL.md`.
- `neworder2/` — **the cultural-refactor's working notes**: `concepts.md` (the exhaustive plan +
  progress), `memory/00-understanding.md` (the v2 model), `memory/EXTRACTION-PROCESS.md` (the generalized
  procedure), `memory/BACKLOG.md` (B1–B4).
- Root: `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (byte-identical trio), `vibe.toml`, `vibe.lock`.

## Architectural / policy decisions in force

- **The v2 extraction model** (owner-corrected): content **moves into** the package; the host section is
  deleted or a thin pointer + residue; the package is a real **dependency**; the host contains **no
  loading prose** (the dependency mechanism handles delivery); hierarchical topics → **families**;
  everything reusable force-loads (static default; inline for boot-critical).
- **Dogfooding via redbook:** vibevm consumes the whole tested edition, so a practice extracted from its
  specs → package → redbook → back to vibevm automatically; each remaining extraction is "thin the host
  spec + cite the flow."
- **delegation-first is fractality-opinionated** (org.vibevm.fractality group) and covers **first-level**
  delegation only; how fractality splits a swarm internally is fractality's own system.
- **PROP-034:** the boot closure is a **static link** — transitive inclusion links, effective-mode
  precedence, dedup, topological order.
- Repo rules: Rules 1–4 (human-authored attribution, Conventional Commits, atomicity, autonomy) are the
  `git-practices` family; source is dual-homed (GitVerse `origin` canonical + GitHub mirror), fanned out
  by `cargo xtask mirror`.

## Recent commits (last 25)

```
3e46162 spec(vibe-workspace): PROP-034 — transitive links + the static boot-link graph
1d9aa2f docs(backlog): mark B4 done — trio delegation block thinned
4720d65 refactor(delegation): thin the trio's fractality operational block (B4)
661e842 docs(backlog): record B4 — finish thinning the trio delegation block
ca9356a refactor(boot): reduce the trio's commit rules to a git-practices pointer
71971e6 refactor(delegation): drop the general obligations from the trio block
a470a77 refactor(fractality): reshape delegation-first per owner review
c8a1aa8 refactor(delegation): thin the trio directive to a delegation-first pointer
09151bf feat(host): depend on delegation-first (static)
0ef57b2 feat(fractality): author the delegation-first flow package
4d5ccf8 refactor(spec): reduce PROP-006 to an operating-modes pointer
ebffebf docs(refactor): light remainder done; §3 settled; Section B/D remain
a210598 refactor(spec): cite the decision-records genre from the design README
40b0da6 refactor(boot): cite two-process-model + sync-from-code from 00-core
70d5600 refactor(spec): cite qualified-naming from PROP-008
5423c34 docs(refactor): mark 7 class-A extractions done; flag §3 as owner-blocked
5cba505 refactor(spec): cite managed-blocks from PROP-012's redirect block
a5dc987 refactor(spec): thin PROP-000 §20 to the secrets-hygiene flow
49fe531 refactor(spec): thin PROP-000 §14 to the manual-tests flow
97cbb23 refactor(spec): thin spec/design/README to the spec-genres flow
b416acb refactor(spec): thin PROP-029's addressing rationale to addressable-specs
81368ee refactor(docs): repoint the mobility plan to the renamed git-* members
b4beb4b refactor(spec): thin PROP-013 to vibevm's health-audit instance
a2de9df docs(refactor): record v2 progress + the redbook reframe in the plan
2883b94 refactor(spec): thin PROP-016 to vibevm's source-mirror setup
```

## Quick-start

```sh
# build the working-tree binary (never the PATH vibe)
cargo build -p vibe-cli

# gate ladder
cargo xtask specmap && git checkout -- specmap.json   # expect 0 suspects, 3 known warnings
./target/debug/vibe.exe check                          # expect clean
bash tools/self-check.sh                               # expect exit 0 (fmt/test/clippy/vibe check/conform/sync-engines)

# reinstall boot artifacts (MCP servers OFF first!)
./target/debug/vibe.exe install --registry packages --assume-yes
```
