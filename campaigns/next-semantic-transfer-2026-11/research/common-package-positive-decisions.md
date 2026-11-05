# Common-package positive route: root decisions, 2026-11-05

Status: accepted design choices for implementation; no positive runtime completion claimed.

## Evidence and cause

Root checked all 12 immutable prototype blobs in the optional-contract JSON against pin `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`. Seventeen of eighteen main snapshot files still match; PROP-069 changed during F15 acceptance and remains the current authority. The memo is historical read-only evidence, not a current source manifest. Current Requires, InlinePackageDepWire and resolution_requires confirm the missing optional declaration/activation path. Prototype configuration.rs explicitly rejects TargetFeature and nonregistry configured declarations. The concrete failure of the historical completion claim is conflating an algebra with an executable consumer. Reuse the algebra; finish the actual integration and distinguish observed support from planned support.

## Decisions

1. Extend the existing inline requires declaration with strict boolean `optional`; omitted means mandatory, explicit false remains distinguishable on round-trip. Preserve all existing source/visibility/link/configuration owners. No second dependency grammar or implicit feature aliases.
2. A dependency effect or strong forward activates an explicitly optional declaration. Mandatory declarations remain mandatory even if a feature mentioning them is disabled. Weak forwarding contributes only when that same owner instance has an active declaration; ambient installation by a different root is insufficient.
3. Actual incoming requires declarations supply consumer identity and original intent. A workspace package's published configuration interface is not an answer for self-configuration. Operations with no real activation context must report that missing boundary, never invent consent or silently clear optionality. Self/root interactive support remains a visible obligation, not a declared completion.
4. Validate every local declaration/reference, including dormant definitions. Do not fetch an inactive optional foreign payload merely to install the active graph. Validate forwarded target features against the selected candidate when the edge is consumed.
5. Reuse the existing solver, workspace/lock owner, transaction machinery and common choice semantics. Candidate rejection must leave no activation/default residue. Durable original and resolved state must survive real cold replay and maintenance before writer activation is considered delivered.
6. First positive acceptance requires ordinary authored input, actual optional dependency graph/materialization, durable meaning and a cold second invocation. Parser-only tests or serialized expansion fields do not meet it. Forwarding, supported source modes and effective loading remain separately evidenced obligations; preserve full source bytes.

## Execution

The existing package-adapter worker is preparing the precise integration design and file ownership without source edits or Cargo. Root retains architecture review and acceptance. F15/A0 remains accepted; A1–A5 remain incomplete. No historical task count changes from these decisions.

## Implementation review, 2026-11-05

The companion common-package-positive-design.md is the reviewed proposal, with these root dispositions: its first coherent fixed workspace consumer/local registry/copy/resolvo runtime is authorized as an increment, not the campaign endpoint. Strict legacy7 plus rich8 reading, lossless legacy7 writes and explicit rich8 writes are approved; no read-time migration or discard-rich-lock recovery. Rich writer acceptance includes Fresh, point replay and actual boot/read closure. The proposed highest-version automatic-consent basis is **not approved**: a separate pinned-source review is underway. Explicit no-auto input can proceed; unsupported default reliance must refuse truthfully until resolved. Avoid unused future consumer/forwarding API shapes with no actual owner. Transitive forwarding and broader source/surface support remain required follow-on work.
