# Current M-14 B/C selected integration checks

Subject tree: `6de6bfdbfc122df449d78db9b29b7e6eef51bf22` on local `next`.
These are targeted checks of accepted environment identity and bound Deploy
code after later campaign commits. Each command used the campaign's two-slot
Cargo wrapper with `--locked --offline`; no workspace or full gate panel ran.

| Contract slice | Exact `cargo test` target/filter | Result | Raw log SHA-256 |
|---|---|---|---|
| B.1 exact lock identity | `-p vibe-core --lib environment::exact::tests::` | exit 0, 10/10 | `4ba8eb510291b950e335facdc416f89e896f847d3cd75a477bd66b293982440d` |
| B.2 legacy strength | `-p vibe-core --lib environment::legacy::tests::` | exit 0, 4/4 | `fcfebf6dfb88d716e999bd785de1dbea633e4d3cf05a10dbc72cce7ad60d2099` |
| C.1 receipt/wire refusal | `-p vibe-wire --lib behaviour::deploy_records::tests::` | exit 0, 14/14 | `a37aa78c01e0124f8ad75e29f66a57d4d2150083de72b03acb5a02a9eb4dbe2a` |
| C.2 physical binding | `-p vibe-lifecycle --lib mechanism::deploy::binding_tests` | exit 0, 8/8 | `ea57e71bcb05d2c5077081e16980c3f2377f494770c4c24b561275ae83aee2a1` |
| C.3 bound builtin effect | `-p vibe-lifecycle --lib mechanism::deploy::bound_effect_tests` | exit 0, 7/7 | `f73ca0c4d0944db8c137fef7db1fbeff77f37b73bfa944438ccc99d65e486e0a` |

All 43 selected cases ran; zero selected tests were never counted as a pass.
The five complete logs are beside this report under `current-selected/`.
The C.3 family now selects seven tests, versus three in its original bounded
review; this receipt reports the current run, not a retrospective rewrite.

This is matching-tree Linux evidence for selected B/C contract consumers. It
does not classify all M-14 source changes, prove macOS/Windows native locking,
clear the known global wire-diff limitation, resolve the 33 missing B/C
release-impact subjects or authorize public 1.1.0 publication. `NEXT-GATE-M-14`
remains open; B-226 (capture import/provenance choice) remains unresolved.
