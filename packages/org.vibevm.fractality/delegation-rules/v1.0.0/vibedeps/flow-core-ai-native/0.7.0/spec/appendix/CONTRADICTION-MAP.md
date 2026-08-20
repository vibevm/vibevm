# Contradiction Map — Synthesis Provenance
**Discipline v0.2 · BETA · appendix**

*Per Charter principle B3 (contradiction is data), the highest-value research output is where sources disagree — with each other and with our hypotheses. An empty contradiction map means shallow research. Each entry: the conflict, the evidence on each side, the resolution, and which discipline decision it drove.*

## C-1 — "AI-native = stricter/more meta" (H1) vs "engineered dialects underperform" (H5)
- **Side A (H1):** stricter, more machine-checkable form helps comprehension/modification. Evidence: type-error dominance, compiler-as-oracle (theory + benchmark).
- **Side B (H5):** models work best on in-distribution idiomatic code; engineered surface goes OOD and underperforms. Evidence: EsoLang 0–11% on unfamiliar surface (benchmark).
- **Resolution:** **split by location.** Surface stays idiomatic (H5 wins for syntax); strictness moves to the envelope — types, contracts, meta, verification (H1 wins for structure). → **Central law** (Manifesto §3, Guide §0). The reversal was forced by dated reading: the OOD collapse is recovered by tools + current models (R2C-007), so "stricter envelope" is safe given a verification loop.

## C-2 — "Specs/context files help" vs "AGENTbench: context files barely help, cost +20%"
- **Side A:** spec-driven development, requirement traceability aid comprehension (our production practice).
- **Side B:** AGENTbench (benchmark): human context files +4%, generated ones negative, all +20% inference cost.
- **Resolution:** the authors' own conclusion is "minimal requirements only" — not "no specs." Bloat that triggers unbounded exploration is the harm, not specification. → **Minimal-sufficiency delivery** (Manifesto §6; card Band-3 extract; lazy-push). Caveat the authors flag: low-parametric-knowledge settings (ours) likely benefit MORE from specs (R2C-009).

## C-3 — "Written strategy transfers capability" vs "only executable scaffolds transfer"
- **Side A (our prior instinct):** a good prose explanation from a strong model lifts a weak one.
- **Side B:** EsoLang follow-up (benchmark): +Text ~0 effect (Sonnet 12→12); +Lib transformative (12→64).
- **Resolution:** **executable beats prose for capability transfer.** → **Runnable capital** (Manifesto §5; the entire scaffold catalog). This reversed our own prior-turn position; recorded as a death so it is not re-nucleated.

## C-4 — "Type-constrained decoding cuts errors 74.8%" vs "no Rust implementation exists"
- **Side A:** type-constrained decoding is highly effective (benchmark, TypeScript).
- **Side B:** the method is per-language manual work; only TypeScript exists; Rust's traits/lifetimes are far harder (the authors' repo, primary source).
- **Resolution:** route the oracle to where it exists — **post-generation `cargo check` loop** (Class E) for Rust today; constrained decoding is a **future tool** (`vibe-tcg`), staged, standing on rust-analyzer rather than reimplementing the type system. The 74.8% does not transfer for free. → Guide §12, tcg spec.

## C-5 — Rust benchmark conflict: 58% (SWE-bench Multilingual) vs 10–17% (Multi-SWE-bench)
- **Side A:** Rust resolves well (highest of 9 languages at 58%).
- **Side B:** Rust resolves poorly (10–17%).
- **Resolution:** **difficulty mix, not Rust-unfriendliness.** Multi-SWE-bench is harder by construction (77% medium+hard) and Rust PRs are large by nature; within a fixed difficulty tier, the compiler's guidance outweighs the larger-edit burden (R2C-006). Rust failure correlates with EDIT SIZE. → drives the locality/size/ownership rules (Guide §1–2; cards D, I attack edit-size directly).

## C-6 — Optimism vs the floor: "current models are fine" vs "weak models stay near floor"
- **Side A (owner, dated, correct):** 2026 models recover OOD via tools; pessimistic readings are stale (3-month-old generations, one-shot prompting). Evidence: R2C-007 (same tasks ~4%→~90–100% in 3 months).
- **Side B:** even WITH executable scaffolds, Haiku-4.5 stayed near the floor (4–7/80); resources amplify, don't create capability.
- **Resolution:** **both true, scoped by reader capability.** Optimism holds for Sonnet/GPT-mini class; a floor remains for the weakest tier (and Qwen-32B may sit lower on some axes). → the discipline **lowers** the floor (consume-only scaffolds for the weakest tier; build/use boundary, scaffold catalog §4) but does not claim to **remove** it (Manifesto §7). This is the open pilot question (R4).

## C-7 — Unresolved / open (honest)
- **Transfer generation→modification:** every scaffold's value is shown for *generation*; transfer to *comprehension/modification* of in-distribution Rust is [E-mid], unmeasured on our codebase. **No source resolves this.** It is the central pilot validation target.
- **Build/use boundary:** whether weak agents can *parameterize* scaffolds (Classes A/I) or only *consume* them (G/H) — first-principles, unmeasured.
- **H6 uniformity:** no controlled measurement of internal-uniformity effect exists in the literature; our own hypothesis, pilot candidate.

These open items are why the package is BETA and why every card carries a falsifiable prediction in place of a present measurement.
