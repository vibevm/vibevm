# Corporate agent platform options: verified Pi macOS arm64 path

2026-11-20. Existing `CORP-AGENT-PACKAGES` implementation slice, baseline `5ba0e3a3435b2715d6e8db18d36510791e9c137f`. This adds a real package option, not another Linux lifecycle replay or six-agent completion claim. Architecture packet: `CORP-PLATFORM-OPTIONS-DELIVERY-01.md` in the architect checkout; independent review remains separate.

## Exact supplier input

Retained official Pi release metadata `v1.5.2`, release ID `440309680`, supplies `pi-darwin-arm64.tar.gz`. One finite public HTTPS acquisition used a cleared-environment filesystem sandbox, no credentials/helpers and no local-data upload. Exact bytes: 48,068,183; SHA-256 `56b5e194d485ced555667b8ba3276157a8272e9ecde9e001e428af80985bdd9b`, matching retained official metadata. No latest-version claim is made.

Read-only tar inspection found 234 regular-file/directory entries, no links. The actual executable is `pi/pi`, Mach-O arm64; package metadata identifies `@earendil-works/pi-coding-agent` version `1.5.2`. Thus `payload_root = "pi"` and launcher `$app_root/pi` are supported by actual archive bytes, not guessed from the Linux archive or a supplier filename. The architect independently checked size, digest, regular executable mode and architecture. No foreign executable was run.

Canonical change: `vibevm/vibepacks/org.vibevm.agents/pi/v1.5.2/vibe.toml`, option `macos-arm64-standalone`, platform `macos`, architecture `aarch64`. Existing Linux x86_64 option is unchanged. No npm/Node prerequisite or new selector/extractor is added.

## Actual staged installation, distinct from native execution

Current accepted CLI SHA-256 `6c45b7c2b6529c3f791ac4c2b8b830f50eca2b9ca0ea2d5f0dd80a379ab26f55` selected the explicit platform/architecture/option through the ordinary public install command. The manifest retains its official URL; a fixture-local digest-keyed remap served the exact acquired archive on isolated loopback. Bubblewrap unshared filesystem/process/network namespaces, cleared the environment, supplied synthetic HOME/settings and mounted only explicit binaries, canonical package and inputs read-only, with the fixture evidence directory writable.

Actual install exited0, extracted the Mach-O executable and resources, and persisted the matching `macos`/`aarch64`/`macos-arm64-standalone` receipt. An unsupported macOS x86_64 request refused and left installation-index bytes unchanged. Neither this staging receipt nor the Linux PATH proxy proves native macOS launch/version/uninstall, signing/quarantine behavior or instruction-carrier admission.

First probe refused before installation because its sandbox omitted the required sibling PATH proxy. That fixture failure is retained; the retry mounted the exact existing sibling read-only. No product guard was weakened.

Evidence: `/data/research/vibevm/pi-platform-asset-20261120-8PqT5y/` contains the acquired archive, acquisition log and read-only inventory. `/data/git/vibevm-keyring-storage-20261120T0100Z/pi-macos-staging-r2/` contains actual steps, staged receipt and runtime log, SHA-256 `1fa535b07b201e347cc206e800f9583cfeacf5c725127706423c3c63db42e251`. The focused `corporate_platform_options` test binds the canonical manifest to the real library selector, exact asset/digest/launcher, unique target selection, unchanged Linux selection, unsupported tuples and cross-target option-ID refusal. Offline locked no-run build exited0. The actual compiled focused selector test ran under cleared-environment netless filesystem/PID isolation: 1PASS, 0filtered, exit0 (`pi-platform-option-runtime-r1.log`, SHA-256 `7947b3aecfc3e66c38c9c6563182a449fcee2a84e02aeed153f61a8e7a03bbca`). Scoped `cargo clippy --offline --locked -p vibe-app --test corporate_platform_options -- -D warnings` exited0; targeted rustfmt check and diff checks passed. These checks do not rebuild or rerun the five supplier Linux lifecycles.

## Remaining batch and boundaries

Retained Pi metadata includes other platform assets; actual archive layout is not yet admitted for those rows. Retained Codex metadata distinguishes CLI from helper/app-server/package families; Qwen's CLI release is distinct from SDK and OpenTUI preview. Exact hashes alone do not establish extraction/launcher layout. Claude's foreign native npm child needs its own exact integrity and archive layout; the parent optional-dependency names are insufficient. Hermes needs platform-specific source/wheel/interpreter/ABI producer closure; Linux wheels are not portable substitutes.

Other eligible rows should reuse this selection/staging proof pattern in a bounded batch, not duplicate the five already accepted Linux native lifecycles. Unsupported or missing-input rows remain explicit. Native foreign execution remains NOT RUN. All component/notice/redistribution holds remain; no supplier archive entered Git, corporate drop or external publication. The independent cross-platform kit producer/upgrade criterion is still open and is not implemented by adding an agent package option.
