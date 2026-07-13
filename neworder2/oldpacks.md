# oldpacks.md — the existing-packages index (§4.2)

Distilled from every canonical package manifest (`packages/<group>/<name>/<version>/
vibe.toml`, depth-4). This is the **dedup input**: the reusable-pattern homes that
already exist, so §4.6 prefers *cite an existing package* over *reinvent*.

Address form (confirmed from live citations + `specmap.toml`):
- host unit → `spec://vibevm/<path-under-spec>/<DOC>#<anchor>`
- package unit → `spec://<group>/<name>/<path>#<anchor>` (e.g.
  `spec://org.vibevm.world/atomic-commits/flows/atomic-commits/conventional-commits#…`)
- `[[external_specs]]` in `specmap.toml` maps a package spec root for **code-edge**
  resolution (gate 1). Prose `spec://` links are **not** graph edges — they are
  grep-resolved (gate 2). Today only `core-ai-native` has an `[[external_specs]]`
  entry.

Status: every row `loaded` (manifest-distilled). "Checker?" notes the gate a
package carries where evident; "—" = none/really a doc-only flow.

---

## Group `org.vibevm.world` — the SDD substrate + culture packages (24 rows, redbook×2)

| object | loaded | covers | exported namespace | checker? |
|---|---|---|---|---|
| addressable-specs/v0.1.0 | ✓ | spec:// URIs, stable anchors, spec-tree layout so a human can correct a machine | `spec://org.vibevm.world/addressable-specs/…` | anchor/URI lint (specmap-adjacent) |
| atomic-commits/v0.1.0 | ✓ | **Conventional Commits** + one-commit-one-idea + **splitting-large-changes** | `spec://org.vibevm.world/atomic-commits/…` | commit-shape (advisory) |
| attribution-policy/v0.1.0 | ✓ | authorship-attribution posture; human-authored surface by default | `spec://org.vibevm.world/attribution-policy/…` | — (policy) |
| campaign-plans/v0.1.0 | ✓ | cold-executable campaign plans: baseline, falsifiable predictions, phase gates, ledger | `spec://org.vibevm.world/campaign-plans/…` | — |
| comparative-research/v0.1.0 | ✓ | evergreen competitor studies, two-way gap analysis, numbered roadmap deltas | `spec://org.vibevm.world/comparative-research/…` | — |
| conflict-protocol/v0.1.0 | ✓ | two writers, one file set: **Human > Spec > Tests > Code**, REVIEW markers | `spec://org.vibevm.world/conflict-protocol/…` | REVIEW-marker sweep |
| decision-records/v0.1.0 | ✓ | decisions-not-facts: record *why* + data + a revisit trigger, at the anchor it governs | `spec://org.vibevm.world/decision-records/…` | — |
| discovery-prompt/v0.1.0 | ✓ | the DISCOVERY collaborative-research prompt: structured-uncertainty grammar, adversarial self-check | `spec://org.vibevm.world/discovery-prompt/…` | — |
| health-audit/v0.1.0 | ✓ | periodic health audit: recurring judgment sweep over what the per-commit gate is blind to | `spec://org.vibevm.world/health-audit/…` | the audit itself (recurring) |
| licensing/v0.1.0 | ✓ | licensing posture: proprietary-with-relicense-intent placeholder, permissive-only deps | `spec://org.vibevm.world/licensing/…` | — (legit eula-template) |
| managed-blocks/v0.1.0 | ✓ | how a tool writes into files it does not own: one delimited block, deterministic scan | `spec://org.vibevm.world/managed-blocks/…` | block-scan |
| manual-tests/v0.1.0 | ✓ | human-runnable manual tests: markdown walkthroughs for integration surfaces suites can't prove | `spec://org.vibevm.world/manual-tests/…` | — |
| operating-modes/v0.1.0 | ✓ | codeword-triggered operating modes: safe default, opt-in postures, red lines | `spec://org.vibevm.world/operating-modes/…` | — |
| qualified-naming/v0.1.0 | ✓ | qualified naming for package ecosystems: mandatory groups, identity tuples, short names at CLI only | `spec://org.vibevm.world/qualified-naming/…` | naming lint |
| redbook/v0.1.0 · **v0.2.0** | ✓ | the redbook **collection**: AI-native dev practices distilled from the book into installable flows (ru chapters) | `spec://org.vibevm.world/redbook/…` | — (collection/aggregator) |
| secrets-hygiene/v0.1.0 | ✓ | surface-secrets never printed/persisted, sanctioned process, token-file convention | `spec://org.vibevm.world/secrets-hygiene/…` | secret-scan (advisory) |
| source-mirrors/v0.1.0 | ✓ | single-writer source mirrors: one mainline, hosts as read-replicas, ff-only fan-out | `spec://org.vibevm.world/source-mirrors/…` | mirror --check |
| spec-genres/v0.1.0 | ✓ | contract vs lore vs research vs plans — what goes where, who wins, two-way links | `spec://org.vibevm.world/spec-genres/…` | — |
| sync-from-code/v0.1.0 | ✓ | reconcile specs with code when the code changed first | `spec://org.vibevm.world/sync-from-code/…` | — |
| tool-design-lessons/v0.1.0 | ✓ | self-updating tools & package systems: pointer-flip activation, immutable instances, identity | `spec://org.vibevm.world/tool-design-lessons/…` | — |
| two-process-model/v0.1.0 | ✓ | human & AI as coprocessors with complementary architectures; files as the only shared memory | `spec://org.vibevm.world/two-process-model/…` | — |
| wal/v0.2.0 | ✓ | **WAL discipline**: the checkpoint file, the cold-resume snapshot, session-start/end hooks | `spec://org.vibevm.world/wal/…` | wal-status skill |
| wal-specspaces/v0.1.0 | ✓ | non-central WALs: register specspaces in a host repo, each with its own WAL + cold-resume | `spec://org.vibevm.world/wal-specspaces/…` | target-resolution |

## Group `org.vibevm.ai-native` — the discipline stack (7 rows)

| object | loaded | covers | exported namespace | checker? |
|---|---|---|---|---|
| core-ai-native/v0.7.0 | ✓ | the AI-Native Code Discipline v0.2: principles, pattern cards, scaffold catalog, raid playbook; **hosts the retained mechanisms** (PROP-014 specmap, BROWNFIELD, ENGINE-CONFORM, LEDGER-INTENT) | `spec://org.vibevm.ai-native/core-ai-native/…` (has `[[external_specs]]`) | conform, specmap, self-trace |
| rust-ai-native-lang/v0.7.0 | ✓ | the Rust projection of the Discipline: cells, naming, contract-first, nine scaffolds, errors-as-contract, vibe-tcg | `spec://org.vibevm.ai-native/rust-ai-native-lang/…` | tcg, conform, floor |
| rust-ai-native-mcp/v0.7.0 | ✓ | the Rust discipline over MCP: every command + tcg type oracle as agent tools | `spec://org.vibevm.ai-native/rust-ai-native-mcp/…` | MCP tools |
| rust-ai-native/v0.7.0 | ✓ | family aggregator (PROP-028): pulls lang + mcp + core; content-minimal, no boot snippet | `spec://org.vibevm.ai-native/rust-ai-native/…` | — (aggregator) |
| typescript-ai-native-lang/v0.6.0 | ✓ | the TS projection: cells, branding over structural typing, the nine scaffolds in TS | `spec://org.vibevm.ai-native/typescript-ai-native-lang/…` | tcg, conform |
| typescript-ai-native-mcp/v0.6.0 | ✓ | the TS discipline over MCP | `spec://org.vibevm.ai-native/typescript-ai-native-mcp/…` | MCP tools |
| typescript-ai-native/v0.6.0 | ✓ | TS family aggregator | `spec://org.vibevm.ai-native/typescript-ai-native/…` | — (aggregator) |

## Group `org.vibevm.fractality` — delegation / agent-OS (2 rows)

| object | loaded | covers | exported namespace | checker? |
|---|---|---|---|---|
| delegation-rules/v0.1.0 | ✓ | the delegation policy layer: a decidable routing matrix (delegate when verification < generation), playbooks | `spec://org.vibevm.fractality/delegation-rules/…` | route/gate calculus |
| fractality/v0.1.0 | ✓ | an agent OS: mission-control scheduler daemon + pods + CLI (the delegation runtime) | `spec://org.vibevm.fractality/fractality/…` | its own test suite |

---

**Acceptance (§4.2):** every package row is `loaded` ✓ (33 canonical packages across
3 groups; fractality machine-logs under `packages/**/runs/` skipped per scope). The
patterns each owns are carried into `concepts.md` as the dedup seed.
