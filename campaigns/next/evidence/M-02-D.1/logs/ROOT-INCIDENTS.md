
## Root observation invocation mistakes, 2026-10-13
- I invoked `python` instead of installed `python3`; shell exit127, observer did not execute. Retried identical observer with python3 successfully; no worker restart or product mutation.
- I guessed `crates/safefs/src/` instead of actual `crates/vibe-safefs/src/`; rg exit2 missing paths. Resolved owner from root Cargo.toml, read actual definitions, then granted exact owners. No write used the guessed path. Promote these actual Root failures to FIXED-ISSUES.md at D1 acceptance.
