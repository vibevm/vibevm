# Tool Spec: `vibe-agentic-tcg-rust` — the Agentic Type Oracle for Rust {#root}

<status stage="spec" state="done"/>

@fact:status-line *Status: component brief at FULL seven-section parity (problem · design
stance · component shape · staged ambition · licensing · risk register ·
summary).* @status:impl/done

@fact:COMMISSIONED-AS-THE-RUST-TWIN *Commissioned 2026-07-07 as the Rust twin of the agentic tcg
line — the sibling of `rust-ai-native-tcg.md` (token-level, VERY-FAR-future)
and of the TS stack's `vibe-agentic-tcg-ts.md` — and implemented by
AGENTIC-TCG-RUST-PLAN v0.1.* @status:impl/done

@fact:companion-documents *Mechanism specs:
[`TCG-ORACLE-RUST-v0.1`](../mechanisms/TCG-ORACLE-RUST-v0.1.md),
[`TCG-PROTOCOL-RUST-v0.1`](../mechanisms/TCG-PROTOCOL-RUST-v0.1.md); the
product-side MCP seam is vibevm's PROP-026 (the `language` parameter was
cut for exactly this arrival: a new value, not new tools).* @status:impl/done

## 1. What problem it solves {#problem}

@fact:RUST-AGENTS-LEARN-TYPE-ERRORS-AT-WRITE-GRAIN Rust is this project's primary language, and a Rust-editing agent today
learns about a type error at write-grain latency: write the file, run
`cargo check` (seconds), parse stderr, retry. @status:spec/done

@fact:MOST-OF-THE-MASKS-VALUE-IS-DELIVERABLE-WITHOUT-IT The TS campaign proved
that most of a logit-mask's value is deliverable WITHOUT the mask —
information ("what is in scope, does this type-check"), feedback latency
(milliseconds instead of a floor round-trip), and generation-time
discipline (the gate's own rules answered while writing) — as tools the
agent consults. @status:spec/done

@fact:GUARANTEE-STAYS-WITH-THE-FLOOR The mask's GUARANTEE stays with the floor; the oracle
makes red floor iterations rare. @status:impl/done

@fact:THE-LOAD-BEARING-DIFFERENCE-RA-IS-NOT-RUSTC The Rust twin closes the same gap with one load-bearing difference,
stated here rather than discovered later: **the TS oracle IS the
compiler** (the LanguageService is tsc's engine — agreement by
construction), while **rust-analyzer is NOT rustc**. @status:spec/done

@fact:RA-NATIVE-DIAGNOSTICS-ARE-PARTIAL Its native
diagnostics are a separate implementation with deliberately partial
coverage. @status:spec/done

@fact:RUST-ORACLE-IS-AN-HONEST-APPROXIMATION The Rust oracle is therefore an honest APPROXIMATION: fast,
overlay-true, discipline-enriched — and never the final word. @status:spec/done

@fact:FLOOR-REMAINS-THE-TRUTH The floor
(`rust-ai-native floor` — the seven steps: cargo fmt → cargo test →
clippy → conform → specmap → test-gate → fast-loop; the compile rides
inside the test step) remains the truth. @status:impl/done

## 2. Design stance (consequences of what we know) {#design-stance}

- @fact:STANCE-STAND-ON-THE-CONSUMERS-RUST-ANALYZER **Stand on the consumer's rust-analyzer, over LSP.** The oracle
  process is the CONSUMER's own `rust-analyzer` component
  (rustup-resolved from the project root so `rust-toolchain.toml`
  pinning is honoured; PATH fallback; absence is a recipe-carrying
  error, never a silent skip — the fresh-box failure mode is real: this
  box lacked the component until plan authoring). No bespoke type
  engine, no embedded analyzer: the `ra_ap_*` library embedding is the
  owner's FAR BACKLOG (vibevm ROADMAP.md) — a much-later capability
  upgrade, rejected for v0.1 on API churn, dependency mass, and
  version-binding grounds. @status:impl/done
- @fact:STANCE-OVERLAYS-NEVER-DISK **Overlays, never disk.** LSP text-document overlays (`didOpen` /
  `didChange` with monotonic per-document versions) validate the
  HYPOTHETICAL edit before it lands. The Phase-0 spike proved a seeded
  type error visible through a pull diagnostic without any disk write,
  answered in 1–2 ms warm. @status:impl/done
- @fact:STANCE-CONFIG-IS-LOAD-BEARING **Config is load-bearing.** rust-analyzer's most valuable native
  diagnostics (type-mismatch E0308, unresolved-name E0425) sit behind
  its default-off `diagnostics.experimental.enable`; the oracle enables
  them deliberately via `initializationOptions` AND the
  `workspace/configuration` answers. A null-config oracle is nearly
  blind — the Phase-0 spike's iteration 1 proved it by silence. @status:impl/done
- @fact:STANCE-ENRICHMENT-IS-IN-PROCESS **Enrichment is in-process — one engine, zero extra hops.** The gate's
  fact extractor (`rust-ai-native-conform-frontend`'s `RustFrontend::extract`) is
  a pure function over source text, so the relay runs the REAL conform
  rules (`rust_ai_native_conform::build_rules` → `conform_core::check`) on the
  overlay content directly. Where the TS twin needed a node-side fact
  emitter and a parity test against duplication, the Rust twin calls
  the library. Findings come back flagged against the project's frozen
  ratchet baseline, with Class-F advice citing GUIDE REQs. @status:impl/done
- @fact:STANCE-FIDELITY-POSTURE-IS-SPEC **The fidelity posture is spec, not fine print.** The differential
  corpus curates error classes INSIDE r-a's native competence and pins
  each against `cargo check` through a committed r-a-code ↔ rustc-code
  mapping table (spike rows: E0308↔E0308, E0425↔E0425, E0107↔E0061,
  E0559↔E0609, E0063↔E0063). Where r-a is known-silent (privacy E0603
  at 1.93.1), the corpus carries a DOCUMENTED-GAP case asserting the
  asymmetry — it flips red the day r-a starts covering it. @status:impl/done
- @fact:STANCE-DUAL-TRANSPORT **Dual transport, agent's choice** (PROP-018 §2.8): a persistent
  `serve` relay for MCP hosts, and one-shot CLI forms
  (`vibe bin exec rust-ai-native-tcg -- validate …`) for agents without MCP. @status:impl/done
- @fact:STANCE-CAPABILITY-ROUTING-IS-FREE **Capability routing is free here.** A tool you may ignore does not
  distort (DR1-015 inverted) — and the TS battery's Stage-A null adds
  the honest complement: a tool merely offered may also not help until
  delivery binds. This brief ships mechanics; delivery experiments are
  the backlogged cross-language Stage B. @status:impl/done

## 3. Component shape (how it fits vibevm) {#component-shape}

@fact:three-processes-lead Three processes; the facts hop of the TS topology is gone because
enrichment happens inside the relay: @status:impl/done

```
agent ──MCP (tcg_validate/tcg_scope/tcg_complete/tcg_type,
   │         language:"rust")──▶ rust-ai-native-mcp serve
   │                             (this family's own MCP server —
   │                              PROP-027; language is a compat guard)
   └─or one-shot─▶ vibe bin exec rust-ai-native-tcg -- <op> ──▶ rust-ai-native-tcg
                                              (TCG-PROTOCOL frames up;
                                               enrichment in-process:
                                               RustFrontend facts →
                                               conform rules → findings,
                                               baseline flags, advice)
                                                  │ LSP over stdio
                                                  ▼
                                              rust-analyzer
                                              (consumer's component;
                                               overlays, pull
                                               diagnostics, hover,
                                               completion)
```

- @fact:COMPONENT-THE-BRIDGE **The bridge** (`crates/rust-ai-native-tcg-bridge`, this package): the
  LSP client as a seam — Content-Length framing, capability-negotiated
  handshake (utf-8 positionEncoding, pull diagnostics, serverStatus),
  overlay/version bookkeeping, quiescence with a `degraded` flag,
  five-way typed error taxonomy, replay-tested without rust-analyzer. @status:impl/done
- @fact:COMPONENT-THE-CLI **The CLI** (`crates/rust-ai-native-tcg`, bin **`rust-ai-native-tcg`**, the package's
  4th `[[binary]]`): `serve` (the enriching relay; self-inits with the
  project's conform.toml topology so a host's first frame can be
  `validate`) + one-shot ops + `bench` (the corpus/latency harness). @status:impl/done
- @fact:COMPONENT-THE-PRODUCT-SEAM **The product seam** (vibevm, PROP-027; PROP-026 §2 keeps the
  grammar): the SAME four `tcg_*` tools, served by this family's own standalone
  MCP package `mcp:org.vibevm.ai-native/rust-ai-native-mcp` — no new tools, and
  `language` survives only as a validated compat parameter that refuses a
  mismatch with the recipe naming the right server. The multiplexed
  `vibe mcp serve` + `vibe-tcg` slot-dispatch topology this row described was
  retired whole (PROP-026 ##SUPERSEDED-TOPOLOGY, ##TCG-CRATE-DELETED); the
  enum-value promise — cashed, and it re-reads as «a new language is a new mcp
  package shipping the SAME tool grammar» (##ENUM-BET-REREAD). @status:impl/done
- @fact:COMPONENT-DETERMINISM-AND-AUDITABILITY **Determinism and auditability:** given (project state, overlay set,
  policy, r-a version), answers are deterministic modulo r-a's own
  analysis; enriched findings cite `spec://` REQs; nothing samples. @status:impl/done

## 4. Staged ambition {#staged-ambition}

- @fact:STAGE-A-CONSULTATION-ORACLE **Stage A — the consultation oracle (THIS brief):** validate / scope /
  complete / type over LSP overlays, discipline-enriched, MCP +
  one-shot delivery, mechanics proven by the differential corpus and
  the bench baseline (no agent battery: the delivery question is the
  backlogged cross-language Stage B). @status:impl/done
- @fact:STAGE-B-RICHER-DISCIPLINE-ADVICE **Stage B — richer discipline advice:** newtype-constructor
  suggestions at seams, policy-ranked completions, cell-topology
  answers — fields the protocol already carries. @status:spec/done
- @fact:STAGE-C-DELIVERY-EXPERIMENTS **Stage C — delivery experiments** (cross-language, with the TS twin,
  when the owner commissions Stage B of the delivery line): MCP-mounted
  and write-path-hook arms over a rust task battery on
  `research/rust-demo`. @status:spec/done
- @fact:STAGE-D-RA-AP-EMBEDDING **Stage D — the `ra_ap_*` embedding (FAR BACKLOG, owner-dispositioned
  «сильно-сильно позже»):** in-process semantic access replacing the
  LSP child — richer scope/brand answers, custom traversals — at the
  cost of pinning our r-a version. Re-enters planning only by the
  owner's word; recorded in vibevm ROADMAP.md's Far backlog. @status:spec/done
- @fact:STAGE-E-TOKEN-LEVEL-TCG **Stage E — token-level TCG** (`rust-ai-native-tcg.md`, VERY-FAR-future):
  the same oracle answers a logit-masker inside a decode loop when
  `vibe-llm` exists. Nothing here is thrown away; the consumer of the
  answers changes. @status:spec/done

## 5. Licensing posture {#licensing}

- @fact:LICENSING-RUST-ANALYZER rust-analyzer: MIT/Apache-2.0, and the consumer brings their own
  component (we spawn, never link, never vendor). @status:spec/done
- @fact:LICENSING-LSP LSP: an open specification; the client is in-house code. @status:spec/done
- @fact:LICENSING-CONFORM-ENGINES The conform engines: in-house (this package + vendored
  core-ai-native copies). @status:impl/done
- @fact:LICENSING-PLDI-PACKAGE-IS-NOT-A-DEPENDENCY The PLDI'25 reproduction package: **not a dependency, not opened**
  for the agentic line (the standing clean-room rule,
  `spec/boot/90-user.md`); this campaign's concept sources are our own
  briefs, the LSP spec, and rust-analyzer's public documentation. @status:impl/done
- @fact:LICENSING-NET-NOTHING-VIRAL Net: nothing viral anywhere near the critical path. @status:impl/done

## 6. The honest risk register {#risk-register}

- @fact:RISK-RA-IS-NOT-RUSTC **r-a is not rustc — the defining risk, owned in the spec.** Native
  diagnostics are partial (borrow-check subtleties, trait-solver
  edges, macro-heavy code may pass the oracle and fail the floor), and
  the experimental set is r-a-experimental by its own naming. The
  corpus curates within competence, the mapping table makes the delta
  inspectable, the documented-gap case keeps the known silence visible,
  and every consumer-facing doc says: the floor is the truth. @status:spec/done
- @fact:RISK-NO-GUARANTEE-BY-CONSTRUCTION **No guarantee by construction.** The agent may ignore the oracle —
  and the TS Stage-A null says a merely-offered tool goes unconsulted
  by weak models. Mechanics ship here; delivery binding is Stage C's
  measured question, not this brief's claim. @status:spec/done
- @fact:RISK-COLD-INIT-ON-LARGE-WORKSPACES **Cold init on large workspaces.** 14.7 s cache-cold on a MINIMAL
  crate on this box (sysroot indexing dominates; 2.5 s warm). On big
  consumer trees the first answer may exceed the product's 60 s
  first-request cap — documented, with the eager-init-at-serve-start
  posture and the degraded flag as the mitigations; targets move only
  with a recorded REPORT reason. @status:spec/done
- @fact:RISK-RA-VERSION-CAPABILITY-VARIANCE **r-a version/capability variance.** Weekly releases; capabilities
  differ across consumer toolchains. Everything is capability-gated at
  initialize and degrades per B5 with recipes; the minimum useful set
  is named in TCG-ORACLE-RUST. @status:spec/done
- @fact:RISK-PROC-MACRO-HEAVY-CONSUMERS **Proc-macro/build-script-heavy consumers.** Slower init, partial
  analysis until the proc-macro server warms; v0.1 does not chase it —
  the demo is dependency-free by design and the limit is named. @status:spec/done
- @fact:RISK-WINDOWS-CHILD-LIFECYCLE **Windows child lifecycle.** The house lesson set applies
  (verbatim-free paths into URIs, kill-on-drop, shutdown/exit dance,
  no-zombie assertions — Phase-0-proven). @status:impl/done
- @fact:RISK-SCOPE-CREEP-TOWARD-AN-LSP-RELAY **Scope creep toward an LSP relay.** The surface is the four
  queries + lifecycle, full stop (PROP-026 §6); rename/code-actions/references
  go through the owner. @status:impl/done

## 7. One-line summary {#summary}

@fact:ONE-LINE-SUMMARY `vibe-agentic-tcg-rust` gives a coding agent millisecond-latency,
overlay-true Rust answers — rust-analyzer diagnostics of unwritten
edits, in-scope symbols, type-valid continuations, and discipline
findings from the same conform engine as the gate — through the same
four `tcg_*` tools the TS twin proved (`language: "rust"`), honestly
labelled as the floor's fast approximation rather than its replacement,
so weak agents stop paying the cargo-check retry tax while the mask
(the only thing this is NOT) waits for an inference substrate far away. @status:impl/done
