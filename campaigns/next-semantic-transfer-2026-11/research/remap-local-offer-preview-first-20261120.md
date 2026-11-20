# Offline local-owned Remap offer preview — candidate

Contract `NST-REMAP-LOCAL-OFFER-PREVIEW-FIRST-01`, architect `0901778819d133ba909e061f11191299a33cd9e4`, integrated `71e5cf01f`. Prior source-bound Doc accepted `728623e72`. No installed.json or lock schema change.

## Provenance decision and public operation

`vibe remap preview org.example/policy --input ANSWERS.json --offline --json`

```json
{"selections":{"mode":{"intent":"exact","members":["on"]}}}
```

Strict dedicated selections-only input. Result carries `protocol=vibe-remap-preview/1`, package/version, source `{kind:local-owned-offer,content_hash:...}` and separate `selected_values`. This proves only a checked local installed offer, not original supplier provenance. The installed index never retained original source URL/ref/kind; no URI is reconstructed by name, no writer migration is invented, and no registry fallback/hydration is allowed.

Actual owner is `RemapHome.installed` plus the exact existing `package_dir` owned tree. Require exactly one matching recorded offer; bind captured installed-index bytes to that selected index, verify complete tree bytes against the stored original content pin through existing VerifiedPayload, read bounded held manifest, compare package key/version/kind and all offer rows/policy against the actual installed offer, then recheck index/manifest/payload before response. Input errors are value-free closed schema errors. Missing/corrupt tree, ambiguous entries, changed offer/index/manifest, no-follow violations refuse without partial values.

## Shared data, no consent

The existing narrow Doc data-only exactly-one admission/evaluation is extracted into one pure vibe-core helper; no filesystem, consent or persistence enters it. Doc retains its source-bound request/response behavior. Remap preview returns data effects separately and never substitutes them into active remap rows or ledger/provider state. `list`, `explain`, `accept` and `reject` code paths are unchanged. Only explicit accept/reject in the scenario mutates owned consent state. Preview does not grant automatic acceptance or execute a row.

## Gates

CLI check passed. Actual public fixture has valid on/off in fresh processes before acceptance, after explicit acceptance and after rejection, comparing list/explain and complete project/home snapshots. Separate malformed input, stale payload/index, missing tree, duplicate offer and symlink worlds refuse. Fixture seed uses the typed installed-offer owner and an actual pinned local tree, not an installation receipt or supplier claim. Initial actual public gate passed18 worlds: six valid consent-phase/answer combinations and twelve invalid cases, with complete project/home and list/explain preservation. This predecessor pair predates manifest-from-proof snapshot accessor and the added same-version manifest drift worlds; final exact pair must rerun rather than inherit those receipts. Source review replaced a second manifest read with read-only VerifiedPayload::manifest_bytes from the exact full-payload snapshot. Affected core/Doc/registry/wire all-target strict lint passed. Final public suite passed110/110 with zero filtered, including20 preview worlds and existing actual registered Doc controls after the shared helper extraction. CLI SHA-256 `2acac8641b59a4a811e583df9c029171c2be680d92397fb39ac66400b16be69e`; fixture `c12180b15e89f637474be0d5a65798f770a6f2f8258444b1348a8651b3a8fa03`. Complete core initial runtime retained setup RED:1046 passed/two ENOENT because canonical format corpora were not mounted; repeat uses the exact read-only formats tree, not a broader host grant. Core r2 still lacked the empty compile-time `crates/vibe-core` directory needed for literal `../..` path traversal; the bounded runner adds only that empty directory, not source access. Core r3 actually passed1048/1048, wire1 and selected-app controls4 passed; strict fixture lint passed. Initial setup failures remain RED. Raw canonical247-file generation reproduced all bytes before fmt normalization (`remap-preview-codegen-proof-r1.json`); separate workspace fmt-check and scoped diff-check passed. Independent exact-source acceptance remains pending. Doc wrapper retains its prior static safe error wording after extraction.

No package install, server/tool/compiler, target probe, network, source recovery or activation. USER-PROJECT real-agent remains UNRUN; submitted publication-fault injection and unrelated broad-bin enum lint remain outstanding. No parent/NST promotion.
