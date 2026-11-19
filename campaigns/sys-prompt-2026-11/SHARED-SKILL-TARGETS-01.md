# Shared native skill directories — one physical owner, logical aliases

Date2026-11-19. Bounded repair of an actual SYS native projection blocker. Pi and Codex both load the same `.agents/skills/<skill>` directory; adding Pi to the existing ALL inventory exposes a planner/receipt collision. Existing full projection suite82PASS/2FAIL is not green; focused Pi tests do not waive this shared default-world failure. Builder owns execution; architect selects this contract. No new PlanMode/owner approval cycle is needed for the bounded repair.

## Semantic decision

One physical file tree has ONE receipt owner and ONE publication/removal transaction. Multiple logical agents may refer to it only for the SAME package binding, source selection and exact payload. Keep each agent's logical identity/status/provenance; do not drop Pi or Codex, fake a different directory, duplicate the physical write, or let ALL installation fail by default.

Group targets by the existing platform-aware physical path/fold identity. A group contains its actual canonical target path and nonempty unique canonical agent aliases, plus one owned-file inventory. Different package bindings or different bytes colliding at the same physical path still refuse before mutation. Alias membership follows actual native path resolution, not a hard-coded Pi/Codex special case or agent priority.

Activation must respect physical reality. Aliases sharing a directory cannot truthfully have opposing effective states. An explicit single-agent enable/disable/update that conflicts with another alias's persisted intent refuses with an actionable affected-alias explanation and a way to select the whole shared group. Do not silently disable another agent, silently enable a disabled alias, or report independent activation that native discovery cannot implement. A group-wide confirmed change records intent/provenance for all affected aliases and writes/removes once. Missing legacy intent retains its current default law; compute the complete intended group before mutation so impossible partial states fail atomically.

Removing an agent association while another admitted alias remains enabled retains the shared tree and its ownership inventory. Removing/disabling the complete group removes only receipt-owned unchanged files through existing reconciliation. User-edited/foreign files retain current refusal/preservation behavior. Package removal removes its logical group only; another unrelated binding never acquires ownership by matching path alone.

## Canonical owners and order

1. Read owning PROP-073 managed activation/package skill facts and current generated package-skill receipt schema/format registry. Record exact task base, existing receipt versions and current dirty candidates. No hand-edit generated readers or corpus hashes.
2. `vibe-agent-projection/src/pkgskill/projection/binding.rs`: retain logical selected agents, lower to shared physical groups before collision checks. Different bindings still collide; same binding/equal payload groups coalesce. Bind plan and receipt to the same canonical group representation. Do not merely delete the existing collision guard.
3. Extend the owning target/receipt schema with logical alias membership and one physical inventory, through the existing codegen/format discipline. Choose one canonical representation, not optional fields whose combinations permit contradictory ownership. Read an admitted old single-agent target as a singleton group, validate and publish the current version through the owner; no indefinite parallel receipt engine. Register the exact change and migration disposition in the existing format log before accepting it.
4. `pkgskill/receipt/state.rs`: validate global uniqueness of PHYSICAL groups, unique/nonempty canonical alias sets, each alias resolving to that canonical target, no duplicate alias ownership within the same binding, and the existing safe owned-file set/digests/root containment. Aliases inside one group do not count as duplicate physical owners. Malformed old/unknown receipts refuse, never guess ownership.
5. `pkgskill/receipt/activation.rs`, effective binding/subset/status/remove and activation projection: resolve logical requests to their group, preflight all affected alias intents, preserve unselected unrelated groups, apply conflict semantics above and render truthful shared ownership/effective state. Stable logical artifact identities remain visible; physical transaction identities are group-owned. Explain grouping in existing CLI/JSON/tree results instead of hiding it.
6. Existing reconcile/stage/transaction/recover/publication owners write/compare/remove each physical tree once. Pending transition/recovery carries alias membership and original file witnesses; do not add a second journal or weaken no-follow, stale-source, concurrency, portability or foreign-content gates. Receipt publication and filesystem mutation retain the same durable transaction boundary.
7. Update the precise PROP facts for shared aliases/activation, generated schemas/readers and affected consumers, then run finite gates. Integrate only the projection repair and its actual native path registration; do not sweep unrelated skills or secrets features.

## Finite acceptance

- Default ALL with Pi/Codex actual shared path and same binding/payload plans and applies once, valid receipt/status includes BOTH agents, repeat bytes/mtime stable.
- Existing distinct-path agents retain behavior; explicitly selecting just Pi or Codex remains valid when no competing ownership/intent exists.
- Different bindings/payloads at the shared path refuse before writes; alias ambiguity and platform-fold collisions retain protection.
- Contradictory per-agent activation refuses without mutation; explicit shared-group change updates all affected logical intents with clear provenance.
- Removing one logical association retains the tree for remaining enabled aliases; final group removal safely removes only unchanged owned files. User edits are preserved/refused under current rules.
- Current receipt roundtrip, old singleton admission/current publication, malformed/unknown refusal, interrupted publication/recovery and concurrent/stale source tests through existing owners.
- Run the previously failing full projection tests and affected activation/MCP/CLI consumers; do not let a later shell command mask an earlier test failure. Report exact exit statuses and baseline findings.

## Stopping condition

Finish when actual shared native paths work through default package projection and receipts/activation stay truthful. No generic filesystem sharing framework, agent-specific fake directory, silent dedup, dual writer, hidden alias loss or broad receipt rewrite. This repairs a concrete first-class native integration requirement, not optional future scope.
