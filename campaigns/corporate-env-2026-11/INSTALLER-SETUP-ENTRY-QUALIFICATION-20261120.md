# Corporate installer to atomic setup — first missing adapter

Architect-selected test/report atom after Pi inventory `7a955440ecbb69ffa35e5804803995d0b66dc7b8`. No production change or reimplementation of the accepted atomic planner. No real profile, kit, credential, provider, supplier install or host activation.

## Current source ownership

`crates/vibe-bootstrap/src/kit.rs::Args` (lines19–47) has kit/profile/mode/bundle/destination/settings/PATH/unattended/dry-run/log arguments, but no `configure-ai`. Its native orchestration (lines348–390) invokes verified Vibe `self apply --dry-run`, then `self apply`, reports Installation complete and returns. Both children are explicitly unattended and `Reporter::child` gives them null stdin. No initial optional setup callback or rerun setup call is present in this owner. An existing successful bootstrap is therefore not evidence of optional AI onboarding.

The first missing owner is this native installer entry/post-success adapter, not the connection store or atomic setup planner. `commands/ai.rs` dispatches the separate `AiCommand::Setup` to `commands/ai/setup.rs`; the accepted route configures existing Pi/Qwen connection metadata, explicit preview/apply and default-No confirmation. It does not install agents, collect new credentials, run providers or implement all three initial/rerun/recovery entry paths. Its accepted cancellation/bulk-publication tests remain valid and are not reopened.

## Actual isolated public probes

Built current unchanged `vibe-bootstrap --bin vibe-installer` with process-local enabled incremental and no toolchain/config changes. Source base `7a955440e`; installer SHA-256 `d667ab7fed7a066b27ce725e8c25bcd55d418d1d05d805d1c618b89da6577b2d`. Standalone Vibe uses retained accepted CLI SHA-256 `2acac8641b59a4a811e583df9c029171c2be680d92397fb39ac66400b16be69e`.

`qualify-installer-setup-entry.py` executed five commands, each in a fresh filesystem/PID/network namespace, cleared environment, synthetic HOME/settings and empty synthetic kit. No private paths/configuration were mounted:

| Probe | Actual outcome | Meaning |
|---|---|---|
| Current installer `--help` | exit0, no `--configure-ai` | Actual parser surface only |
| Installer `--configure-ai --unattended` | exit2, unexpected argument | Rerun opt-in is missing before kit/publication; not a setup runtime failure |
| Initial path with empty kit, unattended/no-path | nonzero reading chosen site profile | Existing input preflight, no bootstrap publication; **not** successful initial installation/onboarding |
| Standalone `ai setup --help` | exit0, explicit `--agent` and `--apply-preview` | Separate public metadata operation exists |
| Standalone `ai setup --json --unattended`, no agents | nonzero, explicit agent selection required | Input-required refusal, no invented consent or provider operation |

All worlds have absent application index/public commands/app payload and absent secret/AI indexes. Exact incidental settings-directory entries are recorded per world; no blanket zero-filesystem-write claim is made. Full successful initial/rerun installer → setup behavior remains **UNRUN/UNIMPLEMENTED at adapter**, not PASS from parser checks. Existing atomic-setup metadata acceptance remains separate.

Private evidence root `/data/git/vibevm-keyring-storage-20261120T0100Z/`: `corp-setup-adapter-installer-build-r1.log`, `corp-setup-entry-public-r1/steps.json`, `summary.json`, outer log and per-world synthetic directories. Exact source/report/runner/artifact hashes are in the candidate manifest. Independent acceptance pending.

## Required next contract, not inferred implementation

Bind an optional post-success adapter and explicit rerun/recovery behavior through the one existing setup owner. Specify how attended input is lent (current bootstrap children are unattended/null-stdin), what structured skipped/deferred/input-required result is returned, and how optional failure preserves completed Vibe installation. New-agent install/Node decision, credential entry and unsupported native API filtering are separate remaining semantics; do not append `ai setup` blindly and label the full corporate wizard done. No product changes are authorized by this report itself. NST remains open; no publication or credentialed trial is implied.
