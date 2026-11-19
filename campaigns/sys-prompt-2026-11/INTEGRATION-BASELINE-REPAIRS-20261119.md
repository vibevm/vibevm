# Accepted native integration baseline — findings and bounded repairs

2026-11-19; product base14b5ef40010073d8ecb2d67ce333b98319422641, preserved unrelated dirty root instructions/materialized dependency inputs. This is not a clean-tree/final SYS acceptance gate.

## First baseline

`control/parity-integration-baseline-self-check.log`: owner-reported OOM termination; fmt step1 PASS, workspace-tests step2 INTERRUPTED with no terminal result. No PASS or product failure inferred. On continuation no Cargo/rustc remained. No cleanup, global configuration or toolchain change.

## Two-job retry

`control/integration-baseline-jobs2-after-oom.log`: existing tools/self-check.sh, process-local CARGO_BUILD_JOBS=2 and existing target/incremental; terminal101 at workspace test step2 after243s, not OOM. cli_app_agents::agents_are_listed_configured_and_presets_shown expected the obsolete six presets, while actual accepted product returned eight including Hermes/Pi. That integration target:10 PASS/1 FAIL; preceding CLI unit target1316 PASS/1 ignored. Steps3–8 UNRUN, not green.

Bounded repair updates that test's exact ordered expected preset list to include Hermes/Pi, matching the existing accepted vibe-app preset contract. No dedup, ignored test, loosened assertion, product semantics or wire format change. Targeted result retained in `control/integration-presets-regression.log`; further integration run remains separately reported. Native upstream blockers and final SYS acceptance are unaffected.
