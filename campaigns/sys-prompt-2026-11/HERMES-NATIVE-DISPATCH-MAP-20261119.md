# Hermes finite native parser/frontend map

2026-11-19; implementation base `4391c12c8f596e43eb01fafe57f5390d3b8341af`; native pin `4ae451d4126327c871fd084e60c045dec0c85911`. Existing SYS frontier, not another Cartesian matrix or acceptance of temporary unsupported paths.

## Native dispatch owners

`hermes_cli/_parser.py` builds top-level and chat argument namespaces. Top `-z/--oneshot PROMPT` is distinct from chat `--oneshot` boolean `oneshot_exit`. Short resume is `-r`. `main._try_fast_chat_launch` uses `_light_chat_parser`, `_prepare_agent_startup`, `_promote_top_level_resume`, `_set_chat_arg_defaults` and `cmd_chat`. Informational/full subcommands use other native dispatch, not the current chat-only supervisor.

`main.cmd_chat(args)` chooses:

| Journey | Native owner | Current Vibe carrier disposition |
|---|---|---|
| Top-level oneshot | `_run_oneshot_from_args` → gateway_chat_startup.launch_gateway_chat | BLOCKED native bootstrap prerequisite; existing RED, no replay or auth bypass |
| stream-json or explicit safe/config bypass | gateway_chat_startup → gateway_chat.launch_from_args | Native gateway query path exists; safe-mode readiness/credential policy is not waived by carrier |
| Classic chat | `_resolve_use_tui` false → `cli.main(**kwargs)` | Existing ordinary no-selector product path; selector path currently uses gateway query frontend and has narrower flag support |
| Native TUI | `_resolve_use_tui` true → `_launch_tui(...)` | Not claimed qualified by selector query proof; must preserve native dispatch or report exact missing seam |
| Resume/continue/explicit worktree/in-dir query | Native session selection + `session_worktree` + gateway query | Accepted bounded4391 slice, native query unattended=false; not classic/TUI parity |
| Informational/subcommands | native main full dispatch | Current product supervisor admits chat only; not declared complete |

`main_tui_launch._resolve_use_tui(args)` gives explicit --cli precedence, then --tui/tui_native, then TTY gate, HERMES_TUI and native display.interface. Vibe must not infer UI selection merely from mode-looking words or quiet flags.

## Exact config-driven worktree gap

Classic `cli.main(...)` calls `_start_worktree_setup(list_tools, list_toolsets, worktree, w)`. That owner selects worktree if explicit flags **or** `cli.CLI_CONFIG.get('worktree', False)`. It sets process-local `_active_worktree`, TERMINAL_CWD and native atexit cleanup; main then invokes `_build_cli_from_args`, joins worktree and enters native query/UI. Passing worktree=false does not suppress configured worktree. Therefore composing prefix before ordinary classic main can bind the wrong project even with no explicit selector. Existing `session_worktree.create/restore/finish` supports external launchers but does not make unchanged classic main accept an already consumed config-default effect.

`hermes_cli.config.load_config_readonly()` is a read-only merged/managed/env-expanded config owner suitable for inspecting the worktree preference without Vibe YAML interpretation. `cli.load_cli_config()` additionally mirrors terminal/browser/auxiliary settings into process environment; importing classic cli triggers native dotenv/display/config setup. A preselection reader must preserve those native semantics and never mutate the read-only config object or CLI_CONFIG.

Current bounded candidate (HERMES-CONFIG-WORKTREE-RESULT-20261119.md): eligible classic finite query enters one held Selection, native read-only policy is frozen before project selection, then session_worktree runs once iff effective true. False creates no tree and also uses the admitted native gateway query frontend, deliberately avoiding a later classic config re-read that could change cwd in the opposite direction. No separate probe/second reader. This does not solve classic flag/TUI or configured interactive worktree parity and cannot be called a global closure. No guessed home/config environment override.

## Classic/frontend preservation seam

Native gateway `validate_options(args)` refuses image, worktree (consumed by the external owner in4391), no_restore_cwd, run_budget, verbose and compact. Ordinary classic main accepts these and owns image collection, display, query/interactive transition, signals, cleanup and optional worktree setup. Sending all selectors to gateway is therefore not full ordinary native dispatch. `--in`/frozen resume cwd policy must remain explicit, not silently clear a requested option merely to evade validation.

The exact absent handoff is an existing classic entry parameter/callback admitting a preselected session/cwd/worktree **before** `_build_cli_from_args`/first turn while suppressing only the already consumed effect and retaining native cleanup. `cli.main` has no such handle/callback. Calling lower `_build_cli_from_args` + `_run_single_query_mode` + run/cleanup manually would copy orchestration; monkeypatching `_start_worktree_setup`, CLI_CONFIG or global class methods is prohibited. Native TUI has a separate own process/runtime attachment and must not inherit an unqualified carrier claim.

Architectural decision required for that missing owner seam; no installed-upstream changes, local fork/copy, fake runtime readiness or unsupported-as-parity waiver. Continue executable bounded portions only under the existing mandate and independently review source/gates.
