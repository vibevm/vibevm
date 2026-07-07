# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-07 (the FOURTH campaign of the day). **The
AGENTIC-TCG-RUST CAMPAIGN (`spec/terraforms/AGENTIC-TCG-RUST-PLAN-v0.1.md`)
is COMPLETE — Phases 0–7 executed end to end on the owner's goal
«выполни план до конца», EXECUTED status in the plan, the full panel
green at close.** The tcg family is BILINGUAL: the same four `tcg_*`
tools now answer `language: "rust"` through the consumer's own
rust-analyzer — PROP-026's central bet (a new language is an enum
value, not new tools) is cashed. rust-ai-native is **0.5.0** with the
owner's D13 language-suffix renames executed. The session's ~28
commits (incl. this checkpoint pair) are PUSHED to the source mirrors
at session close per the wind-down contract._

> **`spec/WAL.md` is the canonical living state**; if this snapshot
> and the WAL disagree, the WAL wins. The **git log is the
> authoritative per-item record**. Boot first (`CLAUDE.md` →
> `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

The session opened on `восстанови сессию`, drafted the Stage-B
delivery plan for the TS null's follow-up (the owner BACKLOGGED it the
same hour — its five §14 review points stay open), then took the real
commission: the Rust twin of the agentic type oracle. The plan was
authored against verified tree facts, owner-amended (seven §17
resolutions — the D13 language-suffix policy is now a STANDING
convention in GUIDE §2), and executed whole: LSP bridge over the
consumer's rustup-resolved rust-analyzer, the `tcg-rust` enriching
relay with IN-PROCESS conform enrichment, `research/rust-demo`, the
product's per-language dispatch, both live chains green, and a 9/9
differential corpus with the privacy gap DOCUMENTED as an expectation.
Every §4 prediction held; no latency target moved (cold 2 535 ms vs
15 s budget; warm validate p50 < 1 ms vs 500 ms). After close the
owner reviewed the WAL's standing findings; the answer (recorded in
the WAL too): six are paid lessons, one is the designed approximation
posture, and exactly TWO items await owner decisions.

## Where work stands

- **Branch `main`**, working tree clean after the checkpoint pair;
  the session (~28 commits `77218b5`→HEAD) pushed to the mirrors at
  close — local == origin at this checkpoint.
- Versions: discipline-core 0.4.0, rust-ai-native **0.5.0** (+2
  crates `tcg-oracle-bridge-rust` / `tcg-cli-rust`, 4th `[[binary]]`
  `tcg-rust`, 3 spec docs, the D13 crate renames), typescript-ai-native
  0.4.0 (untouched — §4.5 held). vibevm product: `vibe-tcg` gained the
  rust language value + per-language recipe tables; vibe-mcp adapter
  untouched by construction.
- Floor at close: `self-check.sh` 13 steps exit 0; conform 0 (11
  gated / 4 exempt); specmap 592/578/590, 0 suspects/0 warnings;
  rust-demo floor ALL green; ts-demo floor 7/7; `vibe check` clean;
  `vibe bin list` = **8 binaries**; both `live_chain_on_*` green in
  2.7 s; corpus agreement 9/9.

## The open items (owner-court)

1. **Commission the discipline-core mini-fix** (RECOMMENDED at the
   post-close review): (a) the validator/scanner disagreement —
   `validate_against_tree` derives no name from a literal `"."` root
   (`Path::new(".").file_name() == None`) while the scanner uses the
   dir basename, so a bare single-crate consumer cannot gate its
   crate. Fix surface: the AUTHORED engine in
   `packages/org.vibevm/discipline-core/v0.4.0/crates/conform-core/src/config.rs`
   (the vendored twin I read is
   `packages/org.vibevm/rust-ai-native/v0.5.0/crates/vendor/conform-core/src/config.rs:248`)
   — resolve the literal against the root and take the basename,
   exactly as the scanner does; (b) optional in the same bump: a
   vacuity warning when gated crates exist but the scan found 0
   tagged items. Ritual: discipline-core version bump → `cargo xtask
   sync-engines` → both stacks re-vendored → re-materialise. ~Half a
   day with gates.
2. **Registry publish 0.5.0/0.4.0/0.4.0** — owner call, unchanged.
3. **Stage-B delivery experiments** — BACKLOGGED
   (`spec/terraforms/TCG-STAGE-B-DELIVERY-PLAN-v0.1.md`, five open §14
   review points; re-verify its §1 facts at pickup — they age).
4. **`vibe install --refresh <pkg>` ergonomics** (optional): a
   consumer's local-dir registry (`--registry ../../packages`) is NOT
   in-workspace for PROP-011 §2.6, so upstream package edits need the
   documented rm-and-reinstall of the demo's slot. Live-able;
   commission only if it starts to annoy.
5. `ra_ap_*` embedding — ROADMAP.md **Far backlog** (first entry);
   token-level TCG — VERY-FAR-FUTURE; PROP-025 v2 surfaces unchanged.

## Quick-start (verify the tree)

```sh
bash tools/self-check.sh; echo "EXIT=$?"        # 13 steps, must be 0
cargo run -q -p vibe-cli -- bin list             # 8 binaries; tcg-rust listed
cd research/rust-demo && cargo run -q --manifest-path ../../Cargo.toml -p vibe-cli -- \
    bin exec tcg-rust -- validate crates/rust-demo/src/cells/greeting.rs --root .
                                # 0 diagnostics; 0 findings; exit 0
cargo run -q -p vibe-cli -- bin exec tcg-rust -- \
    bench --corpus research/tcg-bench/corpus-rust \
    --report /tmp/r.json --root research/rust-demo   # agreement 9/9
cargo test -p vibe-mcp --test tcg_tools -- --ignored  # BOTH live chains
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n{"jsonrpc":"2.0","id":2,"method":"tools/list"}\n' \
  | cargo run -q -p vibe-cli -- mcp serve --path research/rust-demo
                                # four tcg_* tools; language enum = [typescript, rust]
```

## Non-obvious findings (this campaign; the WAL section carries all eight in full)

The post-close review's framing, worth keeping: the WAL writes
findings BLUNTLY by design (FALSE / impossible / load-bearing) so the
next session trips over the words — the tone is the genre, not an
alarm. Status of the eight: five are paid-and-tested lessons
(serverStatus declare-and-trust; the progress-drain heuristic
falsified twice and deliberately ABSENT with a pinning replay test;
r-a's default-off experimental diagnostics shipped via BOTH config
channels; multi-fence hover; vacuous-green scan_roots), one is the
designed D2 approximation (privacy: r-a silent, cargo E0423/E0603 by
reference shape — corpus case 06 asserts the asymmetry and flips red
when r-a catches up; the floor stays the truth), and two are the
owner-court items above (#1, #4).

Mechanical specifics a future session needs:

- The r-a↔rustc CODE_MAP rows (bench-owned): E0308↔E0308,
  E0425↔E0425, **E0107↔E0061** (arity), **E0559↔E0609**
  (unknown-field), E0063↔E0063, E0599↔E0599.
- Corpus content must EXTEND the real demo file, never restate its
  imports (a duplicate `use` added E0252 noise once).
- specmap `scan_roots` are CRATE DIRS named explicitly; a parent dir
  scans nothing and the gate greens by vacuity.
- The tests-out split's conform gotcha bit again: non-`#[test]`
  helpers in a `#[path]` tests file need their own `#[cfg(test)]` or
  their expects read as domain.
- rust-analyzer is a STACK PREREQUISITE (`rustup component add
  rust-analyzer`; 1.93.1 on this box, installed during plan
  authoring); package e2e tests hard-fail with that recipe, never
  skip.

## Repository map (delta over the agentic-tcg map)

```
vibevm/
├─ crates/vibe-tcg/                LANGUAGES += rust; per-language recipe
│   └─ src/registry/tests.rs        tables; tests-out split (351-line cell)
├─ research/rust-demo/             NEW: the committed Rust consumer testbed
│   └─ crates/rust-demo/src/cells/  (GuestName newtype, empty frozen baseline,
│                                    floor green via the slot toolchain)
├─ research/tcg-bench/
│   ├─ corpus-rust/{cases,content}/ NEW: 9 differential cases incl. the
│   │                                documented privacy gap + Cyrillic pin
│   └─ reports/REPORT-2026-07-07-rust-baseline.md   agreement 9/9 + latency
├─ spec/terraforms/
│   ├─ AGENTIC-TCG-RUST-PLAN-v0.1.md   EXECUTED (commit map inside)
│   └─ TCG-STAGE-B-DELIVERY-PLAN-v0.1.md  BACKLOGGED (owner)
├─ ROADMAP.md                      M1.25 (in execution → shipped with this);
│                                   NEW "Far backlog" section (ra_ap entry)
└─ packages/org.vibevm/rust-ai-native/v0.5.0/   (bumped from v0.4.0)
    ├─ crates/{conform,discipline,specmap}-cli-rust/   D13 RENAMES
    ├─ crates/tcg-oracle-bridge-rust/  NEW: the LSP client seam (frame/
    │                                   position/client/oracle cells)
    ├─ crates/tcg-cli-rust/            NEW: bin tcg-rust (serve/one-shot/bench)
    └─ spec/rust/
        ├─ tools/vibe-agentic-tcg-rust.md   NEW seven-section brief
        ├─ tools/vibe-tcg-rust.md           RENAMED from vibe-tcg.md (D13)
        └─ mechanisms/TCG-ORACLE-RUST-v0.1.md, TCG-PROTOCOL-RUST-v0.1.md  NEW
```

## Standing policies in force (long form)

- **D13 language-suffix rule (owner, 2026-07-07, GUIDE §2)**: every
  Rust artifact with a cross-language analog ends in `-rust` — crates
  and modules included, executables and externally visible libraries
  especially; no-analog artifacts (`env-audit`) and language-neutral
  ones (vendored engines, the generic `vibe-tcg` crate) are outside
  the rule.
- **The fidelity posture (D2)**: the Rust oracle is an honest
  APPROXIMATION — r-a is not rustc; curated-class agreement through
  the mapping table, documented gaps as corpus expectations, the
  floor is the truth. Consumer docs repeat it everywhere.
- **rust-analyzer is a stack obligation (D11)**: installing
  ai-native-rust obliges the machine; inside the stack's suite
  absence is a recipe-carrying FAILURE; outside — no obligation.
- **Latency misses report, never cancel (§17.7)** — none occurred.
- Clean-room (PLDI'25 repo untouched), production-grade/no-MVP, the
  four CLAUDE.md rules, mirror/publish held for the owner's word —
  all unchanged.

## Recent commit chain (campaign, newest first — see git log for all)

```
docs(wal)/docs(continue)      session-end checkpoint pair (this push)
docs(wal)                     the agentic-tcg-rust campaign complete
docs(plan)                    flip the rust agentic campaign to executed
build(deps)                   re-materialise vibedeps - campaign close
test(research)                the rust differential corpus + bench baseline (9/9)
style(tcg)                    tests-out split for the grown registry cell
test(mcp)                     the rust live chain + absent-stack recipes
feat(tcg)                     the rust language value across the family
build(deps)                   re-materialise vibedeps with the tcg toolchain
docs(packages)                declare the tcg-rust binary + self-trace roots
feat(rust-ai-native)          tcg-rust - serve, one-shot ops, bench
refactor(conform)             export the rust rule-set assembly seam
feat(rust-ai-native)          tcg-oracle-bridge-rust - the r-a client seam
feat(research)                rust-demo - the committed Rust consumer testbed
fix(rust-ai-native)           correct the stale specmark path in init + guide
build(deps)                   re-materialise vibedeps at rust-ai-native 0.5.0
style(rust-ai-native)         reflow after the -rust renames
docs(spec)                    PROP-026 - the rust rows + roadmap M1.25
docs(rust-ai-native)          the agentic tcg brief + mechanisms
refactor(rust-ai-native)      rename the cli crates to the -rust convention
build(packages)               bump rust-ai-native to 0.5.0
docs(plan)                    phase-0 spike findings rewrite D3/D10/s4.1
docs(plan)                    fold the owner review into the rust tcg campaign
docs                          roadmap far backlog - ra_ap embedding option
docs(plan)                    draft the rust agentic oracle campaign
docs(plan)                    backlog the stage-b delivery campaign
docs(plan)                    draft the tcg stage-b delivery campaign
```

The WAL supersedes this snapshot wherever they diverge. Session-resume
phrase: `восстанови сессию` (boots into a status report and waits —
the open items above are the owner's call, not a standing mandate;
item 1's recommendation awaits the owner's yes/no, not execution).
