# Projection strict-clippy repair — bounded owner-preserving change

2026-11-19; base16cf7329421c4f679845c2364ca5d130c4359a99. Original independent workspace clippy baseline failed with6 projection findings, retained in control/integration-clippy-jobs2.log. No lint suppression.

Five mechanical corrections preserve behavior: native-directory user-scope let-chain, physical-binding collision let-chain, shared-alias activation conflict let-chain, equivalent receipt schema2..=4 pattern, needless borrow removal. Existing guards/reasons/receipt schema remain unchanged; preliminary projection89 unit/20 doctest regression passed before the API tail.

Architect-approved borrowed SkillProjectionTarget bundles only agent/scope/project/injected directories for the existing snapshot_with_dirs writer. Provider, name, desired payload and dry_run remain explicit arguments. Three callers use it; existing ambient compatibility wrapper and receipt/path/publication owners remain. One compiled public-type doctest records the intended shape. No framework, new crate, provider/name tuple, second writer, environment access or schema change.

Essential final gates: full affected projection and MCP tests/doctests, strict clippy through affected owners/workspace, format/diff, conform and independent exact source review. Terminal evidence must be read before declaring the strict gate green; fixing the first crate may expose later workspace warnings. This atom is independent of quiet-policy/preset alignment and does not close native Hermes boundaries or full integration.
