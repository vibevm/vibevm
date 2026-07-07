# The AI-Native Code Discipline — core (flow:org.vibevm/core-ai-native)

The language-independent core of the Discipline: principles, the pattern-card
format, the executable-scaffold catalog, the operating playbooks (raid,
sweep, campaign form, WAL convention), and the mechanism specs the shipped
checkers implement. Code optimal for **comprehension and safe modification
by AI agents — including weak readers** (small models in swarms maintaining
frontier-authored code).

This package is prompt content only. The runnable half — the checkers, the
per-language cards, the guides — ships in each language stack
(`stack:org.vibevm/rust-ai-native-lang` first: `conform-rust`, `specmap-rust`,
`discipline-rust`, the Rust GUIDE and cards).

## Reading order (human reviewer / strong author)

1. `spec/00-MANIFESTO.md` — mission, axioms, the central law, §8 the package map. Start here.
2. `spec/01-PATTERN-CARD-FORMAT.md` — the format every pattern card is written in.
3. `spec/02-EXECUTABLE-SCAFFOLDS.md` — the nine runnable-capital classes.
4. The active language stack's GUIDE (e.g. `rust/GUIDE-AI-NATIVE-RUST.md` in the Rust stack).
5. `spec/03-RAID-PLAYBOOK.md` + `spec/04-SWEEP-PLAYBOOK.md` + `spec/05-CAMPAIGN-FORM.md` — campaigns and the standing sweep.
6. `spec/06-WAL-CONVENTION.md` — session-durable project state (optional but preferred).
7. `spec/mechanisms/` — ENGINE-CONFORM, PROP-014 (specmap), BROWNFIELD-PROTOCOL, LEDGER-INTENT: the normative mechanism specs; `spec://core-ai-native/mechanisms/…` is what code tags cite.
8. `spec/appendix/` — `CONTRADICTION-MAP.md` (synthesis provenance) and `ATLAS.md` (findings ledger).

## The two load-bearing results behind everything

- **Central law (Manifesto §3):** idiomatic inside the file, engineered
  around the file. Surface stays in-distribution (OOD syntax collapses
  models); strictness moves to types, contracts, meta, and the verification
  loop.
- **Runnable capital (Manifesto §5):** explanation capital must be
  executable. Weak agents leapt from executable scaffolds, not prose. Hence
  the nine-class catalog — and why every procedure here is backed by a
  shipped tool, not a description of one.

## Status and honesty

BETA. Maturity is tagged throughout: [E-strong] (benchmark-backed), [E-mid]
(adjacent evidence), [E-hyp] (first-principles, pilot-gated). The central
open question — does the executable-scaffold advantage transfer from
*generation* to *modification* — is unproven and is the pilot's job (see
`spec/appendix/CONTRADICTION-MAP.md` C-7). A discipline that names its
failure modes is more trustworthy than one that hides them.
