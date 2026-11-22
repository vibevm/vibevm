# Remove legacy install hooks — owner-directed modern lifecycle consolidation

Status: planned input to `NST-AI-NATIVE-GLOBAL-REPAIR-INSTALL-G-LEGACY`, executable only after the initial ideal synchronization child and explicit owner execution priority. Later owner sequencing2026-11-18 supersedes the earlier proposal to perform retirement inside PRIOR-APP-STEPS. No legacy behavior improvement or retirement implementation is authorized by this document during current PRIOR work. Architect owns contracts/review; builder owns implementation/tests/integration in their own checkout. Honor STOP after PRIOR; no publication or server maintenance is authorized.

## Owner mandate

> просто избавься от этого легаси, используй новые системы. Сейчас у приложений устанавливаемых с помощью vibe install -g нет пользователей, поэтому ты можешь сделать всё что угодно, чтобы сделать правильно, а легаси вычистить

## Outcome

One supported explicit slot contribution system for package preparation/finalization. No old package `[hooks]` table, synthetic hook lowering, independent legacy hook runner or legacy-specific quiet stream policy remains. Keep finished install/update/reinstall/rollback/cancellation/logging journeys working through the existing modern owners. Do not build a compatibility shim or a second migration framework for an unused external surface.

## Scope and exclusions

- Remove Manifest.hooks/HooksDecl and retired package hook grammar, validation/remap/runtime carriage, synthetic injection and independent workspace/install legacy hook execution mode/APIs. Update actual consumers and documentation together.
- `workspace/hooks.rs` currently mixes the retired runner/policy with neutral interpreter selection (`select_script`, `Platform`, `InterpreterProbe`, `SystemProbe`) used by modern lifecycle handlers. Preserve or extract that actual reusable interpreter owner; do not delete the whole cell by filename. Remove obsolete wrapper APIs by migrating actual consumers, not by leaving no-op compatibility facades.
- Use existing explicit `[[extension]]` script handlers at `slot:pre-install` / `slot:post-install`, existing SlotLifecycleMode::Callback execution and existing shared lifecycle script/process/evidence owners. Preserve modern slot ordering and materialisation-diff decisions, rollback, idempotent no-op behavior, truthful partial effects and security boundaries.
- Retired `[hooks]` input must refuse before effects with a useful diagnostic naming explicit slot contributions. Do not silently accept and ignore a misspelled/retired declaration. No automatic source rewriting or runtime fallback is needed.
- Migrate owned maintained fixtures/examples/packages to explicit contributions. Replace legacy-only tests with tests of modern user behavior; preserve cancellation, exactly-once on materialisation change, no-op/reinstall, failure/rollback, JSON, quiet and complete persistent-log assertions. The old direct-quiet inherited-output assertion is intentionally retired, not preserved as an active gate.
- Remove the superseded uncommitted legacy stream-compatibility shim (SlotOutputPolicy/legacy flag/raw relay), preserving unrelated changes and historical failure evidence. Builder already reports it preserved separately and removed from product source; independently verify the final diff.
- Application installer-option `pre_install`/`post_install` fields are a different contract and are NOT silently included. Agent/harness configuration hooks, Git hooks, compile extensions and third-party/reference/history text are not this retirement scope. Report actual cross-contract dependencies instead of expanding by word matching.

## Governing contracts

`PROP-020-install-hooks#LEGACY-HOOKS-REMOVED` and `#MODERN-SLOT-EXECUTION` replace the old active hook contract; earlier anchors remain void tombstones for stable references. PROP-054 `#POINT-GRAMMAR` and `#H-SCRIPT` now name explicit declarations, not `[hooks]` sugar. Retain shared current script-handler contracts and annotate meaningful moved behavior against modern owning facts rather than leaving active specmarks pointing only to void legacy facts.

## Finite execution route

1. Bind exact accepted implementation base and this committed brief/contract revision before writing. Inventory actual core/lifecycle/workspace/install/orchestrator/CLI consumers and owned fixture families; use the builder's existing finite inventory, not a new exploratory campaign.
2. Remove legacy admission/lowering/execution and wire existing modern callback behavior. Migrate tests at their existing ownership seams; authored explicit contributions must not need reserved synthetic IDs.
3. Run affected parser/admission and modern slot families, including refusal of retired grammar before writes, install→no-op→changed reinstall, failure rollback/partial-effect, cooperative cancellation and logs/quiet/JSON. Preserve fixed Windows authority and cross-thread terminal ownership regressions. Choose exact test targets before filters; no complete suite per helper edit.
4. Review the coherent exact candidate and run the designated final comprehensive gate, reusing valid unaffected per-step evidence under verification-selection. Native runtime proof must be bound to the actual changed runtime; explain invalidated claims rather than automatically replaying unrelated visual/manual checks.
5. Update requirement/evidence dispositions, living physical acceptance guide where affected, and authoritative multi-user-planning before refreshing the tree. Retain old gate RED/aborted results as history, not current success.

## Acceptance

- No maintained production admission or execution path accepts/lowers/runs legacy package `[hooks]`; old input fails actionably before mutation.
- No new legacy-origin metadata, output relay exception or independent permission/runner remains solely to support the removed mechanism.
- Explicit slot contribution install/update/reinstall works through the ordinary lifecycle owner, with meaningful no-op/rerun/failure/cancellation behavior proven.
- Quiet suppresses raw contribution output consistently; JSON preserves bounded typed previews; full persistent logs remain independent/best-effort and evidence failure does not change actual install success.
- Maintained fixtures/docs/annotations agree with the modern surface; tests are not merely deleted to hide missing behavior. Unrelated application/agent hook contracts are preserved.
- Exact committed diff receives architectural review; terminal focused/final evidence and actual manual owner dispositions are reported honestly. No implicit external push, broad cleanup or new campaign priority.

## Transfer binding

Architect task branch: `spec/user-project-root`. Exact architecture delivery commit is supplied in the delivery message after commit creation. Builder records the actual implementation base and destination branch in their delivery; the source policy/brief transfer is committed, not a dirty product snapshot.
