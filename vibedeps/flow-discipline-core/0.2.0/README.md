# The AI-Native Code Discipline — Package v0.2 (BETA)

This is a full update of the Discipline: a language-independent set of principles, plus a Rust projection, for code optimal for **comprehension and safe modification by AI agents — including weak readers** (small models in swarms maintaining frontier-authored code). It is the product of a four-pass research process (Deep Research ×2, a blind control, a first-principles pass, and deep reading of primary sources), synthesized into an authoring/review artifact whose runtime delivery to weak readers is an extract.

**Two top-level parts:**
- `discipline-v0.2/` — **the Discipline (the product).** Canonical; not bent to fit any pilot.
- `vibevm-terraform/` — **the vibevm-specific adoption plan (the pilot).** Tells vibevm how to adopt the Discipline; does not modify it.

## Reading order (for a human reviewer / strong author)
1. `discipline-v0.2/00-MANIFESTO.md` — mission, axioms, the central law, the honest boundary. Start here.
2. `discipline-v0.2/01-PATTERN-CARD-FORMAT.md` — the format every pattern is written in (GoF × JEP × operational layer).
3. `discipline-v0.2/02-EXECUTABLE-SCAFFOLDS.md` — the nine runnable-capital classes.
4. `discipline-v0.2/rust/GUIDE-AI-NATIVE-RUST.md` — the Rust projection (supersedes GUIDE-RUST-v0.1).
5. `discipline-v0.2/03-RAID-PLAYBOOK.md` — scheduled layered sweeps.
6. `discipline-v0.2/cards/` — the nine scaffold patterns as cards; `INDEX.md` first.
7. `discipline-v0.2/appendix/` — `CONTRADICTION-MAP.md` (synthesis provenance) and `ATLAS.md` (the generated findings ledger).
8. `vibevm-terraform/TERRAFORM-PLAN-v0.3.md` — the pilot adoption.

## The two load-bearing results behind everything
- **Central law (Manifesto §3):** idiomatic inside the file, engineered around the file. Surface stays in-distribution (OOD syntax collapses models); strictness moves to types, contracts, meta, and the verification loop. Recovered-by-tools dating evidence makes the strict envelope safe.
- **Runnable capital (Manifesto §5):** explanation capital must be executable. Weak agents leapt from executable scaffolds, not prose (Sonnet 12→64 vs 12→12). Hence the nine-class catalog.

## Status and honesty
This is BETA. Maturity is tagged throughout: [E-strong] (benchmark-backed), [E-mid] (adjacent evidence), [E-hyp] (first-principles, pilot-gated). The central open question — does the executable-scaffold advantage transfer from *generation* to *modification* — is unproven and is the pilot's job (see `appendix/CONTRADICTION-MAP.md` C-7). Measurement is deferred by design; every card carries a falsifiable prediction in place of a present measurement. A discipline that names its failure modes is more trustworthy than one that hides them.

## After the pilot
The vibevm adoption ends in a terraform REPORT of what the pilot taught — including which Discipline documents the evidence suggests should change. That REPORT is the input to Discipline v0.3. We return to it then.
