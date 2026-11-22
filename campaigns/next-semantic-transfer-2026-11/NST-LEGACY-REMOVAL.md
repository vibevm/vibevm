# NST — legacy removal ledger

Genre: campaign removal ledger, not a separate campaign or compatibility backlog.
Owner instructions2026-11-18: mark legacy as encountered; do not improve it now. Removal owner: child `NST-AI-NATIVE-GLOBAL-REPAIR-INSTALL-G-LEGACY`, after `…-SYNC-START` ideal synchronization. All entries below are planned removal, not completed deletion.

Use stable IDs and concrete behavior/owners. Add entries when ordinary authorized work reveals them; do not run a new full-tree annotation sweep. A nearby source comment in an already edited file may use `LEGACY-REMOVE NST-L001: <behavior>; see campaigns/next-semantic-transfer-2026-11/NST-LEGACY-REMOVAL.md`. Keep the contract in its canonical spec, not in the comment. Markers must not imply that modern functionality is disposable.

## NST-L001 — package `[hooks]` declaration and synthetic lifecycle lowering

- **Behavior to remove:** package-role `[hooks]` pre/post-install declarations and their lowering to synthetic slot contributions. No old declaration fallback or legacy-origin policy is to be perfected.
- **Actual owners identified by builder:** `vibe-core` Manifest.hooks/HooksDecl and `manifest/package/hooks.rs`, related validation/remap; `vibe-lifecycle` legacy hook lowering; `vibe-install` lifecycle plan SyntheticHookIdentity injection. Resolve exact current paths at execution rather than relying on stale line numbers.
- **Replacement:** explicit `[[extension]]` script contributions at `slot:pre-install` and `slot:post-install`, using the ordinary lifecycle callback/handler pipeline.
- **Consumers:** maintained inline CLI/install/workspace fixture families, including hook-slot/rerun/failure/rollback/cancellation/incremental tests; migrate meaningful behavior, not just delete tests. Initial inventory found no maintained production package TOML declaration, but final removal must verify this.
- **Contracts:** PROP-020 historical hooks and PROP-054 POINT-GRAMMAR/H-SCRIPT; staged retirement amendments identify the future replacement. Once retirement lands, old syntax refuses actionably before effects.
- **Current disposition:** owner-directed removal planned at global-repair child2; no compatibility work now. Legacy parser/lowering may still exist until that stage executes. No false claim of completed removal.

## NST-L002 — direct quiet install inherits legacy hook stdout/stderr

- **Behavior to remove:** special inherited raw stdout/stderr from old `[hooks]` during direct `install --quiet`, unlike quiet explicit contributions and suppressed lifecycle prerequisites.
- **Evidence:** `crates/vibe-cli/tests/cli_hook_rerun.rs::direct_install_quiet_keeps_the_existing_slot_stream_policy`; old assertion failed during the d063 coherent diagnostic gate after modern full logging. The owner explicitly rejected retaining this behavior.
- **Replacement:** one modern contribution output policy: quiet suppresses raw child output, JSON has bounded structured previews, complete persistent logs remain independently retained/best-effort.
- **Do not add:** SlotOutputPolicy::LegacyHook, `legacy_hook` boolean, raw-relay exception or another runner solely for this compatibility. A superseded uncommitted shim was preserved outside product source and removed by builder; it is not accepted work.
- **Current disposition:** historical assertion superseded by owner decision, not PASS and not a modern-runtime defect to repair. Retire/replace the test with modern policy coverage when child2 removes legacy admission. Old diagnostic gate remains aborted/non-PASS with explicit classification.

## NST-L003 — independent legacy runner, permission policy and wrapper APIs

- **Behavior to remove:** independent run_package_hook/HookRunner/HookPolicy and legacy HookReport/HookOutput carriage, SlotLifecycleMode::LegacyHooks and old apply/materialise wrappers maintained only for that runner.
- **Actual owners identified by builder:** `crates/vibe-workspace/src/hooks.rs`, install hook/preparation/output cells; `vibe-install` apply/model/slot-lifecycle APIs and consumers; orchestrator legacy references.
- **Replacement:** existing modern SlotLifecycleMode::Callback and lifecycle script/process/evidence owners. Migrate real callers instead of leaving no-op facade APIs.
- **Preserve:** neutral interpreter selection `select_script`, Platform, InterpreterProbe/SystemProbe currently shared with modern handlers. Extract/retain that capability where appropriate; deleting a whole file by its name is not the goal.
- **Current disposition:** removal planned at child2; do not improve the legacy permission/stream/runtime branch now.

## Boundaries and future discoveries

Application installer-option pre_install/post_install and other NST-related `vibe install -g` adapters must be assessed in child2 by actual mechanism/replacement, not silently grandfathered or deleted by word matching. Agent/harness hooks, Git hooks, third-party source and historical evidence are not automatically in scope. New concrete obsolete mechanisms get a new stable row with owners, replacement and disposition; they do not create another campaign or authorize immediate deletion.

At removal, record exact delivery commit, removed paths/symbols, migrated consumers and modern positive/refusal tests. Update the authoritative parent/child plan first, then refresh the visible task tree. Keep the living physical acceptance guide consistent with affected user journeys and owner exceptions.
