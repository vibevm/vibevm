# Existing npm/defer pre-effects owner — bounded public regression

Follow-up selected by `CORP-HERMES-INTERPRETER-QUALIFY-01` when the isolated system-Python ABI discriminator is blocked. This report reuses existing tests, not a new implementation or attended-cancel claim. APP-VERSIONS baseline and parked guarded-default r3 RED are unchanged.

Built `cargo test -p vibe-cli --test cli_app_steps --no-run` with process-local `RUSTC_WRAPPER=` and `CARGO_INCREMENTAL=1`; no configuration or production source edit. The exact emitted `cli_app_steps-a48d3769a353caa6` ran in existing `run-preview-isolated.py` with explicit retained baseline CLI SHA-256 `2acac8641b59a4a811e583df9c029171c2be680d92397fb39ac66400b16be69e`, explicit proxy, cleared environment, synthetic HOME and filesystem/PID/network namespaces. Only a synthetic loopback archive server is used; no provider, external npm registry, real npm or Node remediation.

`corp-npm-defer-public-r1.log`: **4 passed, 0 failed, 6 filtered**, not a whole-suite gate:

- `a_missing_command_can_be_deferred_without_installation_and_retried_fresh`: selected npm runtime metadata does not fetch on explicit defer; zero fixture HTTP hits, no app-work/index/proxy/version file. A fresh invocation with the synthetic command succeeds and performs the one expected fetch.
- `deferred_reinstall_preserves_existing_receipt_payload_and_foreign_command`: explicit defer leaves accepted installation and unrelated command intact.
- `defer_does_not_relabel_noncommand_or_selection_refusals`: terminal need and invalid installer retain their own refusal classes.
- `defer_stays_out_of_dependency_and_plan_only_failures_and_standalone_options`: defer does not turn unrelated planning/dependency/parser cases into successful installation.

Existing `command:vibe-fixture-npm` is a synthetic command-availability oracle, not native npm version/ABI qualification or a complete offline dependency closure. Explicit `--on-missing-command=defer` is not observed terminal cancellation. No attended cancel or native missing/incompatible npm result is claimed. Remaining UI/runtime/platform/distribution obligations require their own finite contracts; this result does not close CORP-AGENT-PACKAGES or NST.

Private evidence root `/data/git/vibevm-keyring-storage-20261120T0100Z/`: `corp-npm-defer-fixture-build-r1.log`, `corp-npm-defer-public-r1.log`; exact source/test/binary hashes are bound by the accompanying candidate manifest. Independent acceptance pending.
