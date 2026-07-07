# Tool Spec: `vibe-agentic-tcg-ts` — the Agentic Type Oracle for TypeScript
*Status: component brief at FULL seven-section parity (problem · design
stance · component shape · staged ambition · licensing · risk register ·
summary). Commissioned 2026-07-07 as the AGENTIC delivery of the tcg line —
the sibling of `vibe-tcg-ts.md` (token-level, VERY-FAR-future) — and
implemented by AGENTIC-TCG-TS-PLAN v0.1. Mechanism specs:
[`TCG-ORACLE-v0.1`](../mechanisms/TCG-ORACLE-v0.1.md),
[`TCG-PROTOCOL-v0.1`](../mechanisms/TCG-PROTOCOL-v0.1.md); the product-side
MCP seam is vibevm's PROP-026.*

## 1. What problem it solves

True type-constrained decoding masks logits inside the sampler. A hosted
coding agent (Claude Code, opencode, Codex over their APIs) never exposes
logits — the harness receives sampled text. So the by-construction
guarantee is unreachable in agentic mode, and waiting for it (a local
inference substrate; `vibe-llm` is an M0 stub) parks ALL of the tcg
line's value indefinitely.

Decompose what the mask actually buys, and most of it is not the mask:

1. **Guarantee** — a type-invalid continuation cannot be emitted. Decode-
   loop-only. NOT deliverable agentically; stays with the token-level
   sibling.
2. **Information** — "what is in scope here, with what type; does this
   fragment type-check; what continuations are type-valid at this
   position." Pure data. Deliverable TODAY as tool responses.
3. **Feedback latency** — a mask rejects instantly; an agent today learns
   about a type error via write-file → full floor → parse stderr → retry.
   An incremental in-memory validate answers in ~20 ms (Phase-0 spike
   fact on ts-demo) without touching disk.
4. **Discipline at generation time** — the §8 unsafe-set bans, branding
   at seams (§4), boundary validation (§2), enforced while writing rather
   than post-hoc. Deliverable as enriched answers driven by the SAME
   conform engine the gate runs.

`vibe-agentic-tcg-ts` ships 2–4: a long-lived language-service oracle the
agent CONSULTS, instead of a mask the sampler OBEYS. The guarantee (1)
stays where it already lives — the floor. The oracle's job is to make
red floor iterations rare; the floor's job is unchanged: to be the truth.

## 2. Design stance (consequences of what we know)

- **Stand on the real compiler, not a model of it.** The oracle is an
  incremental `LanguageService` over the CONSUMER's own `typescript`
  install (the same one `tsc --noEmit` uses, resolved the same way
  ts-extract does), with tsconfig read exactly as tsc reads it
  (`getParsedCommandLineOfConfigFile`). No bespoke type engine, no
  subset: whatever the project's compiler accepts/rejects is the oracle's
  answer. This is the structural opposite of the PLDI'25 artifact (a
  hand-built incremental type system for a TS subset, integrated into a
  decoder) — which is also why the clean-room rule is satisfied
  trivially: their code is not needed and not read.
- **Overlays, never disk.** Validation of a HYPOTHETICAL file state —
  the edit the agent is about to make — happens against in-memory
  overlays with versioned snapshots. The agent checks before it writes;
  the write happens once, already-typed.
- **One extraction, two consumers.** The oracle's `validate` returns the
  compiler's diagnostics AND the per-file conform facts/markers (the same
  §8/§9 fact classes ts-extract emits), so the Rust layer can run the
  REAL conform rules over the fragment and merge discipline findings into
  the same response. The agent sees "does it compile" and "does it
  conform" in one roundtrip.
- **Policy interpretation lives in Rust, once.** The node side emits
  facts only. `conform.toml` reading, rule assembly, baseline awareness,
  advice strings — all in the `tcg-typescript` middle layer, through the
  same `conform_core::check` the gate calls. Two engines cannot drift
  when there is one engine.
- **Dual transport, agent's choice** (the PROP-018 §2.8 idiom): a
  persistent `serve` loop for MCP hosts, and one-shot CLI forms
  (`vibe bin exec tcg-typescript -- validate …`) for agents without MCP
  or for scripts. Same protocol shape on both.
- **Capability routing is free here.** DR1-015's over-constraint risk
  (hard constraints distort strong models) does not transfer: a tool you
  may ignore does not distort. Strong agents will consult the oracle
  less; weak agents more; the battery measures the weak population
  (gpt-oss-20b) because that is where the lift is claimed.

## 3. Component shape (how it fits vibevm)

Three processes, two thin NDJSON hops, each layer doing the one thing
the layer below cannot:

```
agent ──MCP (tcg_validate/tcg_scope/tcg_complete/tcg_type)──▶ vibe mcp serve
   │                                                    (vibe-tcg registry:
   │                                                     lazy spawn, slot
   │                                                     dispatch, consent)
   └─or one-shot─▶ vibe bin exec tcg-typescript -- <op> ──▶ tcg-typescript
                                                        (discipline
                                                         enrichment: conform
                                                         rules, baseline
                                                         flags, advice,
                                                         cell/seam context)
                                                            │ NDJSON duplex
                                                            ▼
                                                        node oracle.ts
                                                        (LanguageService +
                                                         overlays, consumer
                                                         typescript)
```

- **The oracle** (`tools/ts-oracle/oracle.ts`, this package): self-
  contained erasable-only TypeScript, embedded into the Rust binary via
  `include_str!` and materialised content-addressed — the ts-extract
  delivery, reused. Ops: `init`, `validate`, `scope`, `complete`,
  `type`, `update`, `shutdown` (TCG-PROTOCOL-v0.1).
- **The bridge** (`crates/tcg-oracle-bridge`): persistent child client —
  correlation ids, timeouts, kill-on-drop, five-way typed error taxonomy,
  replay-tested without node.
- **The CLI** (`crates/tcg-cli-typescript`, bin **`tcg-typescript`**,
  the package's 4th `[[binary]]`): `serve` (the enriching relay an MCP
  host drives) + one-shot ops + `bench` (the battery's latency/agreement
  harness).
- **The product seam** (vibevm, PROP-026): the `vibe-tcg` crate — tool
  schemas, registry, slot dispatch via the PROP-025 model — mounted by
  vibe-mcp as four `tcg_*` tools with a `language` parameter
  (`"typescript"` today; the Rust twin adds a value, not new tools).
  Deliberately liftable into a standalone MCP server (zero vibe-mcp
  imports) — the owner's portability amendment.
- **Determinism and auditability:** given (project state, overlay set,
  policy), every answer is deterministic; enriched findings cite
  `spec://` REQs Class-F style; nothing here samples anything.

## 4. Staged ambition

- **Stage A — the consultation oracle (THIS brief, shipped by the plan):**
  validate/scope/complete/type over overlays, discipline-enriched,
  MCP + one-shot delivery, measured by the two-arm battery.
- **Stage B — richer discipline advice:** brand-constructor suggestions,
  seam-aware "what should cross here", completions ranked/filtered by
  policy (the `unsafe` flag hardening into ordering). Data the Stage-A
  protocol already carries fields for.
- **Stage C — the Rust twin agentically** (`tcg_rust` over rust-analyzer,
  a separate commissioning): the language parameter and PROP-026 are cut
  to admit it without new surface.
- **Stage D — token-level TCG (the sibling brief, VERY-FAR-future,
  owner-dispositioned 2026-07-07):** when `vibe-llm` exists, the SAME
  oracle answers a logit-masker's completability queries inside the
  decode loop. Nothing in Stages A–C is thrown away; the consumer of the
  answers changes. The clean-room rule re-binds there.

## 5. Licensing posture

- TypeScript Compiler API / LanguageService: Apache-2.0 — clean, and the
  consumer brings their own install (we link nothing).
- node, NDJSON, our own engines: in-house or std.
- The PLDI'25 reproduction package: **not a dependency, not a reference,
  not opened** for the agentic line (inspiration-only rule,
  `spec/boot/90-user.md`). No third-party grammar/CFG tooling is needed
  at all in agentic mode.
- Net: nothing viral anywhere near the critical path.

## 6. The honest risk register

- **No guarantee by construction.** The agent may ignore the oracle.
  Mitigation is structural: the floor stays the gate; the oracle only
  shortens the distance to green. The battery measures whether it DOES
  (control vs with-tools arms; if the delta is noise, that is a finding
  against Stage B investment, recorded, not hidden).
- **Latency under scale.** The Phase-0 spike says 22 ms warm validate on
  a demo-sized tree; large consumer trees are unmeasured. The ladder:
  per-file semantic diagnostics only → background pre-warm on init →
  posted targets move WITH a recorded reason. Never a silent miss.
- **LS-vs-tsc divergence.** Same engine underneath but options are
  assembled differently; the differential corpus (validate-vs-tsc on
  seeded errors) is the standing detector, and config is read via the
  same API tsc uses from the start.
- **Windows child lifecycle.** Junction/verbatim/PATH quirks are known
  house lessons; kill-on-drop + shutdown op + no-zombie assertions are
  Phase-0-proven and re-asserted in the hermetic test.
- **Strong-model indifference.** Plausible (DR1-015 inverted); the
  battery deliberately measures the weak population where the claim
  lives. A strong-agent arm is a later, optional measurement.
- **Scope creep toward an LSP relay.** The surface is the four queries +
  lifecycle, full stop; rename/code-actions/references are named
  non-goals (PROP-026 carries the line).

## 7. One-line summary

`vibe-agentic-tcg-ts` gives a coding agent the type oracle's INFORMATION
at millisecond latency — real-compiler validation of unwritten edits,
in-scope symbols, type-valid continuations, and discipline findings from
the same engine as the gate — delivered as consultation tools over MCP
and one-shot CLI, so weak agents stop paying the red-floor retry tax
today, while the token-level mask (the only thing this is NOT) waits for
an inference substrate on its own very-far-future track.
