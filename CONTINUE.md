# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-28. **The TRACEABILITY RELOCATION CAMPAIGN (PROP-024 follow-up)
is COMPLETE and green.** `specmap-core` + `specmark` + `specmark-grammar` now
live in `stack:org.vibevm/rust-ai-native` next to the conform toolchain, so the
package ships the **whole** Rust verifier — `conform-rust` AND `specmap-rust` —
and **traces + gates itself**. The Ф4a specmark-free debt is PAID: the conform
crates carry their `scope!` tags again. Local on `main` at `f38d719`,
**11 commits ahead of the origin/mirror tip `c3fcf63`, NONE mirrored — the
mirror is HELD for the owner's explicit word.** Floor green: `self-check.sh`
9 steps exit 0; vibevm `specmap --check` 614/566/578/0/0 (4 exempt, 0 orphans);
package `specmap-rust --gate` 0 orphans; `vibe check` 0/0/0._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record**.
> Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`),
> then read this.

---

## TL;DR

PROP-024 makes a package a project that ships runnable code. The conform half
relocated earlier (Ф1–Ф7, `conform-rust`); **this campaign relocated the
traceability half** — specmap/specmark — so the package ships the verifier
whole and disciplines its own code. Five phases, each green at its boundary,
7 commits `ce4eaa1`→`f38d719`. The campaign plan
(`spec/terraforms/TRACEABILITY-RELOCATION-PLAN-v0.1.md`) is fully executed.

## Where work stands

- **Branch `main`**, tip `f38d719`. **11 commits ahead of `origin/main`
  (`c3fcf63`); NONE mirrored.** Working tree clean.
- The 11 ahead = 4 from the prior session (`2a2932a`, `c240fc7`, `4d53ccf`,
  `452a5dc` — the conform-rust rename + plan + its checkpoint) + the 7 this
  campaign (`ce4eaa1` Ph1, `1456944` Ph2, `f532eab`+`97d0bef`+`d33ed84` Ph3,
  `dee0321`+`f38d719` Ph4).
- Floor **green** at the tip: `bash tools/self-check.sh` exit 0 (9 steps: 5
  vibevm + 3 package fmt/test/clippy + 1 package self-trace); vibevm
  `cargo xtask specmap --check` 614/566/578/0/0 (4 exempt); package
  `specmap-rust --gate` 0 orphans; `vibe check` 0/0/0.

## The one open item — MIRROR (held, outward-facing)

The 11 ahead commits are local. The mirror is gated for the owner's explicit
word (PROP-016 hub-and-spoke). **Unblock: the owner says "mirror" →
`cargo xtask mirror`** (after `cargo xtask mirror --check`). Do NOT mirror
autonomously. This is the only thing the campaign left pending.

## What "done" looks like (achieved)

```
packages/org.vibevm/rust-ai-native/v0.2.0/
├─ Cargo.toml                  the package's OWN Cargo workspace (8 members)
├─ conform.toml? NO            policy stays with the CONSUMER (vibevm); the package ships engines
├─ specmap.toml                NEW — the package's own orphan-coverage policy (--gate)
├─ specmap.json? NO            the package self-trace is --gate (orphans-only), no committed index
└─ crates/
   ├─ conform-core             re-tagged (13 scope! restored, specmark dep back)
   ├─ conform-frontend-rust    re-tagged
   ├─ conform-cli              bin: conform-rust   (exempt from the package gate — CLI driver)
   ├─ env-audit                re-tagged
   ├─ specmap-core             owns the Specmap JTD types; config-driven; carries its scope! tags
   ├─ specmap-cli              bin: specmap-rust   (NEW — exempt; CLI driver)
   ├─ specmark                 proc-macro          (exempt — bootstrap pair)
   └─ specmark-grammar         shared grammar      (exempt — bootstrap pair)
```
- The package builds **two binaries** (`conform-rust`, `specmap-rust`) and
  **traces + gates itself** (self-check step 9: `specmap-rust --gate`).
- vibevm consumes both engines by external path-dep; `cargo xtask conform` and
  `cargo xtask specmap` are thin shims; the 10 specmark dogfooders are unchanged
  and still tagged.
- `vibe-wire` no longer owns the Specmap types; that edge is gone.

## Next-steps recipe (whoever picks up)

1. **Mirror the 11 ahead on the owner's word** (`cargo xtask mirror`). Nothing
   in this campaign is mirrored yet.
2. **Smaller follow-ups, none blocking:**
   - `crates/vibe-registry/src/lib.rs` at the 600-line budget edge — split the
     copy + hash into a module (a health-collector follow-up).
   - `.vibeignore` glob support (PROP-024 §2.2 "optional", no consumer needs it).
   - **discipline-core consolidation** (the deferred owner decision): promote the
     neutral engines (conform-core + specmap-core) into
     `flow:org.vibevm/discipline-core` for cross-language reuse — only worth it
     once a SECOND language (TypeScript) actually needs the shared core. A
     neutral package depending on a Rust proc-macro is a backwards edge; defer
     until the decomposition pays.
   - The package could also **conform-check ITSELF** (it only specmap-traces
     itself today) — a symmetric follow-up if desired.

## Non-obvious findings (still in force)

- **The `/generated/` path exclusion is crate-agnostic** (rscan + conform
  `exclude_substrings = ["/generated/"]`). That is what let the JTD Specmap
  types move into `specmap-core/src/generated/` without becoming orphans —
  byte-identity in Ph1 rode on it.
- **Dangling edges are WARNINGS, not failures; the ratchet gates orphans, not
  resolution.** This is why the package self-trace is `specmap-rust --gate`
  (orphans-only): the package's `scope!` tags cite vibevm-hosted
  `spec://vibevm/discipline/…` units, so on the package tree every edge is
  cross-repo dangling — a committed package `specmap.json` would be all-noise,
  but "is every gated crate's public surface tagged" is the real self-discipline.
- **A new tagged module grows the index by exactly +1 code_item +1 edge** (Ph2's
  config.rs); the relocation removed specmap-core's ~18 modules (Ph3,
  584→566 / 597→578). The drift classifier prints edge deltas, not code-item
  deltas — read the summary line for code items.
- **The generated `Specmap`/`Edge` derive only Serialize/Deserialize (no
  `Debug`)** — test assertions cannot `{:?}` them.
- **Machine quirk that bit twice this session: `bash … > "$VAR/file" 2>&1` with
  an UNSET `$VAR`** writes to `/file` → Git-Bash permission-denied and the
  command never runs (a background self-check reported "exit 0" that was the
  failed redirect, not a real pass). Inline the scratchpad path, or set the var
  on the SAME line. Unchanged: Edit/Write only (PS Set-Content corrupts
  UTF-8-no-BOM); `git commit -F - <<'MSG'`; self-check via Git Bash; check the
  REAL exit code, never a `| tail`'d pipe.

## Repository map (post-campaign)

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = cargo xtask
├─ Cargo.toml                root: exclude=[packages,vibedeps]; specmark/specmap-core/specmap-cli → package paths
├─ conform.toml              vibevm's conform policy — 10 gated / 4 exempt (the 3 moved crates de-gated)
├─ specmap.toml              vibevm's specmap policy — scan crates/*+xtask, 4 exempt (specmark/grammar dropped)
├─ specmap.json              the traceability index — 614/566/578 (specmap-core's modules left)
├─ schemas/specmap.jtd.json  the Specmap schema (generator INPUT; output routes to the PACKAGE now)
├─ spec/
│   ├─ WAL.md                CANONICAL living state
│   ├─ common/               PROP-000.. + PROP-024-code-bearing-packages.md
│   ├─ discipline/           ENGINE-CONFORM, PROP-014 (the spec units the package's tags cite — vibevm-hosted)
│   └─ terraforms/           TRACEABILITY-RELOCATION-PLAN-v0.1.md (THIS campaign — now executed)
├─ packages/org.vibevm/rust-ai-native/v0.2.0/
│   ├─ Cargo.toml + specmap.toml + LICENSE.md   the package's workspace + self-trace policy
│   └─ crates/{conform-core, conform-frontend-rust, conform-cli (bin conform-rust), env-audit,
│              specmap-core, specmap-cli (bin specmap-rust), specmark, specmark-grammar}
├─ vibedeps/                 materialised install (git-TRACKED); rust-ai-native slot ships all 8 crates (no target/)
├─ crates/                   vibe-{core,cli,install,registry,resolver,workspace,mcp,check,publish,index,wire,graph,llm}
│                            (specmap-core / specmark / specmark-grammar NO LONGER HERE — moved to the package)
├─ xtask/                    conform.rs + specmap.rs = thin shims over the package CLIs; codegen.rs routes specmap → package
├─ tools/self-check.sh       9-step floor gate (5 vibevm + 3 package fmt/test/clippy + 1 package specmap self-trace)
└─ specmap.json              the traceability index + (folded) ratchet now in specmap.toml
```

## Recent commit chain (newest first)

```
f38d719 build(deps): re-materialise vibedeps for the package self-trace      (Ph4)
dee0321 feat(specmap): re-tag the conform crates + trace the package itself   (Ph4 — the payoff)
d33ed84 build(deps): re-materialise vibedeps for the traceability move        (Ph3)
97d0bef chore(specmap): regen the index for the relocation                    (Ph3)
f532eab refactor(specmap): relocate the traceability toolchain into the package (Ph3)
1456944 refactor(specmap): config-driven scan via specmap.toml                (Ph2)
ce4eaa1 refactor(specmap): own the Specmap types, sever the vibe-wire edge    (Ph1)
452a5dc docs(wal): save session — Post-Ф7 rename + traceability plan, mirror held  (← origin/mirror tip is c3fcf63, 4 below)
4d53ccf docs(continue): save session — conform relocation done, traceability next
c240fc7 docs(plan): write the traceability relocation campaign + resume pointers
2a2932a refactor(conform): name the binary conform-rust to avoid PATH collisions
c3fcf63 docs(wal): checkpoint — Ф4-Ф6 landed green, verification proven        (← MIRRORED tip)
d725b71 docs(continue): cold-resume checkpoint — code-bearing packages Ф4-Ф6
6c5ee9e docs(typescript): spec the future conform-frontend-typescript
ac50f72 docs(conform): drop a stale CONFORM_GATED reference from the env rule
56c08a0 docs(discipline): reflect the conform.toml policy in the sweep manual
302454b test(conform): prove the shipped gate catches violations end-to-end
b0d1830 docs(discipline): point the mechanism table at the relocated conform
12c8592 fix(registry): exclude build output from the content hash and copy
2b0e6f6 refactor(conform): relocate the conform toolchain into the package
f2e2ab7 chore(specmap): regen + exempt the conform crates from the ratchet
26858dc refactor(conform): drop the specmark tags from the conform crates      (the Ф4a strip Ph4 reversed)
```

## Quick-start

```sh
bash tools/self-check.sh; echo "EXIT=$?"          # 9-step floor gate; must be 0 (real exit code)
cargo xtask specmap --check                        # vibevm index clean (614 / 566 / 578 / 0 / 0, 4 exempt)
cargo xtask conform check                          # 0 findings, 10 gated / 4 exempt
cargo run -q -p vibe-cli -- check --path .         # vibe check 0/0/0
cargo run -p vibe-cli -- install --registry packages --assume-yes   # re-materialise vibedeps

# The SHIPPED engines directly (consumer surface), from the package manifest:
PKG=packages/org.vibevm/rust-ai-native/v0.2.0
cargo run --manifest-path $PKG/Cargo.toml -p conform-cli  --bin conform-rust  -- check --path .
cargo run --manifest-path $PKG/Cargo.toml -p specmap-cli  --bin specmap-rust  -- --check --path .
cargo run --manifest-path $PKG/Cargo.toml -p specmap-cli  --bin specmap-rust  -- --gate  --path $PKG  # the package self-trace

cargo xtask mirror --check                         # confirm GitVerse + GitHub sync (HELD; do not mirror without owner word)
```

The WAL supersedes this snapshot wherever they diverge. Session-resume phrase:
`восстанови сессию` (boots into a status report and waits — the candidate next
work above is the owner's call, not a standing mandate).
