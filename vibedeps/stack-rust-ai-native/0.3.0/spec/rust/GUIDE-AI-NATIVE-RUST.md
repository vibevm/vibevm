# AI-Native Rust — The Guide
**Discipline v0.2 · status: BETA · T2 · supersedes GUIDE-RUST-v0.1**

*The projection of the Discipline onto Rust. Rust is the pilot language; other languages are projected after this one is validated on vibevm. This guide absorbs and extends GUIDE-RUST-v0.1 — every rule it had survives here, recast under the central law and the scaffold catalog.*

*A human CAN read and modify AI-Native Rust; it may be less comfortable to write by hand than ordinary Rust, but it remains ordinary idiomatic Rust at the token level. What differs is the envelope: dense machine-checkable metadata, contract-bearing types, executable scaffolds, and a fast per-cell verification loop.*

---

## 0. The law, applied to Rust

> **Idiomatic inside the file; engineered around the file.**

Rust source under this discipline reads as *ordinary idiomatic Rust*. No invented syntax, no exotic dialect — that would incur the out-of-distribution penalty (EsoLang: 0–11% on unfamiliar surface; in-context learning cannot teach it). The strictness lives entirely in the envelope: types, contracts, metadata, verification. We exploit a fact specific to Rust — **the borrow/type checker is already a verifier that converts a class of semantic errors into local, machine-caught ones.** AI-Native Rust maximizes how much intent is expressed in that machine-checkable form. The compiler is a free hallucination detector; we give it as much to check as possible (A3 at the language level).

## 1. Cells — the unit of paging and ownership

*(From GUIDE-RUST-v0.1, retained.)* The **cell** is the unit of modification, closed under paging (R3-001): an editable unit declares its full semantic dependency set so a pager can assemble sufficient context mechanically.

- Default granularity: **module**, with promotion criteria to a larger cell when cohesion demands.
- A cell carries a `#[cell]` manifest attribute naming it and its seams.
- **One cell, one registration point.** Cells import seams + core only, never sibling cells (R-002).
- Ambient coupling — globals, thread-locals, inheritance-style magic, ambient config read outside the composition root — **breaks closure and is forbidden**; reads of shared state are declared (R3-001).
- **Ownership aligns with file boundaries** (R3-013): one cell = one file-set with a single registration point. God-files serialize the swarm and are an anti-pattern (`cards/` anti-pattern set). Shared facts go to append-only ledgers, not shared mutable modules.

## 2. Surface form: naming, layout, position

- **Names are token programs** (R3-004, R-020). Canonical cell name is **computed** from the manifest (`{Variant}{Seam}`). Across contract surfaces: one name = one referent repo-wide; **no shadowing, no synonym pairs**. Structural tokens come from a closed vocabulary. Length is free; ambiguity is not. (Short closure-local bindings are exempt — scope the rule to contract surfaces, not every local.)
- **Contract-first ordering within an item** (R3-002): signature, then invariants, then error contract, then one canonical example, *before* the body. Autoregression makes reading order conditioning order; intent goes first.
- **Position is a resource** (R3-003): safety-critical invariants live at file top or bottom, never the diluted middle. Prefer more, smaller, single-purpose files at equal token mass. A conform check warns on files over a length threshold and on invariant-bearing comments in the middle third.
- **Uniformity is load-bearing** (R3-006, H6): one way per operation. The codebase is the few-shot prompt; a second coexisting idiom becomes false training signal and propagates. Legitimate exceptions are MARKED (`#[spec(deviates, reason)]`) so they do not propagate as imitation.

## 3. The nine scaffolds in Rust

Each is a card in this package's `cards/` (the Rust projection of the language-neutral scaffold catalog `02-EXECUTABLE-SCAFFOLDS.md`); here is the Rust shape and the rule.

- **A — Generators / codegen** (`scaffold-a-generators`). `build.rs` codegen, declarative/proc generators emitting boilerplate cells, FFI bindings, serializers, state-machine transition tables, exhaustive match arms. Committed output is plain in-distribution Rust; the GENERATOR carries the structural decision. *Rule:* where an artifact is mechanically derivable from a smaller spec, ship generator + committed output + determinism check, not hand-maintained output (A3).
- **B — Typed builders / typestate** (`scaffold-b-typed-builders`). Make the statistically-likely wrong call un-representable: typestate (phantom-typed state machines where illegal transitions don't compile), newtypes over primitives at every seam, builders with type-mandatory required fields, sealed traits, `#[must_use]`, no boolean/positional argument soups, no stringly-typed protocol surfaces. *Rule:* seam protocols are encoded in types, not docstrings; the wrong call fails `cargo check`, not a runtime assert (R3-008; 94% of compile errors are type-level).
- **C — Runnable contracts** (`scaffold-c-runnable-contracts`). `debug_assert!` witnessing cross-cell invariants AT USE SITES (R3-009: redundancy is ground truth for a paged reader), contract crates or Kani `requires`/`ensures`/`modifies`, refined-type witnesses, property-test-backed behavioral claims. *Rule:* every load-bearing invariant is witnessed by a runnable assertion or proof where it is relied upon, not only documented at definition.
- **D — Differential / characterization oracles** (`scaffold-d-differential-oracle`). proptest old-vs-new harnesses; `insta` goldens for opaque legacy behavior; fuzz targets as behavior boundaries. *Rule:* no replacement of a non-trivial cell merges without a differential or characterization oracle against prior behavior (R-040). The modification-specific safety net.
- **E — Per-cell fast loop** (`scaffold-e-fast-loop`). Every cell independently compilable + testable in seconds: `discipline-rust fast-loop --cell <crate>` (shipped) + `cargo test -p <cell>`. The agent loop is edit → cell-check → read structured error → edit; first signal < ~60s. *Rule:* whole-repo CI is not an agent loop; the per-cell loop is the substrate that makes every other scaffold's signal fast enough (R3-007).
- **F — Structured, REQ-citing diagnostics** (`scaffold-f-structured-diagnostics`). `thiserror` messages carry a `spec://` REQ URI + a one-line fix-surface hint; conform emits SARIF; custom clippy lints name the rule and the remedy. *Rule:* every custom check emits "violates REQ-X: <why>; fix surface: <where>", never bare free text (R3-011).
- **G — Executable examples / doctests** (`scaffold-g-doctests`). One compiled doctest per public seam showing the ONE canonical construction and use; `examples/` cells that compile in CI. *Rule:* every public seam carries ≥1 compiled doctest of canonical use; behavioral claims in prose are doctest-backed or marked unverified. A doctest that lies fails CI; a comment that lies ships (R2C-004, H4).
- **H — Local simulators / reference models** (`scaffold-h-simulators`). A runnable reference implementation of a protocol/state-machine; an in-memory fake of an external dependency; an executable spec of the resolver's fixpoint the reader can step through. *Rule:* subsystems with non-obvious dynamics ship a runnable model or fake, not a prose description (execution-prediction is where weak models are weakest — CRUXEval ~63% even for strong models).
- **I — Scaffolded edit operations / codemods** (`scaffold-i-codemods`). `cargo`-integrated codemods for "add a cell," "register a variant," "rename across the trait surface"; `syn`-based AST rewrites performing a multi-file change atomically and verifiably. *Rule (provisional, [E-hyp]):* a capability-demanding multi-file edit (Rust's actual failure mode — failure correlates with edit size, R2C-006) is offered as one parameterized checked operation, converting it into a parameter-filling task. Validate in pilot whether weak agents can parameterize these.

## 4. Errors as contract surface

*(From GUIDE-RUST-v0.1, retained and extended.)* One `thiserror` enum per layer; variants carry `#[spec]` REQ edges; `#[track_caller]` on fallible constructors; `anyhow` only at the binary edge; **panics are defects**. Extended by Class F: error messages are agent food — structured, REQ-citing, fix-surface-hinting.

## 5. Registry & flags

*(From GUIDE-RUST-v0.1, retained.)* Flags read once at the composition root; a registry selects cells; **no `if flag` in domain logic** (R-001). Explicit `match` at the composition root over link-time magic — "one match is the system's table of contents." Two tiers: cargo features (code in binary) vs runtime flags (cell selected). The flag registry is data with provenance, birth, and sunset.

## 6. Bans and their escape hatches

Forbidden by default in domain cells; legal with `#[spec(deviates, reason="...")]` and the required machinery:
- **`unwrap`/`expect` in domain logic** → use the error contract; deviation allowed at well-justified boundaries with a reason.
- **Inline assembly** → banned, but legal when programming hardware directly, wrapped and reasoned (the canonical escape-hatch example).
- **Proc-macro magic, `Deref` polymorphism, decision-making `Default`, effectful `From`** → hidden control flow is forbidden (R-021); deviations require reason and machinery.
- **Stringly-typed protocol surfaces, boolean/positional argument soups** → replaced by typed builders (Class B); deviation requires reason.
A ban with no escape hatch is a discipline bug; a deviation with no reason is a code bug.

## 7. Metadata layer (specmap)

*(PROP-014, retained as discipline meta-layer.)* `spec://` URIs; in-source inert attributes `#[spec(implements|verifies|documents|deviates|informs)]` (≤3 edges per item, the specmark budget); two-tier revisions (author-asserted semantic revision + content hash) with **asymmetric invalidation** (spec bump → edges suspect; code change → edges stay valid); a derived deterministic committed index; an orphan ratchet; `deviates` requires a reason. The metadata is the authored retrieval index (R3-012): stable anchors + a uniform one-line what/why per public item, in a fixed grammar the pager consumes.

## 8. Prose discipline (the asymmetric hazard)

Wrong prose is worse than no prose (R2C-004, H4): models condition on in-repo text with high trust, so a lying comment is adversarial input, and the harm exceeds that of absence. Therefore prose near code is **machine-checked** (doctests for behavioral claims, `#[spec(documents)]` edges making drift detectable via spec-rev bumps) or **explicitly trust-labeled** (verified / unverified / aspirational). Misleading log/print strings count too (the harm is the false claim, not the comment syntax). rustdoc remains the human detail layer; duplication with the spec is a spec defect.

## 9. Replacement protocol

*(R-040, retained.)* Replacing a cell ships a **differential oracle** (Class D) against the old cell, plus the `#[spec(verifies)]` edge. Characterization goldens pin opaque legacy behavior; goldens must fail loudly when stale, never auto-update.

## 10. Test matrices

*(R-060, retained.)* Declared test matrices, never `2^n`. Property tests for behavioral surfaces; the differential oracle covers replacement; per-cell tests run in the fast loop.

## 11. How a weak reader actually uses this guide

The weak swarm does **not** read this guide. It receives, per edit, the Band-3 ops extract of whichever cards' triggers fire — a small, activation-matched set (lazy-push, R3-014; minimal sufficiency, AGENTbench). This guide and the cards are the authoring/review artifact for the strong author and the human; the runtime surface for the weak reader is "the right card's routine + checker, when its trigger fires." Cross-cutting concerns the per-edit loop cannot hold are swept by raids (`03-RAID-PLAYBOOK.md`).

## 12. Tooling roadmap pointer
`rust/tools/vibe-tcg.md` specifies a future type-aware constrained-generation tool: generation-time masking to rust-analyzer-validated, discipline-conformant continuations. It is the generation-time complement to the post-generation `cargo check` loop (Class E) — the loop exists today; vibe-tcg is the harder, higher-leverage bet for the weak-agent swarm.

## 13. Wiring the gates in a consumer project

The stack ships everything below; nothing requires the discipline's dev tree.

1. **Install the toolchain.** `vibe install` materialises this package into `vibedeps/`. Then either put the umbrella binary on PATH once — `cargo install --path vibedeps/<stack-slot>/crates/discipline-cli` — or run it in place: `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p discipline-cli --bin discipline-rust -- <args>`. Add `vibedeps/**/target/` to `.gitignore`.
2. **Bootstrap.** `discipline-rust init` writes `conform.toml` (topology-detected roots; every crate exempt-with-a-reason — the pre-adoption posture), `specmap.toml` (your `namespace` + `[[external_specs]]` discovered from the installed packages, so citations of `spec://discipline-core/…` resolve), and the `discipline/registry/` files. Idempotent; `--force` to regenerate.
3. **Take the tags.** Your crates dep the shipped proc-macro:
   ```toml
   # workspace Cargo.toml
   [workspace.dependencies]
   specmark = { path = "vibedeps/<stack-slot>/crates/specmark" }
   ```
   then per crate `specmark.workspace = true`, and modules carry `specmark::scope!("spec://<your-ns>/<doc>#<anchor>")` (§7).
4. **First unit, first index.** Write `spec/PROP-001.md` with an anchored req (`## X {#req-…}` + `` `req r1` ``), tag the implementing module, run `discipline-rust specmap` to mint `specmap.json`, commit it.
5. **The floor.** `discipline-rust floor` = fmt → test → clippy → conform → specmap → test-gate (when the baseline exists). One exit code; per-policy origin lines (a `Defaulted` policy announces itself — never trust a green you didn't configure). This replaces a hand-rolled self-check script.
6. **Adopt crate by crate.** Drain a crate to zero findings (`conform check --scope <crate>`), then flip it into `gated_crates` and drop its `[[exempt]]` entry — the expand-as-you-conform rhythm; a flip must never widen the baseline. The `every-crate-gated-or-exempt` invariant is enforced by the engine on every check.
7. **Procedures.** `vibe skill install` projects `/terraform-rust` (brownfield adoption) and `/discipline-sweep` (the recurring sweep) into your agents; the methods are the core package's playbooks.

## 14. Sweep idioms (Rust)

The recurring sweep's Tier-1 moves (04-SWEEP-PLAYBOOK), in their Rust shape — each proven across the pilot's campaigns:

- **Tests-out split** (danger-band files): move an inline `#[cfg(test)] mod tests` to a sibling `foo/tests.rs` declared `#[cfg(test)] #[path = "foo/tests.rs"] mod tests;`. Cell registration is untouched. Gotchas: the conform frontend parses files standalone, so a non-`#[test]` helper in the tests-out file needs its own `#[cfg(test)]` or its unwraps read as domain; `pub(super)` items cannot be re-exported wider (E0364).
- **Responsibility split** (when the production half alone exceeds the budget): split along the file's seam into module-grain cells; **every new module carries the parent's `scope!` URI** so it stays in the retrieval index (no gated orphan). Measure with the rule (physical `lines().count()`), not the eye.
- **The four doctest idioms** (pub-doctest drain): a TOML round-trip for serde sections (`toml::from_str::<T>(r#"…"#)` — the wire form is the canonical use); a parse one-liner for newtypes (via their `Deref<str>`/`PartialEq<str>` ergonomics); a variant/`Default` assert for bare enums; a construct-and-Display assert for error enums (the Class-F message already cites its REQ, so the example doubles as a navigability demo).
- **Restructure beats testify** (unwrap drain): types carry the invariant — split-first tuples, `let-else`, `next_if`, read-then-advance counters, parser early-returns; `from_validated` beats a fake-fallible signature; a structural `semver::Comparator` beats parsing a formatted string that panics on edge input. `#[spec(deviates)]` is the last resort, and it decays: a deviation whose invariant became encodable is a defect.
- **Flip-only-after-drain**: a crate enters `gated_crates` (or `gated_pub_doctest`) only at zero findings; the collector (`discipline-rust health`) names the promotion candidates and ranks the drain backlog smallest-gap-first.
