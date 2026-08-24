# MARKUP-B5 — `go-ai-native-lang` v0.1.0 {#root}

**Phase:** B (markup, facts pass). **Executor:** Opus. **Reviewer:** the boss,
who owns sense-preserving splits, anchor names and `audience`.
**Corpus:** `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/`.

**All twenty-nine locked conventions in
[`MARKUP-B1.md`](MARKUP-B1.md#locked) bind this batch** — fifteen from B1a, six
from B1b, eight from B2. They were paid for; do not re-litigate them.
`core-ai-native` v0.8.0 is fully marked (943 units over sixteen files) and is
the reference shape.

## Scope {#scope}

**19 files, ~411 facts** — the whole live slot except what the excludes already
drop (`LICENSE.xml` by file name; `spec/cards/INDEX.md` as a derived index).
Nothing here is a superseded slot, a legacy projection, or a book.

**Out of scope and not yours:** anything under `crates/` — including
`crates/vendor/**`, which is machine-written by `cargo xtask sync-engines` and
where a hand edit is the exact failure the floor's step 6 exists to catch.

## What is different about this batch {#different}

This is the **first batch outside `core-ai-native`** and the first for a
language stack, so three things change:

- @fact:B5-GUIDE-IS-THE-BULK `spec/go/GUIDE-AI-NATIVE-GO.md` is the largest file and a **language
  guide**, not a mechanism spec: version floors, gate lists, licence flags,
  suppression policy. Most of it is `@impl/done` — these are rules in force —
  but claims about **Go the language** («gofmt ended formatting debates») are
  outside-world facts and ruling 10 sends them to `@spec/done`.
- @fact:B5-CARDS-ARE-A-GENRE `spec/cards/*.md` are **pattern cards** in the format
  `01-PATTERN-CARD-FORMAT.xml` defines — Band 1 identity, Band 2 justification,
  Band 3 a fenced `card-ops` block. **The Band-3 fence is code and carries no
  markers** (`##FENCE-AWARE`); its `trigger:` / `routine:` / `checker:` lines
  are inside it and are not units.
- @fact:B5-GO-CARRIER-IS-A-COMMENT Go's spec carrier is `//spec:verifies <uri> r=<N>` and its siblings —
  a **line comment**, not an attribute. Where the guide documents that syntax,
  it is prose about a carrier; mark it, do not confuse it with a marker.

## Two things this batch is likely to surface {#expect}

Named so they are recognised rather than rediscovered:

- @fact:B5-EXPECT-VERSION-CLAIMS The guide states tool versions and licence facts (Go 1.24,
  staticcheck MIT, golangci-lint GPL-3.0). Those are checkable and may be
  **stale** — that is a finding, reported, never fixed.
- @fact:B5-EXPECT-ENGINE-HISTORY DRIFT-036 found this package's vendored engine had frozen at
  vendoring time and missed what the gate carried while it sat outside.
  It is synced now. **If a document claims something about the engine version
  it ships, check it against the tree before marking** — that exact claim was
  measured on 2026-07-26 and held, but the neighbourhood has form.

## Boundaries {#boundaries}

- **Semantic edits are forbidden.** Markers, sense-preserving splits, anchors.
  A semantic problem is **reported, never fixed**.
- **Do not touch `spec/**` at the repository root**, `crates/` anywhere, any
  other package, or `campaigns/**`.
- **Do not run any `vibe` command and do not run `tools/self-check.sh`.**
- **Do not commit.** The reviewer reads the diff and commits.

## Acceptance {#acceptance}

```bash
cargo run -q -p vibe-cli --bin vibe -- progress check --no-cache --campaign campaigns/packages-2026-09
```

Zero unmarked units in the 19 files under `--exhaustive`; every marked unit
anchored; no id collision within a file across the **one case-sensitive address
space** shared with heading anchors; `git diff` shows markers, splits and
anchors and nothing else.

## Report back {#report}

Per-file counts · every `@unknown` with its text and why · every semantic
problem seen and not fixed · any case the twenty-nine conventions did not
decide. Ten batches have run; **every one found a factual error in its own
brief by measuring.** If this one is wrong, say so with the measurement.
