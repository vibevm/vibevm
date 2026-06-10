# Terraform vibevm — discipline package, v0.2-beta

The complete document set required to begin terraforming vibevm: turning an AI-assisted **unfinished** legacy codebase into the reference implementation of the Discipline. The package is itself a product in beta — execute it against vibevm, measure, revise.

© 2026 Oleg Chirukhin. License: TBD (target: UPL-1.0).

**v0.2 changelog.** Brownfield revision: the package no longer assumes a healthy baseline. New BROWNFIELD-PROTOCOL document; Charter amended with axiom A6; spec-authoring guide gains lifecycle statuses (`planned` / `disputed`); PROP-014 gains unit statuses and `conflicts_with` edges; Playbook rewritten to v0.2 (inventory-not-gate, xfail-strict test gate, debt/intent registries, characterization capture, carry-over reconciliation).

## Document map

| File | Layer | Role | Status |
|---|---|---|---|
| `DISCIPLINE-CHARTER-v0.1.md` | product / T0–T1 | Axioms (A1–A6), tier architecture, rule schema + samples, metrics, governance | beta (amend. v0.1.1) |
| `BROWNFIELD-PROTOCOL-v0.1.md` | product / T1 | Terraforming unfinished projects: registries, xfail-strict, disputes, characterization, carry-over guarantee | beta |
| `PROP-014-specmap-bidirectional-traceability.md` | product mechanism (vibevm-hosted) | Bidirectional spec↔code traceability: anchors, revisions, tags, statuses, index, queries | beta |
| `GUIDE-SPEC-AUTHORING-v0.1.md` | product / T1 guide | Writing specs with bidirectional binding; lifecycle statuses; checklists; importer contract | beta |
| `GUIDE-RUST-v0.1.md` | product / T2 guide | Rust bindings: cells, seams, errors, flags, naming, risk table, conform checks | beta |
| `GUIDE-TYPESCRIPT-v0.1.md` | product / T2 guide | TypeScript/JS bindings: subset selection, cells/seams, suppression policy, boundary modules | beta |
| `GUIDE-PYTHON-v0.1.md` | product / T2 guide | Python bindings: pyright-strict gate, boundary parsing, import-time purity, dynamism bans | beta |
| `ENGINE-CONFORM-v0.1.md` | product / T3 | Cross-language conformance engine: escalation tiers, compiler frontends, fact store, SARIF | beta |
| `LEDGER-INTENT-v0.1.md` | product / T3 mechanism | Intent ledger: memoized understanding, fact vs interpretation classes, epochs, release slice | beta |
| `PLAYBOOK-TERRAFORM-VIBEVM-v0.2.md` | carrier-specific | The operative plan Claude Code executes inside the vibevm repo; inventory, phases, gates, stop conditions | beta |

Placement after ratification: product docs → the Discipline's own repository (to be created); `PROP-014` → `vibevm/spec/common/`; the playbook → `vibevm/terraform/PLAYBOOK.md` (working copy inside the carrier).

## Reading order

**Human (first pass):** Charter → BROWNFIELD → PROP-014 §1–2 → this README → Playbook. Guides and engine docs on demand.

**Claude Code (working session in vibevm):** Playbook §0 is the boot text; it references everything else lazily. Do not preload the full package every session — pull sections when a phase needs them (axiom A2 applies to reading, too).

## Known beta gaps (deliberate)

- C++ guide: not written. (The Rust guide fixed the genre; TypeScript/JavaScript and Python guides landed 2026-06-10.)
- The legacy-spec importer and the conflict-scan heuristics exist as contracts (GUIDE-SPEC-AUTHORING §7, BROWNFIELD §5), not as tuned tools — precision is measured, then tuned, on the real corpus.
- Signing of the release ledger slice: designed (LEDGER §7), not implemented; blocks public runtime exposure, not local work.
- Cell granularity default (module vs crate) is provisional in GUIDE-RUST §1 and explicitly a pilot-measured decision.

## Change protocol

Beta documents are revised on pilot evidence only — each carries the house footer: mechanisms not exercised by their named playbook phase are removed rather than carried as aspiration. Once the Discipline repo exists, these documents become specmap'd spec units themselves (anchored, revisioned, hash-audited), and the registries fold into the metamodel per BROWNFIELD OQ-4.
