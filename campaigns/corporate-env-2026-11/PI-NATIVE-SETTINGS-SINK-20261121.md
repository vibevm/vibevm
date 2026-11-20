# Pi concrete native-settings sink — first E2E candidate

## Implemented boundary

The internal `vibe-connections` adapter facade delegates Pi to a concrete trusted sink colocated with the private secret reader in `vibe-secrets::references::native_settings::pi`. There is no public raw-value getter, caller-provided callback, arbitrary destination-file/key-path operation, CLI export, broker, service, plugin, fork or credential environment injection.

The opaque preview binds the effective keyring scope, injected HOME, connection/reference digest, logical key/current revision, store revision and native content witness. Apply checks the confirmation token, then takes inventory → store → native-settings locks, rechecks references/key revision, target directory identity, protection and native contents before reading the immutable key. Native output is fixed to `.pi/agent/models.json` under the injected HOME. Existing `.pi` and `agent` directories must already be protected; no insecure directory is silently repaired. Windows fails closed through the existing private-storage admission until a witnessed ACL owner exists.

One protected atomic replacement publishes endpoint, native API, escaped literal key and model as a coherent native file. Unknown fields, other providers and other models survive. An existing selected model's endpoint/API overrides follow the confirmed pairing while its unrelated fields survive. A provider marked OAuth is refused rather than silently converted. The sink reads `auth.json` only through the protected bounded native reader, never renders or overwrites it. Exact Pi auth precedence requires refusal when the same provider already has a stored credential; adjacent OAuth remains untouched. Auth content is part of the confirmation witness and is rechecked before publication.

## Executed gates

Evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`.

- `pi-native-sink-build-r3/`, `pi-native-sink-runtime-r3/`: actual offline compilation followed by complete library artifact execution in filesystem/PID/network isolation: **20/20 PASS, no ignored/filtered**. New test covers no-confirm preservation, successful merge, exact native key escaping, neighbor preservation, no-op unchanged bytes, stale native contents, stale key rotation and unsafe 0644 refusal.
- `pi-native-adapter-build-r1/`: facade compilation and isolated actual capability test **1/1 PASS**.
- `pi-native-sink-clippy-r1/`: both affected crates, all targets, warnings denied: exit 0.
- `pi-native-writer-e2e-build-r1/`: bounded evidence helper linked to actual compiled libraries, without adding a product export route.
- `pi160-writer-native-e2e-r2/`: helper creates synthetic key/connection, invokes adapter preview/apply and no-op; the exact official Pi 1.6.0 standalone then independently reads the generated native file and makes one loopback synthetic request with the original literal key. **Exit 0**, no credential environment, no marker output, preserved adjacent provider/foreign field/OAuth file and unchanged native files after direct launch.

The E2E marker includes leading command/interpolation metacharacters. The fixture passes it to the trusted helper over stdin, never argv/environment. Only boolean comparisons are recorded. No real provider/account/auth store or private corporate profile is mounted.

Retained failures: first unit runtime failed during fixture keyring creation because its synthetic settings directory was 0755; corrected fixture to 0700. First E2E failed before native launch because its synthetic HOME was not protected; corrected fixture to 0700. Neither failure relaxed product checks. The standalone native-reader-only evidence remains separate in `PI-NATIVE-SETTINGS-READER-20261121.md`.

## Concurrency boundary

The Vibe native-settings lock coordinates Vibe writers only. Protected snapshot rechecks immediately before atomic replacement detect observed drift, but they are **not a compare-and-swap against an independent native editor**: another process can write between the last check and rename. This operation does not claim to serialize arbitrary native writers or prevent that final external-editor race. Do not edit the same native file concurrently with apply. The atomic replacement guarantee concerns the completeness/protection of Vibe's published file, not exclusive ownership of native settings.

## Public CLI and latest clean qualification

`vibe ai setup --native-settings --agent pi --connection ID` is a separate explicit native-write mode. Unattended execution without a response returns a redacted preview and makes no native write. An exact `--apply-preview` token applies; attended confirmation defaults to keeping settings. An explicit Pi agent-directory override is refused rather than ignored. Other agents are currently refused by this first public slice.

`pi-native-public-clean-build-r2` overlays accepted owner sources plus only the new native declarations, excluding stopped preparation. `pi160-public-native-e2e-r2` passed actual public keyring create via stdin, connection add, no-write preview, wrong-token refusal, confirmed apply and no-op followed by exact Pi standalone read. Receipt SHA-256: `1292bf89765b914b52529acc0768c7ca79803a46e54079f87356671e4773d328`. Architect accepted this Unix existing-home scenario separately. `pi-native-clean-tests-build-r4/runtime.log` records **20/20 PASS** with same-provider stored-auth refusal and directory-replacement token refusal. Earlier successes remain tied to their earlier source subjects.

## Remaining before completion

This is not full CORP-AI-CONNECTIONS or CORP-AI-SETUP acceptance. Complete attended/multi-agent setup integration beyond this explicit Pi route, complete redacted inspect/capabilities interface, fresh native-directory genesis, the five other agent implementations, native Windows ACL/reparse qualification and platform E2Es remain. Native default-model selection outside the explicit provider/model invocation is not yet implemented. Existing stopped owner-file preparation must stay excluded from a scoped accepted commit; run clean-owner verification before promotion. Architect source review accepted the Unix existing-home writer/native-reader and public CLI scenarios; final scoped fingerprint and strict gate remain pending. `pi-native-public-pty-r2` additionally passes Escape, safe default Keep and explicit Apply with terminal settings restored and no secret/provider output. A durable public CLI regression is added through the incumbent `cli_ai_connections` fixture; `pi-native-public-regression-build-r1` compiled the exact clean pair and its isolated runtime passed **1/1, 14 filtered**. `pi-native-public-clean-clippy-r2` completed all targets of all three affected crates with warnings denied, exit 0. No draft sink changes are committed or pushed.
