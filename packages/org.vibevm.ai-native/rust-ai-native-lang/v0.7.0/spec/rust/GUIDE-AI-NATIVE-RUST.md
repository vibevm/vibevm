# AI-Native Rust — The Guide {#root}

<status stage="spec" state="done"/>

@fact:status-line **Discipline v0.2 · status: BETA · T2 · supersedes GUIDE-RUST-v0.1** @status:impl/done

@fact:projection-onto-rust *The projection of the Discipline onto Rust.* @status:impl/done

@fact:RUST-IS-THE-PILOT-LANGUAGE *Rust is the pilot language; other languages are projected after this one is validated on vibevm.* @status:spec/done

@fact:GUIDE-ABSORBS-AND-EXTENDS-V0-1 *This guide absorbs and extends GUIDE-RUST-v0.1 — every rule it had survives here, recast under the central law and the scaffold catalog.* @status:impl/done

@fact:A-HUMAN-CAN-READ-AI-NATIVE-RUST *A human CAN read and modify AI-Native Rust; it may be less comfortable to write by hand than ordinary Rust, but it remains ordinary idiomatic Rust at the token level.* @status:spec/done

@fact:what-differs-is-the-envelope-lead *What differs is the envelope:* @status:impl/done

- @fact:ENVELOPE-DENSE-MACHINE-CHECKABLE-METADATA *dense machine-checkable metadata,* @status:impl/done
- @fact:ENVELOPE-CONTRACT-BEARING-TYPES *contract-bearing types,* @status:impl/done
- @fact:ENVELOPE-EXECUTABLE-SCAFFOLDS *executable scaffolds,* @status:impl/done
- @fact:ENVELOPE-FAST-PER-CELL-LOOP *and a fast per-cell verification loop.* @status:impl/done

---

## 0. The law, applied to Rust {#law}

> @fact:LAW-IDIOMATIC-INSIDE-ENGINEERED-AROUND **Idiomatic inside the file; engineered around the file.** @status:impl/done

@fact:RUST-SOURCE-READS-AS-ORDINARY-IDIOMATIC-RUST Rust source under this discipline reads as *ordinary idiomatic Rust*. @status:impl/done

@fact:NO-INVENTED-SYNTAX-NO-EXOTIC-DIALECT No invented syntax, no exotic dialect — that would incur the out-of-distribution penalty (EsoLang: 0–11% on unfamiliar surface; in-context learning cannot teach it). @status:spec/done

@fact:strictness-lives-in-the-envelope-lead The strictness lives entirely in the envelope: @status:impl/done

- @fact:STRICTNESS-TYPES types, @status:impl/done
- @fact:STRICTNESS-CONTRACTS contracts, @status:impl/done
- @fact:STRICTNESS-METADATA metadata, @status:impl/done
- @fact:STRICTNESS-VERIFICATION verification. @status:impl/done

@fact:BORROW-CHECKER-IS-ALREADY-A-VERIFIER We exploit a fact specific to Rust — **the borrow/type checker is already a verifier that converts a class of semantic errors into local, machine-caught ones.** @status:spec/done

@fact:MAXIMIZE-MACHINE-CHECKABLE-INTENT AI-Native Rust maximizes how much intent is expressed in that machine-checkable form. @status:impl/done

@fact:COMPILER-IS-A-FREE-HALLUCINATION-DETECTOR The compiler is a free hallucination detector; we give it as much to check as possible (A3 at the language level). @status:spec/done

## 1. Cells — the unit of paging and ownership {#cells}

@fact:CELL-IS-THE-UNIT-OF-MODIFICATION *(From GUIDE-RUST-v0.1, retained.)* The **cell** is the unit of modification, closed under paging (R3-001): an editable unit declares its full semantic dependency set so a pager can assemble sufficient context mechanically. @status:impl/done

- @fact:CELL-DEFAULT-GRANULARITY-IS-THE-MODULE Default granularity: **module**, with promotion criteria to a larger cell when cohesion demands. @status:impl/done
- @fact:CELL-CARRIES-A-CELL-MANIFEST-ATTRIBUTE A cell carries a `#[cell]` manifest attribute naming it and its seams. @status:impl/done
- @fact:ONE-CELL-ONE-REGISTRATION-POINT **One cell, one registration point.** Cells import seams + core only, never sibling cells (R-002). @status:impl/done
- @fact:AMBIENT-COUPLING-IS-FORBIDDEN Ambient coupling — globals, thread-locals, inheritance-style magic, ambient config read outside the composition root — **breaks closure and is forbidden**; reads of shared state are declared (R3-001). @status:impl/done
- @fact:OWNERSHIP-ALIGNS-WITH-FILE-BOUNDARIES **Ownership aligns with file boundaries** (R3-013): one cell = one file-set with a single registration point. God-files serialize the swarm and are an anti-pattern (`cards/` anti-pattern set). Shared facts go to append-only ledgers, not shared mutable modules. @status:impl/done

## 2. Surface form: naming, layout, position {#surface-form}

- @fact:NAMES-ARE-TOKEN-PROGRAMS **Names are token programs** (R3-004, R-020). The canonical cell type name is **computed** from the manifest — `Pascal(variant)` followed by the seam SPELLED AS WRITTEN (`SatDepSolver`, not `SatDepsolver`) — and that is now a description, not a goal: the `cell-name-is-computed` conform rule (`rust-ai-native-conform`, mounted in `go-ai-native-conform` too) reads the `#[cell(seam = "…", variant = "…")]` manifest and reds any declared name whose final path segment is not the composed one. **The rule checks composition only** — it does not check the rest of this clause. Length is free; ambiguity is not. (Short closure-local bindings are exempt — scope to contract surfaces, not every local.) @status:impl/done
- @fact:NAMES-REST-OF-R3-004-IS-UNBUILT The other halves of R3-004 — structural tokens drawn from a closed vocabulary, one name = one referent repo-wide, and no synonym pairs or shadowing on contract surfaces — are **not built**: no such checker exists, and no closed-token vocabulary exists anywhere in the tree (the owner's fork №1 took computed names; the closed-vocabulary variant that would need all three was not taken). They stay specified above as the target; **no backlog entry exists for them yet** — building them needs a token vocabulary plus a referent/uniqueness checker, and a filed entry first. @status:spec/done
- @fact:CONTRACT-FIRST-ORDERING-WITHIN-AN-ITEM **Contract-first ordering within an item** (R3-002): signature, then invariants, then error contract, then one canonical example, *before* the body. Autoregression makes reading order conditioning order; intent goes first. *Specified, not built (→ B-038): nothing checks intra-item ordering. The card that would carry it, `rule-contract-first-ordering`, is still listed among this stack's pending cards ("named, not yet authored") in `cards/INDEX.md`, and no `rust-ai-native-conform` rule inspects the order of parts within an item.* @status:impl/plan
- @fact:POSITION-IS-A-RESOURCE **Position is a resource** (R3-003): safety-critical invariants live at file top or bottom, never the diluted middle. Prefer more, smaller, single-purpose files at equal token mass. This is now enforced, not promised — the long-standing `file-length` check on files over the budget, plus **`invariant-comment-position`**: a comment whose marker is in the configured vocabulary is *buried* when it lands in a file's middle third — line `l` with `lines/3 < l <= 2·lines/3` (integer-divided; for a 120-line file, lines 41–80) — and the rule fires one finding through the normal gate (the Class-F `violates REQ …; fix surface: …` grammar; remedy: move the comment to the file's top or bottom, or split the file). The vocabulary and the floor are root `conform.toml` keys, language-neutral beside `max_file_lines`: `invariant_comment_markers` (default the five labeled markers `INVARIANT:` / `WARNING:` / `PANICS:` / `MUST:` / `NEVER:` — a marker is a labeled tag, not a bare word, so the colon is the markup signal; `SAFETY:` is excluded: in Rust it must hug its `unsafe` block, so it is block-local justification, not a file-level invariant) and `invariant_comment_min_file_lines` (default 120 — below it a «third» means nothing, so the whole file is skipped). Test-context markers are out of scope, and the rule re-checks the vocabulary itself rather than trusting the extractor. Honest vacuum note: these markers are near-absent in this tree (measure: `SAFETY:` 6, `INVARIANT:` 0, `PANICS` 0, rustdoc `# Safety` 0; TS `@invariant` 0), so the rule is shown on fixtures, not host code — a fact about our tidiness, not an argument against a rule the discipline forbids weakening for disuse. @status:impl/done
- @fact:UNIFORMITY-IS-LOAD-BEARING **Uniformity is load-bearing** (R3-006, H6): one way per operation. The codebase is the few-shot prompt; a second coexisting idiom becomes false training signal and propagates. Legitimate exceptions are MARKED (`#[spec(deviates, reason)]`) so they do not propagate as imitation. @status:impl/done
- @fact:FAMILY-PREFIX-RULE **The family-prefix rule (owner policy, 2026-07-07; supersedes the `-rust` suffix rule).** Every named surface of the Rust discipline is language-FIRST: it carries the family stem `rust-ai-native` as a *prefix*, not a `-rust` suffix. The umbrella binary is the family name itself (`rust-ai-native`, over `init` / `floor` / …; its crate `rust-ai-native-cli`); the standalone tools and their crates share `rust-ai-native-<role>` (`rust-ai-native-conform`, `rust-ai-native-specmap`, `rust-ai-native-tcg`, and the libraries `rust-ai-native-conform-frontend`, `rust-ai-native-tcg-bridge`, `rust-ai-native-env-audit`); the server package/crate/binary is `rust-ai-native-mcp` and the agent-visible server name is the family (`rust-ai-native`); the skills are `rust-ai-native-sweep` / `rust-ai-native-terraform`; the token brief is `rust-ai-native-tcg.md` beside `typescript-ai-native-tcg.md`. The earlier suffix policy (`conform-rust` beside `conform-typescript`) is superseded (PROP-028 §2.4): language-NEUTRAL artifacts still stand outside any family stem (the shared engine crates take the core stem `core-ai-native-*`; vibevm's own generic `vibe-*` crates keep their names). The point is unchanged from the uniformity rule above: a name is a token program, and now the WHOLE name — prefix included — sorts and reads a family together. @status:impl/done

## 3. The nine scaffolds in Rust {#scaffolds}

@fact:scaffold-cards-lead Each is a card in this package's `cards/` (the Rust projection of the language-neutral scaffold catalog `02-EXECUTABLE-SCAFFOLDS.md`); here is the Rust shape and the rule. @status:impl/done

- @fact:SCAFFOLD-A-GENERATORS **A — Generators / codegen** (`scaffold-a-generators`). `build.rs` codegen, declarative/proc generators emitting boilerplate cells, FFI bindings, serializers, state-machine transition tables, exhaustive match arms. Committed output is plain in-distribution Rust; the GENERATOR carries the structural decision. *Rule:* where an artifact is mechanically derivable from a smaller spec, ship generator + committed output + determinism check, not hand-maintained output (A3). @status:impl/done
- @fact:SCAFFOLD-B-TYPED-BUILDERS **B — Typed builders / typestate** (`scaffold-b-typed-builders`). Make the statistically-likely wrong call un-representable: typestate (phantom-typed state machines where illegal transitions don't compile), newtypes over primitives at every seam, builders with type-mandatory required fields, sealed traits, `#[must_use]`, no boolean/positional argument soups, no stringly-typed protocol surfaces. *Rule:* seam protocols are encoded in types, not docstrings; the wrong call fails `cargo check`, not a runtime assert (R3-008; 94% of compile errors are type-level). @status:impl/done
- @fact:SCAFFOLD-C-RUNNABLE-CONTRACTS **C — Runnable contracts** (`scaffold-c-runnable-contracts`). `debug_assert!` witnessing cross-cell invariants AT USE SITES (R3-009: redundancy is ground truth for a paged reader), contract crates or Kani `requires`/`ensures`/`modifies`, refined-type witnesses, property-test-backed behavioral claims. *Rule:* every load-bearing invariant is witnessed by a runnable assertion or proof where it is relied upon, not only documented at definition. @status:impl/done
- @fact:SCAFFOLD-D-DIFFERENTIAL-ORACLES **D — Differential / characterization oracles** (`scaffold-d-differential-oracle`). proptest old-vs-new harnesses; `insta` goldens for opaque legacy behavior; fuzz targets as behavior boundaries. *Rule:* no replacement of a non-trivial cell merges without a differential or characterization oracle against prior behavior (R-040). The modification-specific safety net. @status:impl/done
- @fact:SCAFFOLD-E-PER-CELL-FAST-LOOP **E — Per-cell fast loop** (`scaffold-e-fast-loop`). Every cell independently compilable + testable in seconds: `rust-ai-native fast-loop --cell <crate>` (shipped) + `cargo test -p <cell>`. The agent loop is edit → cell-check → read structured error → edit; first signal < ~60s. *Rule:* whole-repo CI is not an agent loop; the per-cell loop is the substrate that makes every other scaffold's signal fast enough (R3-007). @status:impl/done
- @fact:SCAFFOLD-F-STRUCTURED-DIAGNOSTICS **F — Structured, REQ-citing diagnostics** (`scaffold-f-structured-diagnostics`). Two of the three channels are built: `thiserror` error surfaces carry their `#[spec]` REQ edge and cite it in their Display text (the Class-F halves `error-enum-cites-req`, the attribute half, and `error-message-cites-req`, the message half, both in `rust-ai-native-conform`), and conform findings ship as SARIF. The grammar they speak — `violates REQ <uri>: <why>; fix surface: <where>` — is held in exactly one place: the renderer `req_message` plus the acceptor `matches_req_grammar` in the engine (`core-ai-native-conform/src/rules/mod.rs`), with every finding rendering through that one renderer, so the layer of custom checks that already exists cannot mis-spell it. *Not built — the third channel:* a custom clippy lint whose own message names the rule and the remedy. The only route to such a lint is `dylint`, whose library links the compiler internals through `#![feature(rustc_private)]` and does not build on `stable`; this project pins `stable` (`rust-toolchain.toml`, the only toolchain file in the tree) deliberately. The gap is exactly one named thing — Rust has no vehicle that SEES TYPES (conform reads syntax) — not a grammar gap; the promise stands, the build is planned, and the route is recorded: `BACKLOG.md {#b-050}` (owner ruling 2026-08-04). *Rule:* every custom check emits "violates REQ-X: <why>; fix surface: <where>", never bare free text (R3-011). The parity behind it — no projection enforces the discipline more weakly than another without a recorded reason — is a discipline law in the manifesto (`spec://org.vibevm.ai-native/core-ai-native/00-MANIFESTO#PARITY-ACROSS-PROJECTIONS`); the asymmetry that TypeScript has this channel built and Rust does not yet is held by its sibling law (`spec://org.vibevm.ai-native/core-ai-native/00-MANIFESTO#PARITY-GAP-IS-NEVER-SILENT`), recorded with a reason and a route, not in silence. @status:impl/plan
- @fact:SCAFFOLD-G-EXECUTABLE-EXAMPLES **G — Executable examples / doctests** (`scaffold-g-doctests`). One compiled doctest per public seam showing the ONE canonical construction and use; `examples/` cells that compile in CI. *Rule:* every public seam carries ≥1 compiled doctest of canonical use; behavioral claims in prose are doctest-backed or marked unverified. A doctest that lies fails CI; a comment that lies ships (R2C-004, H4). @status:impl/done
- @fact:SCAFFOLD-H-LOCAL-SIMULATORS **H — Local simulators / reference models** (`scaffold-h-simulators`). A runnable reference implementation of a protocol/state-machine; an in-memory fake of an external dependency; an executable spec of the resolver's fixpoint the reader can step through. *Rule:* subsystems with non-obvious dynamics ship a runnable model or fake, not a prose description (execution-prediction is where weak models are weakest — CRUXEval ~63% even for strong models). @status:impl/done
- @fact:SCAFFOLD-I-CODEMODS **I — Scaffolded edit operations / codemods** (`scaffold-i-codemods`). `cargo`-integrated codemods for "add a cell," "register a variant," "rename across the trait surface"; `syn`-based AST rewrites performing a multi-file change atomically and verifiably. *Rule (provisional, [E-hyp]):* a capability-demanding multi-file edit (Rust's actual failure mode — failure correlates with edit size, R2C-006) is offered as one parameterized checked operation, converting it into a parameter-filling task. Validate in pilot whether weak agents can parameterize these. @status:impl/done

## 4. Errors as contract surface {#errors}

@fact:ONE-ERROR-ENUM-PER-LAYER *(From GUIDE-RUST-v0.1, retained and extended.)* One `thiserror` enum per layer; variants carry `#[spec]` REQ edges; `#[track_caller]` on fallible constructors; `anyhow` only at the binary edge; **panics are defects**. *Specified, not built: the REQ-edge half ships and is checked (`error-enum-cites-req` in `rust-ai-native-conform`), and `no-unwrap-in-domain` carries the panic ban — but `#[track_caller]` is on no fallible constructor anywhere in the shipped surface (this stack's crates, the host's `crates/`, or `research/rust-demo/`), and no checker requires it. That clause is a wish.* @status:spec/done

@fact:ERROR-MESSAGES-ARE-AGENT-FOOD Extended by Class F: error messages are agent food — structured, REQ-citing, fix-surface-hinting. @status:impl/done

## 5. Registry & flags {#registry-and-flags}

@fact:FLAGS-READ-ONCE-AT-THE-COMPOSITION-ROOT *(From GUIDE-RUST-v0.1, retained.)* Flags read once at the composition root; a registry selects cells; **no `if flag` in domain logic** (R-001). @status:impl/done

@fact:EXPLICIT-MATCH-OVER-LINK-TIME-MAGIC Explicit `match` at the composition root over link-time magic — "one match is the system's table of contents." @status:impl/done

@fact:TWO-TIERS-OF-FLAGS Two tiers: cargo features (code in binary) vs runtime flags (cell selected). @status:impl/done

@fact:FLAG-REGISTRY-IS-DATA-WITH-PROVENANCE The flag registry is data with provenance, birth, and sunset. @status:impl/done

## 6. Bans and their escape hatches {#bans}

@fact:forbidden-by-default-lead Forbidden by default in domain cells; legal with `#[spec(deviates, reason="...")]` and the required machinery: @status:impl/done
- @fact:BAN-UNWRAP-EXPECT-IN-DOMAIN-LOGIC **`unwrap`/`expect` in domain logic** → use the error contract; deviation allowed at well-justified boundaries with a reason. @status:impl/done
- @fact:BAN-INLINE-ASSEMBLY **Inline assembly** → banned, but legal when programming hardware directly, wrapped and reasoned (the canonical escape-hatch example). @status:impl/done
- @fact:BAN-HIDDEN-CONTROL-FLOW **Proc-macro magic, `Deref` polymorphism, decision-making `Default`, effectful `From`** → hidden control flow is forbidden (R-021); deviations require reason and machinery. *Specified, not built (→ B-038): R-021 is cited across the corpus but authored nowhere — the core ATLAS roster carries only `BLD-` / `DR1-` / `DR2-` / `R2C-` / `R3-` ids and has no R-021 entry — and no forbidden-idiom scan ships in `rust-ai-native-conform`. The ban binds a reader, not a checker.* @status:impl/plan
- @fact:BAN-STRINGLY-TYPED-SURFACES **Stringly-typed protocol surfaces, boolean/positional argument soups** → replaced by typed builders (Class B); deviation requires reason. @status:impl/done

@fact:BAN-WITHOUT-A-HATCH-IS-A-DISCIPLINE-BUG A ban with no escape hatch is a discipline bug; a deviation with no reason is a code bug. @status:impl/done

## 7. Metadata layer (specmap) {#specmap}

@fact:SPECMAP-METADATA-LAYER *(PROP-014, retained as discipline meta-layer.)* `spec://` URIs; in-source inert attributes `#[spec(implements|verifies|documents|deviates|informs)]` (≤3 edges per item, the specmark budget); two-tier revisions (author-asserted semantic revision + content hash) with **asymmetric invalidation** (spec bump → edges suspect; code change → edges stay valid); a derived deterministic committed index; an orphan ratchet; `deviates` requires a reason. @status:impl/done

@fact:METADATA-IS-THE-AUTHORED-RETRIEVAL-INDEX The metadata is the authored retrieval index (R3-012): stable anchors + a uniform one-line what/why per public item, in a fixed grammar the pager consumes. @status:impl/done

## 8. Prose discipline (the asymmetric hazard) {#prose-discipline}

@fact:WRONG-PROSE-IS-WORSE-THAN-NO-PROSE Wrong prose is worse than no prose (R2C-004, H4): models condition on in-repo text with high trust, so a lying comment is adversarial input, and the harm exceeds that of absence. @status:spec/done

@fact:PROSE-NEAR-CODE-IS-CHECKED-OR-TRUST-LABELED Therefore prose near code is **machine-checked** (doctests for behavioral claims, `#[spec(documents)]` edges making drift detectable via spec-rev bumps) or **explicitly trust-labeled** (verified / unverified / aspirational). @status:impl/done

@fact:MISLEADING-LOG-STRINGS-COUNT-TOO Misleading log/print strings count too (the harm is the false claim, not the comment syntax). @status:impl/done

@fact:RUSTDOC-IS-THE-HUMAN-DETAIL-LAYER rustdoc remains the human detail layer; duplication with the spec is a spec defect. @status:impl/done

## 9. Replacement protocol {#replacement-protocol}

@fact:REPLACEMENT-SHIPS-A-DIFFERENTIAL-ORACLE *(R-040, retained.)* Replacing a cell ships a **differential oracle** (Class D) against the old cell, plus the `#[spec(verifies)]` edge. @status:impl/done

@fact:CHARACTERIZATION-GOLDENS-MUST-FAIL-LOUDLY Characterization goldens pin opaque legacy behavior; goldens must fail loudly when stale, never auto-update. @status:impl/done

## 10. Test matrices {#test-matrices}

@fact:DECLARED-TEST-MATRICES-NEVER-EXPONENTIAL *(R-060, retained.)* Declared test matrices, never `2^n`. @status:impl/done

@fact:TEST-KINDS-BY-SURFACE Property tests for behavioral surfaces; the differential oracle covers replacement; per-cell tests run in the fast loop. @status:impl/done

## 11. How a weak reader actually uses this guide {#weak-reader}

@fact:WEAK-SWARM-DOES-NOT-READ-THIS-GUIDE The weak swarm does **not** read this guide. @status:impl/done

@fact:WEAK-READER-RECEIVES-THE-BAND-THREE-EXTRACT It receives, per edit, the Band-3 ops extract of whichever cards' triggers fire — a small, activation-matched set (lazy-push, R3-014; minimal sufficiency, AGENTbench). @status:impl/done

@fact:GUIDE-IS-THE-AUTHORING-ARTIFACT This guide and the cards are the authoring/review artifact for the strong author and the human; the runtime surface for the weak reader is "the right card's routine + checker, when its trigger fires." @status:impl/done

@fact:CROSS-CUTTING-CONCERNS-ARE-SWEPT-BY-RAIDS Cross-cutting concerns the per-edit loop cannot hold are swept by raids (`03-RAID-PLAYBOOK.md`). @status:impl/done

## 12. Tooling roadmap pointer {#tooling-roadmap}
@fact:tcg-line-has-two-briefs The tcg line has two briefs here. @status:impl/done

@fact:BRIEF-SHIPPED-AGENTIC-TCG **Shipped:** `rust/tools/vibe-agentic-tcg-rust.md` — the agentic type oracle (`rust-ai-native-tcg` over the consumer's rust-analyzer; validate/scope/complete/type on in-memory overlays, discipline-enriched by the same conform engine as the gate; the four `tcg_*` MCP tools answer `language: "rust"`). @status:impl/done

@fact:BRIEF-VERY-FAR-FUTURE-TOKEN-MASKING **Very-far-future:** `rust/tools/rust-ai-native-tcg.md` — token-level masking to rust-analyzer-validated, discipline-conformant continuations; waits on an inference substrate. @status:spec/done

@fact:ORACLE-IS-THE-GENERATION-TIME-COMPLEMENT The oracle is the generation-time complement to the post-generation `cargo check` loop (Class E) — consultation today, masking maybe-someday; the floor stays the truth either way. @status:spec/done

## 13. Wiring the gates in a consumer project {#wiring}

@fact:STACK-SHIPS-EVERYTHING-BELOW The stack ships everything below; nothing requires the discipline's dev tree. @status:impl/done

1. @fact:WIRING-INSTALL-THE-TOOLCHAIN **Install the toolchain.** `vibe install` materialises this package into `vibedeps/`. Then either put the umbrella binary on PATH once — `cargo install --path vibedeps/<stack-slot>/crates/rust-ai-native-cli` — or run it in place: `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p rust-ai-native-cli --bin rust-ai-native -- <args>`. Add `vibedeps/**/target/` to `.gitignore`. @status:impl/done
2. @fact:WIRING-BOOTSTRAP **Bootstrap.** `rust-ai-native init` writes `conform.toml` (topology-detected roots; every crate exempt-with-a-reason — the pre-adoption posture), `specmap.toml` (your `namespace` + `[[external_specs]]` discovered from the installed packages, so citations of `spec://org.vibevm.ai-native/core-ai-native/…` resolve), and the `discipline/registry/` files. Idempotent; `--force` to regenerate. Run it after your workspace skeleton exists (topology is detected at init time); re-run with `--force` if the layout changes later. @status:impl/done
3. @fact:WIRING-TAKE-THE-TAGS **Take the tags.** Your workspace deps the shipped proc-macro — and **excludes the slot tree** (the packages are their own Cargo workspaces; without the exclude, cargo binds their crates to YOUR workspace and manifest inheritance breaks — PROP-024 §2.4): @status:impl/done
   ```toml
   # workspace Cargo.toml
   [workspace]
   members = ["crates/*"]
   exclude = ["vibedeps"]

   [workspace.dependencies]
   specmark = { path = "vibedeps/<stack-slot>/crates/vendor/specmark" }
   ```
   @fact:WIRING-PER-CRATE-SPECMARK-AND-SCOPE then per crate `specmark.workspace = true`, and modules carry `specmark::scope!("spec://<your-ns>/<doc>#<anchor>")` (§7). @status:impl/done
4. @fact:WIRING-FIRST-UNIT-FIRST-INDEX **First unit, first index.** Write `spec/PROP-001.md` with an anchored req (`## X {#req-…}` + `` `req r1` ``), tag the implementing module, run `rust-ai-native specmap` to mint `specmap.json`, commit it. @status:impl/done
5. @fact:WIRING-THE-FLOOR **The floor.** `rust-ai-native floor` = fmt → test → clippy → conform → specmap → test-gate (when the baseline exists). One exit code; per-policy origin lines (a `Defaulted` policy announces itself — never trust a green you didn't configure). This replaces a hand-rolled self-check script. @status:impl/done
6. @fact:WIRING-ADOPT-CRATE-BY-CRATE **Adopt crate by crate.** Drain a crate to zero findings (`conform check --scope <crate>`), then flip it into `[rust] gated` and drop its `[[rust.exempt]]` entry — the expand-as-you-conform rhythm; a flip must never widen the baseline. The `every-crate-gated-or-exempt` invariant is enforced by the engine on every check. @status:impl/done
7. @fact:WIRING-PROCEDURES **Procedures.** `vibe skill install` projects `/rust-ai-native-terraform` (brownfield adoption) and `/rust-ai-native-sweep` (the recurring sweep) into your agents; the methods are the core package's playbooks. @status:impl/done
8. @fact:WIRING-GENERATION-TIME-ASSISTANT **The generation-time assistant.** The stack ships an agentic type oracle (`vibe-agentic-tcg-rust.md`): before writing a nontrivial `.rs` edit, check the HYPOTHETICAL content instead of paying a red floor iteration — `vibe bin exec rust-ai-native-tcg -- validate src/cells/<cell>.rs --content-from - --root .` (the edit on stdin; exit 1 = an error-grade diagnostic or a non-baselined finding, with the findings and REQ-citing advice printed), or the `tcg_validate` / `tcg_scope` / `tcg_complete` / `tcg_type` MCP tools with `language: "rust"` when vibevm's server is mounted. **Prerequisite: rust-analyzer.** Installing this stack obliges the machine to carry it (`rustup component add rust-analyzer`); the oracle resolves the CONSUMER's component (toolchain-file-aware) and refuses with that recipe when absent. Honesty note: the oracle is rust-analyzer, not rustc — a clean validate shortens the distance to green, the floor remains the truth (TCG-ORACLE-RUST §5). @status:impl/done

## 14. Sweep idioms (Rust) {#sweep-idioms}

@fact:sweep-idioms-lead The recurring sweep's Tier-1 moves (04-SWEEP-PLAYBOOK), in their Rust shape — each proven across the pilot's campaigns: @status:impl/done

- @fact:SWEEP-TESTS-OUT-SPLIT **Tests-out split** (danger-band files): move an inline `#[cfg(test)] mod tests` to a sibling `foo/tests.rs` declared `#[cfg(test)] #[path = "foo/tests.rs"] mod tests;`. Cell registration is untouched. Gotchas: the conform frontend parses files standalone, so a non-`#[test]` helper in the tests-out file needs its own `#[cfg(test)]` or its unwraps read as domain; `pub(super)` items cannot be re-exported wider (E0364). @status:impl/done
- @fact:SWEEP-RESPONSIBILITY-SPLIT **Responsibility split** (when the production half alone exceeds the budget): split along the file's seam into module-grain cells; **every new module carries the parent's `scope!` URI** so it stays in the retrieval index (no gated orphan). Measure with the rule (physical `lines().count()`), not the eye. @status:impl/done
- @fact:SWEEP-FOUR-DOCTEST-IDIOMS **The four doctest idioms** (pub-doctest drain): a TOML round-trip for serde sections (`toml::from_str::<T>(r#"…"#)` — the wire form is the canonical use); a parse one-liner for newtypes (via their `Deref<str>`/`PartialEq<str>` ergonomics); a variant/`Default` assert for bare enums; a construct-and-Display assert for error enums (the Class-F message already cites its REQ, so the example doubles as a navigability demo). @status:impl/done
- @fact:SWEEP-RESTRUCTURE-BEATS-TESTIFY **Restructure beats testify** (unwrap drain): types carry the invariant — split-first tuples, `let-else`, `next_if`, read-then-advance counters, parser early-returns; `from_validated` beats a fake-fallible signature; a structural `semver::Comparator` beats parsing a formatted string that panics on edge input. `#[spec(deviates)]` is the last resort, and it decays: a deviation whose invariant became encodable is a defect. @status:impl/done
- @fact:SWEEP-FLIP-ONLY-AFTER-DRAIN **Flip-only-after-drain**: a crate enters `[rust] gated` (or `[rust] gated_pub_doctest`) only at zero findings; the collector (`rust-ai-native health`) names the promotion candidates and ranks the drain backlog smallest-gap-first. @status:impl/done
