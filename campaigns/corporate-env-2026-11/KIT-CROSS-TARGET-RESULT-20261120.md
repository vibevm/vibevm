# Linux-host kit production for actual historical foreign targets

2026-11-20 candidate, baseline `54201ecd4b812ee637b8ab1b1fca56c108f59f40`. Existing `CORP-KIT-BUILDER` AC3: distinguish producer host from consumer target. No private corporate directory, foreign binary execution, publication, build-host provisioning or native-OS claim.

## Changes and actual inputs

Recipe schema1 fields/meaning are unchanged; admitted target values now follow existing `vibe_wire::behaviour::release_manifest::SUPPORTED_DISTRIBUTION_TARGETS`. Unknown values refuse. Output installer names use `.exe` for Windows; profiles use the native consumer's `profile-windows-auto.toml`, `profile-macos-auto.toml` or unchanged Linux name. Linux publication's current-user0700/private-stage/no-replace contract is unchanged and does not become Windows/macOS filesystem support.

Actual historical source is `576e6a8623c10087c9ad5a0730754d6ba9d61be6`, version1.0.12. Inputs were read only from retained nonprivate delivery staging `/fast/tmp/corp-final-delivery-576e6a862/{windows,macos}/`, with hashes independently checked against their original `ADMISSION.json` before execution:

| Target | Bundle SHA-256 | Installer SHA-256 |
|---|---|---|
| `x86_64-pc-windows-msvc` | `14c6f989920e85db3e64676a755b35a940042e2321e3de6b04c7de6a2dbbb777` | `c9b2318b2065ae919d14e97d22bce3fc013cb3f1dc64479666aec9e9a5f0aad7` |
| `x86_64-apple-darwin` | `4b6cbb78c35767db74c4dfcf761c02a4389e3561fec6b9784955666cd453bdf8` | `7e7858e2487d40a52313a27bad43063b73cd776e08eb6602eb42deebbcc44b12` |

The profile is newly authored synthetic data (`path.fix=never`), not a historical corporate profile. Installer digests prove exact author-selected bytes, not new authentication of role/provenance. The producer never executes them.

## Terminal actual proof

`kit_linux_host_produces_exact_historical_windows_and_macos_inputs` passed1/1 with6filtered, exit0,27.97s in filesystem/PID/network isolation, cleared environment and synthetic HOME/settings. It creates clean local recipe scopes and private sibling output parents, produces both kits through the actual public CLI, verifies exact complete output membership, bundle source/target/version, all components/source through existing `verify_bundle`, all checksum-listed bytes and exact installer bytes. It separately checks collision preservation, target mismatch, unknown target, incorrect digest, version mismatch and corrupted source refusal without published output. No foreign executable runs.

Preserved exact prior CLI SHA-256 `6c45b7c2b6529c3f791ac4c2b8b830f50eca2b9ca0ea2d5f0dd80a379ab26f55` rejects each foreign recipe's validate/build without output. That is deliberate old-reader refusal, not backwards compatibility of new target values. Existing Linux recipe behavior retains its prior shape. After moving filename selection into the shared `vibe-bootstrap::kit::kit_layout(target)` owner used by producer and consumer, the final rebuilt historical-input test passed1/1 (6filtered),27.03s. Target-enum/schema-reader parity passed1/1 (6filtered), checking every existing distribution target plus an unsupported value. Shared layout unit proof passed1/1 (5filtered), preserving Linux naming and unknown-target refusal. Explicit profile selection and subsequent bundle verification remain in their existing consumer order.

Evidence root `/data/git/vibevm-keyring-storage-20261120T0100Z/`: `kit-cross-target-build-r1.jsonl`, `kit-cross-target-runtime-r2.log`, `kit-cross-target-clippy-r1.log`, preserved prior executable and isolated runner. Scoped strict CLI/test lint exited0. The first runner attempt failed before product execution because `CARGO_BIN_EXE_vibe` was missing from its cleared environment; it is retained as r1, not a product failure or successful proof. The corrected r2 supplied the explicit mounted binary.

## Boundaries

This is Linux-host production of two actual historical foreign-target kits, not synthetic shipment, today's foreign release rebuild or native consumer/upgrade. Native Windows/macOS producer execution, current foreign native consume/upgrade, other untested targets and whole campaign final gates remain separately qualified. Historical native functional receipts retain their original source/ordinary-user limits. Independent candidate acceptance and final exact manifest remain separate.
