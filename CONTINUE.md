# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-07, session close. This session authored the
**redbook collection** — the AI-native development practices distilled
from the book and this project's own laws into installable, product-
agnostic `flow` packages under an edition-pinned umbrella. Two waves:
wave 1 built the book's core (10 flows) + the umbrella at edition
0.1.0; wave 2 built the project-practice layer (11 flows) + edition
0.2.0 pinning all 21 members. Everything is committed and PUSHED to
both mirrors (`d28de5c`); the tree is clean; self-check was green at
close. The collection is NOT wired into vibevm's own `vibe.toml` — it
is a product for OTHER projects; this repo only authors and publishes
it._

> **`spec/WAL.md` is the canonical living state**; if this snapshot
> and the WAL disagree, the WAL wins. The **git log is the
> authoritative per-item record.** Boot first (`CLAUDE.md` →
> `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

The session opened on a deep-analysis mandate: extract the
generalizable practices from `refs/book/` (3 chapters) and the
project's spec corpus into installable packages under an umbrella
called **redbook**, for any product / language / agent. The analysis
mapped ~16 packageable practices; the owner then commissioned the
build in two waves with six rulings (packages are CANON over the
Discipline's internal copies where they clash; the umbrella is
`kind=flow`; edition versioning; the attribution package ships with
concealment as the default; the DISCOVERY prompt is included; packages
in English, the book itself shipped in Russian as-is with an English
edition to follow and take priority). Both waves are done: **21
practice packages** and a two-edition umbrella, all UPL-1.0, all
grep-verified product-agnostic, each protocol doc carrying a
"Re-derive for your project" prompt-task.

## Where work stands

- **Branch `main`**, tree clean, **local == origin == github @
  `d28de5c`** (mirrored at close per Rule 4 — owner-commissioned).
- **redbook collection** lives entirely under
  `packages/org.vibevm/`, 22 new package dirs (21 members + the
  umbrella in two editions). Nothing else in the repo was touched by
  the collection work except `spec/WAL.md`.
- **The umbrella:** `flow:org.vibevm.world/redbook` in **two editions** —
  `v0.1.0` (pins 10 core members) and `v0.2.0` (pins all 21). An
  edition is a tested set: the umbrella version IS the edition number,
  every member exact-pinned (`=X.Y.Z`), members evolve on their own
  lines between editions. The book ships inside the umbrella at
  `spec/book/ru/` (chapters 1–3, verbatim Russian).
- **The 21 members** (all `flow:org.vibevm/*`, UPL-1.0):
  - *Book core (edition 0.1.0):* two-process-model, **wal 0.2.0**
    (the rest are 0.1.0), sync-from-code, atomic-commits,
    addressable-specs, decision-records, conflict-protocol,
    campaign-plans, discovery-prompt, attribution-policy.
  - *Project practices (edition 0.2.0):* operating-modes,
    health-audit, manual-tests, secrets-hygiene, licensing,
    source-mirrors, spec-genres, comparative-research, managed-blocks,
    qualified-naming, tool-design-lessons.
  - **Three skills** across the set: `wal-status` (in wal),
    `health-audit`, `draft-eula` (in licensing).
- **Two members are canon over the Discipline:** `wal` and
  `campaign-plans`. Owner ruling: the package is canonical, and
  `flow:core-ai-native`'s `06-WAL-CONVENTION` + `05-CAMPAIGN-FORM`
  defer to the packages **from core-ai-native's next release** (a
  deferring edit that is NOT part of this session — see open item 2).
- Close panel: **self-check all green (exit 0)** with every redbook
  package in-tree; all manifests parse; boot slots collision-free
  (see the grid in the repo map); umbrella 0.2.0 pins 21 members with
  zero non-exact pins.

## Active blocker

**None.** The collection is complete and mirrored; the tree is clean.
The remaining work is all owner-court (publish timing, the deferring
edit into core-ai-native) — nothing is blocked, nothing is half-done.

## The open items (owner-court — none is a standing mandate)

1. **Publish the redbook collection to `vibespecs`.** Order:
   **members first, the umbrella LAST**, so the umbrella's exact pins
   resolve. `wal 0.2.0` must publish before the umbrella (the registry
   today has only `wal 0.1.0`). The live registry is exactly 6 repos
   (3 active `org.vibevm.*` at UPL-1.0, 3 archived `flow-*`). `gh` is
   ABSENT on this box — the publisher path is `vibe registry publish`,
   not the GitHub CLI.
2. **The Discipline-defers edit** — `06-WAL-CONVENTION.md` and
   `05-CAMPAIGN-FORM.md` inside `flow:core-ai-native` should shrink to
   a pointer at the canonical redbook packages. Rides the NEXT
   core-ai-native version bump; do NOT ship two independent
   definitions in the meantime.
3. **License stragglers** — some older local manifests (fixtures, a
   few vibedeps-era copies) still say `license = "EULA"` while the
   published trio and all redbook packages say `UPL-1.0`. Align at
   their next publish.
4. **The vibevm-PRODUCT open items** predate the collection and still
   stand (from the eighth/prior campaigns; the WAL history carries the
   detail): registry publish of the discipline families (rust
   **0.7.0** / ts **0.6.0** / core **0.7.0** / the two `-mcp` at 0.7.0
   / 0.6.0 — owner call); a TS-STACK step in `self-check.sh`;
   colon-free fact-store slot names (today `sha256:<hex>.json` lands
   as an NTFS alternate data stream); `vibe install --refresh <pkg>`
   ergonomics; the `app` kind; the Stage-B delivery experiments
   (`spec/terraforms/TCG-STAGE-B-DELIVERY-PLAN-v0.1.md`); vibe-mcp
   rebase onto mcp-core; PROP-025 v2 shims.

## Next-steps recipe (if the owner says "publish the collection")

```sh
# Members first (each is a self-contained flow package dir):
#   vibe registry publish packages/org.vibevm/<member>/v<ver>
# Publish wal 0.2.0 and the other nine 0.1.0 core members, then the
# eleven wave-2 members, THEN the umbrella editions last:
#   vibe registry publish packages/org.vibevm.world/redbook/v0.1.0
#   vibe registry publish packages/org.vibevm.world/redbook/v0.2.0
# The token is read from the per-user file with no echo (secrets
# hygiene). Verify each landed:  (gh is absent — use the web or the
# publisher's own output).
```

Do NOT publish without the owner's explicit word (the standing
posture). The publish surface is scope-locked to the `vibespecs` org.

## Non-obvious findings (this session)

- **The edition model absorbs growth cleanly.** A second edition is a
  new umbrella version with refreshed exact pins; the prior edition
  stays byte-identical. Members and editions are decoupled — a member
  bump only moves the editions that re-pin it.
- **PROP-028's aggregator law bent deliberately for a *collection*.**
  Content-light umbrella + exact pins were kept; `kind=stack` and
  unison versioning were dropped (a collection is a `flow`, its
  members are independent bricks; the edition model supplies the
  tested-set semantics). If a second collection ever appears, a
  one-page PROP recording the "collection" family shape is worth it.
- **Three members target non-default audiences** (managed-blocks,
  tool-design-lessons → tool authors; qualified-naming → ecosystem
  designers). Their boot snippets are deliberately SMALL — "read once
  while building", not standing per-session instructions. A legitimate
  flow shape the collection now demonstrates.
- **The boot-slot grid is the allocator.** 20 in-tree snippets at
  03/05/10/15/17/25/35/40/42/44/45/50/52/55/57/60/62/65/67/70, plus
  **20 and 30 RESERVED** for the published `sync-from-code` and
  `atomic-commits` (which live in the registry / fixtures, not in
  `packages/`). A wave-3 member must claim an unused slot.
- **The book ships verbatim, Russian-only for now.**
  `spec/book/README.md` records the edition plan: `en/` is reserved;
  once it exists the English text takes priority and the Russian
  stays as translation. Chapter internal refs (e.g. the Safe Harbor
  note) resolve at the book's own public home, not inside the package.
- **`attribution-policy` ships concealment as the DEFAULT** (owner
  ruling) — a human-authored surface with the honest jurisdictional
  rationale traveling WITH the rule, the single-place law, and the
  disclosure posture documented first-class (incl. where it is
  mandatory). It is the collection's own mirror of this repo's Rule 1.

## Repository map (what the collection added)

```
vibevm/
├─ packages/org.vibevm/
│   ├─ redbook/v0.1.0/            umbrella, edition 1: pins 10 core members
│   │   └─ spec/book/ru/          chapters 1–3 (verbatim); spec/book/README
│   ├─ redbook/v0.2.0/            umbrella, edition 2: pins all 21 members
│   │   └─ spec/book/ru/          (same book, carried forward)
│   ├─ two-process-model/v0.1.0/  the foundation flow (coprocessors, IPC)
│   ├─ wal/v0.2.0/                CANON: two-file model + wind-down/resume +
│   │                              wal-status skill (supersedes the 0.1.0 fixture)
│   ├─ addressable-specs/  decision-records/  conflict-protocol/
│   ├─ campaign-plans/v0.1.0/     CANON: the cold-executable plan format
│   ├─ discovery-prompt/v0.1.0/   the DISCOVERY prompt, packaged VERBATIM
│   ├─ attribution-policy/v0.1.0/ default concealment + disclosure alt
│   ├─ operating-modes/  health-audit/  manual-tests/  secrets-hygiene/
│   ├─ licensing/        source-mirrors/  spec-genres/  comparative-research/
│   └─ managed-blocks/   qualified-naming/  tool-design-lessons/
└─ spec/WAL.md                    updated: ninth campaign, waves 1 & 2

Every member: kind=flow, UPL-1.0, spec/boot/<NN>-flow-<name>.md +
spec/flows/<name>/*.md + README.md + LICENSE.md; the atomic-commits
fixture is the style base. sync-from-code and atomic-commits are NOT
here — they are the already-published trio (with wal), re-homed into
the collection by the umbrella's pins.
```

The vibevm PRODUCT tree (crates/, the discipline families in
packages/, spec/modules, spec/terraforms) is unchanged by this
session — it stands where the eighth campaign (total naming coherence)
left it: the rust family at **0.7.0**, typescript at **0.6.0**,
core-ai-native at **0.7.0**, ten binaries under the family names, both
MCP servers live.

## Standing policies in force (long form)

- **redbook is CANON over the Discipline where they overlap** (owner
  ruling this session): `flow:wal` owns the WAL convention,
  `flow:campaign-plans` owns the plan format; the Discipline's copies
  defer from its next release. redbook is pure method (survives with
  only git + a markdown editor); the Discipline is code-enforced rigor
  (cards, gates, checkers per language). Complementary layers.
- **The edition model:** the umbrella version is the edition number;
  an edition pins every member exactly; members move on their own
  lines between editions; a new edition re-pins.
- **The collection is a product, not a dependency of this repo.** It
  is deliberately absent from vibevm's own `vibe.toml`; this repo
  authors and publishes it for other projects.
- **The book is the source of the spirit**, shipped verbatim; English
  edition to lead once written, Russian the reference until then.
- **Publish held for the owner's word**, scope-locked to `vibespecs`;
  the four `CLAUDE.md` rules, clean-room, production-grade/no-MVP, and
  every foundation invariant unchanged.

## Recent commit chain (this session, newest first)

```
d28de5c docs(wal): redbook wave-2 checkpoint
01f1cdc feat(redbook): edition 0.2.0 — the project-practice wave
a07f897 feat(redbook): tool-design-lessons 0.1.0
fcc209b feat(redbook): qualified-naming 0.1.0
4d549d4 feat(redbook): managed-blocks 0.1.0
382ce72 feat(redbook): comparative-research 0.1.0
ab69a86 feat(redbook): spec-genres 0.1.0
ff4ccdf feat(redbook): source-mirrors 0.1.0
5ab0da2 feat(redbook): licensing 0.1.0
3115dc2 feat(redbook): secrets-hygiene 0.1.0
742e6eb feat(redbook): manual-tests 0.1.0
e6f3e83 feat(redbook): health-audit 0.1.0
1bddd60 feat(redbook): operating-modes 0.1.0
a29f009 docs(wal): the redbook collection checkpoint
68ace63 feat(redbook): the umbrella, edition 0.1.0
7b39849 feat(redbook): attribution-policy 0.1.0 — default concealment
20dc8fb feat(redbook): discovery-prompt 0.1.0 — packaged verbatim
d956413 feat(redbook): campaign-plans 0.1.0 — the plan-format canon
ec6b5b5 feat(redbook): wal 0.2.0 — the canonical WAL convention
85aec67 feat(redbook): conflict-protocol 0.1.0
0f2f563 feat(redbook): decision-records 0.1.0
006166d feat(redbook): addressable-specs 0.1.0
f23999d feat(redbook): two-process-model 0.1.0 — the foundation flow
```

(Before these: the eighth campaign — total naming coherence — ending
`89b5210`…`f96b737`; see the WAL eighth-campaign section.)

## Quick-start (verify the tree)

```sh
bash tools/self-check.sh; echo "EXIT=$?"        # must be 0
# Validate every redbook manifest parses + the umbrella pins resolve shape:
python -c "import tomllib,glob;[tomllib.load(open(p,'rb')) for p in glob.glob('packages/org.vibevm/*/v0.*/vibe.toml')]" \
  && echo "manifests OK"
# Confirm product-agnosticism (expect empty):
grep -rilE 'conform|specmap|vibedeps|xtask' \
  packages/org.vibevm/{operating-modes,health-audit,manual-tests,secrets-hygiene,licensing,source-mirrors,spec-genres,comparative-research,managed-blocks,qualified-naming,tool-design-lessons,two-process-model,addressable-specs,decision-records,conflict-protocol,campaign-plans,discovery-prompt,attribution-policy,wal}/v0.*/spec 2>/dev/null || echo "clean"
```

The WAL supersedes this snapshot wherever they diverge. Session-resume
phrase: `восстанови сессию` (boots into a status report and waits —
the open items above are the owner's call, not a standing mandate).
