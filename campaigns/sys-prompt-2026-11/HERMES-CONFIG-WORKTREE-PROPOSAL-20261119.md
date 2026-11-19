# Hermes configured-worktree query repair — bounded proposal

2026-11-19; base4391c12c8; native4ae451d4126327c871fd084e60c045dec0c85911. Existing SYS parser/frontend atom, no new campaign.

## Selected subset and precedence

Only ordinary native classic finite query: parsed chat query/query-file/quiet or chat oneshot_exit on noninteractive execution, no top-level unattended oneshot, no native TUI, no informational/plugin subcommand. Probe uses native `_light_chat_parser` and `_set_chat_arg_defaults`, native `_resolve_use_tui` and bypass/stream dispatch owners. Unsupported classic frontend flags (verbose, run_budget, image, compact, explicit no_restore_cwd) must refuse this candidate before config-worktree creation with a precise native dispatch gap, not silently fall into gateway validation after effects. This refusal remains an open gap, never full parity.

Configured worktree follows native `cli._start_worktree_setup`: explicit worktree OR effective config worktree. Pinned `_parser.py` has no `--no-worktree` option: top flag store_true defaultFalse, chat flag store_true defaultSUPPRESS. Native unknown negative spelling must still fail; do not invent a negative flag or make false override config. Explicit native TUI/CLI precedence and safe/config-bypass must be honored. Config-driven effect is not applied to native top-level oneshot or stream/bypass dispatch that does not use classic worktree config.

## Owners and operation

Before Rust select/build/opt-in, run a bounded read-only selected-runtime probe using native merged/managed/env-expanded config owner; only a tiny decision (native frontend and effective worktree boolean) crosses private scratch result. No config copy to Rust, stdout, ready or durable receipts. Prefer `config.load_config_readonly()` if its worktree semantics match the classic effective reader; verify this against `cli.CLI_CONFIG` in an isolated fixture, otherwise use native classic config owner without copying its merge rules. Profile flags must be resolved by native owner before the read; no guessed home.

For the finite eligible query with effective worktree true, enter existing `Selection::start` before project decisions. Supervisor re-resolves native effective worktree through the same owner, records only the selection effect in invocation-local parsed state, calls existing session_worktree.create once, and uses the accepted exact-session/cwd rendezvous/query frontend/finish. No global CLI_CONFIG change, patched native method, second runner or duplicated create. Probe is advisory routing; supervisor re-admission is authoritative against changed config/runtime and refuses drift rather than launching at stale cwd.

Default false/no effective tree preserves previous ordinary launch path. Existing explicit selectors retain their already accepted path; do not rerun those unchanged proofs. Query-only repair does not claim classic UI/TUI/verbose/image/run-budget or unattended-bootstrap support.

## Allowed files / essential gates

Existing native launcher/selected-runtime/selection helpers and finite dispatch documentation; a narrow native probe asset only if no existing helper owner is suitable. No wire/receipt schema, standalone config framework, installed native changes, credentials, global environment or personal-layer solver.

One actual isolated native fixture: profile worktree=true, no explicit --worktree, finite query → exactly one native tree, ready-selected cwd, composed boot/local artifact and actual provider request bound to that cwd; false config control creates no tree. One bounded negative checks native unsupported negative spelling/unsupported frontend flag before worktree effect. Native default/explicit profile selection can reuse prior accepted owners. Focused CLI tests, format/diff/conform, exact candidate source/binary identity and independent review. No fullsize/cache/MCP/skills replay or expanded matrix.

## Accepted refinement — one held selection owner

Architect accepted eliminating the separate advisory probe: every eligible classic finite query enters the existing Selection owner; native effective policy is read once there before project selection. True creates one tree, false creates none. The chosen query child never re-enters classic worktree setup, so neither true→false nor false→true config changes can relocate cwd after prefix admission. This deliberately changes the false-policy finite-query frontend to native gateway query; it is not unchanged classic dispatch. No new helper/config reader/probe. TUI/top-level oneshot/stream/bypass and interactive seeded queries are not newly admitted by mere presence of -q; unsupported routes refuse before effects and remain gaps. Read-only/native classic comparison on synthetic worktree false/true passed in control/hermes-config-reader.log.
