# Contradiction Map — Synthesis Provenance {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · BETA · appendix** @status:impl/done

@fact:contradiction-is-the-highest-value-output *Per Charter principle B3 (contradiction is data), the highest-value research output is where sources disagree — with each other and with our hypotheses.* @status:spec/done

@fact:EMPTY-MAP-MEANS-SHALLOW-RESEARCH *An empty contradiction map means shallow research.* @status:impl/done

@fact:ENTRY-CARRIES-FOUR-PARTS *Each resolved entry: the conflict, the evidence on each side, the resolution, and which discipline decision it drove. One entry is deliberately unresolved — C-7 holds the open questions, and the fourth part it drove is the package's BETA status (below).* @status:impl/done

## C-1 — "AI-native = stricter/more meta" (H1) vs "engineered dialects underperform" (H5) {#c-1-stricter-vs-in-distribution}
- @fact:c-1-side-a **Side A (H1):** stricter, more machine-checkable form helps comprehension/modification. Evidence: type-error dominance, compiler-as-oracle (theory + benchmark). @status:spec/done
- @fact:c-1-side-b **Side B (H5):** models work best on in-distribution idiomatic code; engineered surface goes OOD and underperforms. Evidence: EsoLang 0–11% on unfamiliar surface (benchmark). @status:spec/done
- @fact:C-1-RESOLUTION **Resolution:** **split by location.** Surface stays idiomatic (H5 wins for syntax); strictness moves to the envelope — types, contracts, meta, verification (H1 wins for structure). → **Central law** (Manifesto §3, Guide §0). The reversal was forced by dated reading: the OOD collapse is recovered by tools + current models (R2C-007), so "stricter envelope" is safe given a verification loop. @status:impl/done

## C-2 — "Specs/context files help" vs "AGENTbench: context files barely help, cost +20%" {#c-2-specs-vs-context-cost}
- @fact:c-2-side-a **Side A:** spec-driven development, requirement traceability aid comprehension (our production practice). @status:spec/done
- @fact:c-2-side-b **Side B:** AGENTbench (benchmark): human context files +4%, generated ones negative, all +20% inference cost. @status:spec/done
- @fact:C-2-RESOLUTION **Resolution:** the authors' own conclusion is "minimal requirements only" — not "no specs." Bloat that triggers unbounded exploration is the harm, not specification. → **Minimal-sufficiency delivery** (Manifesto §6; card Band-3 extract; lazy-push). Caveat the authors flag: low-parametric-knowledge settings (ours) likely benefit MORE from specs (R2C-009). @status:impl/done

## C-3 — "Written strategy transfers capability" vs "only executable scaffolds transfer" {#c-3-written-vs-executable}
- @fact:c-3-side-a **Side A (our prior instinct):** a good prose explanation from a strong model lifts a weak one. @status:spec/done
- @fact:c-3-side-b **Side B:** EsoLang follow-up (benchmark): +Text ~0 effect (Sonnet 12→12); +Lib transformative (12→64). @status:spec/done
- @fact:C-3-RESOLUTION **Resolution:** **executable beats prose for capability transfer.** → **Runnable capital** (Manifesto §5; the entire scaffold catalog). This reversed our own prior-turn position; recorded as a death so it is not re-nucleated. @status:impl/done

## C-4 — "Type-constrained decoding cuts compile errors 75.3%/70.2%" vs "no Rust implementation exists" {#c-4-tcg-without-rust}
- @fact:c-4-side-a **Side A:** type-constrained decoding is highly effective (benchmark, TypeScript). @status:spec/done
- @fact:c-4-side-b **Side B:** the method is per-language manual work; only TypeScript exists; Rust's traits/lifetimes are far harder (the authors' repo, primary source). @status:spec/done
- @fact:C-4-RESOLUTION **Resolution:** route the oracle to where it exists — **post-generation `cargo check` loop** (Class E) for Rust today; constrained decoding is a **future tool** (`vibe-tcg`), staged, standing on rust-analyzer rather than reimplementing the type system. The 75.3%/70.2% (synthesis/translation — DR2-012's canonical pair) does not transfer for free. → Guide §12, tcg spec. @status:impl/done

## C-5 — Rust benchmark conflict: 58% (SWE-bench Multilingual) vs 10–17% (Multi-SWE-bench) {#c-5-rust-benchmark-conflict}
- @fact:c-5-side-a **Side A:** Rust resolves well (highest of 9 languages at 58%). @status:spec/done
- @fact:c-5-side-b **Side B:** Rust resolves poorly (10–17%). @status:spec/done
- @fact:C-5-RESOLUTION **Resolution:** **difficulty mix, not Rust-unfriendliness.** Multi-SWE-bench is harder by construction (77% medium+hard) and Rust PRs are large by nature; within a fixed difficulty tier, the compiler's guidance outweighs the larger-edit burden (R2C-006). Rust failure correlates with EDIT SIZE. → drives the locality/size/ownership rules (Guide §1–2; cards D, I attack edit-size directly). @status:impl/done

## C-6 — Optimism vs the floor: "current models are fine" vs "weak models stay near floor" {#c-6-optimism-vs-the-floor}
- @fact:c-6-side-a **Side A (owner, dated, correct):** 2026 models recover OOD via tools; pessimistic readings are stale (3-month-old generations, one-shot prompting). Evidence: R2C-007 (same tasks ~4%→~90–100% in 3 months). @status:spec/done
- @fact:c-6-side-b **Side B:** even WITH executable scaffolds, Haiku-4.5 stayed near the floor; resources amplify, don't create capability. Evidence: R2C-008 (the three-condition weak-agent test that includes Haiku 4.5 — it publishes per-model figures for Sonnet 4.6, 12→64, and GPT-5.4-mini, 5→53, and none for Haiku, so no Haiku score is on record here). @status:spec/done
- @fact:C-6-RESOLUTION **Resolution:** **both true, scoped by reader capability.** Optimism holds for Sonnet/GPT-mini class; a floor remains for the weakest tier (and Qwen-32B may sit lower on some axes). → the discipline **lowers** the floor (consume-only scaffolds for the weakest tier; build/use boundary, scaffold catalog §4) but does not claim to **remove** it (Manifesto §7). This is the open pilot question (R4). @status:impl/done

## C-7 — Unresolved / open (honest) {#c-7-unresolved-open}
- @fact:C-7-OPEN-TRANSFER **Transfer generation→modification:** every scaffold's value is shown for *generation*; transfer to *comprehension/modification* of in-distribution Rust is [E-mid], unmeasured on our codebase. **No source resolves this.** It is the central pilot validation target. @status:spec/done
- @fact:C-7-OPEN-BUILD-USE-BOUNDARY **Build/use boundary:** whether weak agents can *parameterize* scaffolds (Classes A/I) or only *consume* them (G/H) — first-principles, unmeasured. @status:spec/done
- @fact:C-7-OPEN-H6-UNIFORMITY **H6 uniformity:** partly measured, not settled. The ATLAS files four records under H6, and one of them measures a uniformity effect: DR1-022 (benchmark, med) — matching the syntactic paradigm of prompt and test examples lifts rule extraction 2.3–125%, read there as support that intra-corpus uniformity is an in-context signal. What no record measures under control is a *codebase's own* internal uniformity; that is the part that stays our hypothesis and the pilot candidate. @status:spec/done

@fact:open-items-are-why-the-package-is-beta These open items are why the package is BETA and why every card carries a falsifiable prediction in place of a present measurement. @status:spec/done
