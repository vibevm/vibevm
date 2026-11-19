# Deferred agent-adapter consolidation

Date2026-11-19. Owner decision: finish current functionality through existing owners; make architectural cleanup a separate plan stage. This supersedes any suggestion to block current SYS/CORP delivery on introducing a centralized adapter framework. Planning only, not current implementation authority.

## Plan identity and ordering

Stable proposed node `AGENT-ADAPTER-CONSOLIDATION`, a separate bounded refactoring stage after functional agent delivery. Integrate it into the existing NST global-repair REPAIR phase before FINAL-GATES, with actual dependencies on the completed supported-agent functionality. Do not make current SYS-PROMPT or CORP-ENV depend on it, do not add a new campaign, and preserve original task accounting. Builder updates the authoritative source plan before task-tree refresh; architect owns detailed refactor design/review at that stage.

## Intent

Centralize knowledge specific to each agent behind one explicit per-agent adapter/capability owner. Current responsibilities span core manifest harness admission, workspace prompt delivery, CLI executable/version/carrier dispatch, application presets, and skills/MCP profile/config paths. Keep their existing functional contracts but stop requiring inconsistent independent inventories and repeated family-specific dispatch knowledge.

Shared boot/personal composition, proxy lifetime, generic secret resolution, safe publication/receipts and process supervision remain shared services. A per-agent adapter consumes these services; it does not become a god object, credential vault, second boot solver or replacement native agent engine. Respect crate dependency direction and pure planning versus effectful execution; do not move CLI/environment reads into core libraries merely for apparent centralization.

## Bounded acceptance to expand at execution

- Inventory actual agent-specific decision points and callers against the accepted functional baseline.
- Select a cycle-free canonical capability/adapter boundary and migrate one coherent slice at a time without maintaining a parallel implementation.
- Preserve stored harness names, user launcher aliases, native CLI behavior, stable-prefix/fresh-tail semantics, platform/runtime admission, proxy/credential boundaries and skills/MCP ownership.
- Existing tests and source-bound native evidence are reused where behavior is unchanged; run affected consumer regression and final integrated gates. No ritual remeasurement of provider caching.
- Remove obsolete duplicated agent knowledge after consumers migrate; do not leave old and new registries silently diverging.
- Review design quality and dependency direction explicitly, with finite findings and stopping condition. This is maintenance of working behavior, not a fresh native integration campaign or open-ended perfection pass.

## Current work remains unblocked

Continue current SYS native projections, shared physical skill ownership and Hermes MCP integration using already selected owners. Correctness/security/data-loss defects and the existing architectural-quality acceptance floor remain mandatory now. Deferring structural consolidation is not permission to ship known failures or count temporary unsupported capabilities as full first-class support.
