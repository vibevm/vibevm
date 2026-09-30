# M-07-B.1 actual Linux GNU bundle smoke

Observed on Linux x86_64 from a clean detached checkout of commit
`095187156d1b43f4db7c2d9f36e99af57ae0d428`. The exact command
`cargo xtask dist build --target x86_64-unknown-linux-gnu --out-dir
/tmp/vibevm-next-smoke-gnu-095187156` exited 0. No workspace check, full tests
or self-check flag was requested. The fragment names version 1.1.0, the same
source commit, and target `x86_64-unknown-linux-gnu`.

The intentionally ignored, explicit-artifact test
`dist::smoke::tests::real_distribution_bundle_native_smoke_from_env` was run
with `VIBEVM_SMOKE_OUTPUT=/tmp/vibevm-next-smoke-gnu-095187156` and
`VIBEVM_SMOKE_TARGET=x86_64-unknown-linux-gnu`; it exited 0, one test passed.
It verified the fragment and bootstrap, checked the closed bundle and its
components, staged the ZIP's `vibe` in a fresh temporary home, remeasured the
executable, and required an actual child process to return exact `vibe 1.1.0`
with a successful exit. The home was removed after the test.

Artifact SHA-256:

- bundle ZIP: `3ee9830e9489706c047f961b93e3e89902006e7b4b4feead0823505ea5bf07ec`
- bootstrap: `9f63ddd479564f271e4c3a5be3059054a7b7c651871828836973602ff70bfba5`
- fragment JSON: `37a782366c2023d2f95c1b84e5465946015ef5a4a38159f4effc256d2e825f78`

This is one native bundle proof, not a five-target apply-ready subject. Linux
musl was proved separately in `distribution-b1-native-linux-musl.md`; Windows
x64 and both macOS targets remain unrun on their native hosts.
No release host, public Git or image registry was touched.
