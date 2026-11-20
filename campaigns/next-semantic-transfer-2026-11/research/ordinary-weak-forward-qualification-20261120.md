# Ordinary weak forwarding — declaring-edge qualification

Contract: NST-ORDINARY-WEAK-FORWARD-QUALIFICATION-01 (`c5d4369b6fd58d8e767d84da57af753edd7646c2`), integrated at `bdb3a079d`. Product baseline: accepted guarded update `15ca6fefab008bffb8154ba640326f95e6b8cd37`. Test/report only; candidate pending independent acceptance. No parent, NST, REMAINDER, IDE or USER-PROJECT state promotion.

## Four independent public worlds

The actual ordinary `install --offline` fixture uses an exact local flow parent and target, an explicitly selected parent feature, and a target feature yielding public `forwarded=true` selected data. Each weak case gets a fresh project and registry:

| World | Declaring parent's edge | Independent target root | Observed public result |
| --- | --- | --- | --- |
| inactive | Optional, not separately activated | No | Unsupported, exit 1 |
| independent-root | Optional, not separately activated | Yes | Unsupported, exit 1 |
| activated-parent | Optional, separately activated by a dependency effect in the same selected parent feature | No | Unsupported, exit 1 |
| mandatory-parent | Mandatory | No | Unsupported, exit 1 |

Parsed fixture assertions distinguish the authored dependency optionality, independent root and separate dependency effect. In every world the exact actual diagnostic contains:

`selected cross-package forwarding requires candidate-conditioned incoming consumers; only exact compulsory strong edges are connected`

The earliest reached refusal is the shared package configuration evaluator's `ConfigurationEffect::Forward` branch in `crates/vibe-core/src/manifest/configuration/package.rs`: non-Strong requests are rejected before incoming candidate context can be assembled. The later source-solve diagnostic `weak forwarding is not connected` is not the observed boundary. A selected dependency effect earlier in the same feature does not bypass this rejection.

All four failed operations preserve the project semantic snapshot (including authored input and boot), complete registry snapshot, absence of lock, parent/base/target slots and hook log. Complete `.vibe` observations are printed separately: only the coordination directory and `lifecycle.lock` appeared. Project semantic snapshots intentionally exclude that coordination namespace; this is not a claim of byte-for-byte whole-project immutability or complete cache absence.

## Accepted strong route as positive control

A fifth independent world retains the unmodified exact one-hop strong declaration. Public install succeeds; the retained parent has the active target dependency, the target has one forwarded input and public `forwarded=true`, the target slot and boot marker exist, and its hook ran. A fresh CLI install invocation against the retained lock preserves package/boot/lock state, hook counts and exact authored request bytes. This exercises the existing supported strong route, not weak execution.

## Meaning and smallest next implementation boundary

The result qualifies a conservative unsupported boundary in all four weak worlds. It does **not** demonstrate correct weak inactive-edge suppression, activation by the parent's own edge, or distinction from an independently selected workspace target: all four are rejected before those outcomes can be observed.

The smallest proposed implementation slice is exact local one-hop weak forwarding through the existing candidate-conditioned incoming context. Derive declaring-parent edge activity independently of weak forwards from mandatory declarations and selected dependency effects; an independently rooted target must not activate that parent edge. Only active declaring edges may contribute weak target input. Preserve original consumer input, separate provenance, current pin/closure validation and strong behavior; repeat these four worlds and cold replay with supported outcomes. This is a proposed boundary for the architect, not permission to relax one evaluator check in isolation. No weak implementation, recursive/range route, renewal, fetching redesign or format change is included here.

## Evidence

Selector: `default_review::weak_forwarding_declaring_parent_edge_four_worlds_and_strong_control`.

- `weak-forward-qualification-public-r1.log`: retained oracle RED. First inactive world reached the public refusal, but the assertion expected the later internal phrase. Other worlds and strong control did not run; no coverage is credited to r1.
- `weak-forward-qualification-public-r2.log`: corrected exact public oracle; all four diagnostic/state checks plus strong install/cold repeat PASS (one test, 70 filtered out).
- `weak-forward-qualification-build-r2.log`: build-only PASS.
- `weak-forward-qualification-clippy-r1.log`: strict fixture clippy PASS (`-D warnings`).
- Format/diff checks are sealed separately with the candidate receipt.

Runtime uses the existing finite `python3 -I` bwrap supervisor: filesystem/process/network isolation, cleared environment, synthetic HOME, explicit read-only test/CLI artifacts and system runtime only; no host home, credentials, auth stores, sockets or network. No model inference or provider execution.

Actual CLI SHA-256: `d333e4f94696c110742937ba2f24305736e092cb6cdfd9f6036db78873976108`.
Actual fixture SHA-256: `a42ea8af478200384d13f26dc13a121ffff366ddb2b1d39acfe3fff37c83870d`.
Exact two-file source/evidence hashes are recorded in `weak-forward-qualification-candidate-r1.json` outside the repository. Independent review and local integration remain pending.
