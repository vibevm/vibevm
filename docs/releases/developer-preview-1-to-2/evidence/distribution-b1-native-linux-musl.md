# M-07-B.1 actual Linux musl bundle smoke

Observed on Linux x86_64 from a clean detached checkout of commit
`095187156d1b43f4db7c2d9f36e99af57ae0d428`. The first narrow
`cargo xtask dist build --target x86_64-unknown-linux-musl` exited 1 because
`cc-rs` could not find `x86_64-linux-musl-gcc`; no bundle was produced. The
Rust musl target was then installed, followed by the host packages `musl`,
`musl-dev` and `musl-tools`. The same target-only build, with output directory
`/tmp/vibevm-next-smoke-musl-095187156`, exited 0. No workspace check, full
tests or self-check flag was requested. The fragment names version 1.1.0, the
same source commit and target `x86_64-unknown-linux-musl`.

The explicit-artifact test
`dist::smoke::tests::real_distribution_bundle_native_smoke_from_env` was run
with `VIBEVM_SMOKE_OUTPUT=/tmp/vibevm-next-smoke-musl-095187156` and
`VIBEVM_SMOKE_TARGET=x86_64-unknown-linux-musl`; it exited 0, one test passed.
It verified the fragment and bootstrap, checked the closed bundle and its
components, staged the ZIP's `vibe` in a fresh temporary home, remeasured the
executable, and required an actual child process to return exact `vibe 1.1.0`
with a successful exit. The home was removed after the test.

Artifact SHA-256:

- bundle ZIP: `76820521092e22f263ec06ecff582551f165fa4143bcc9393fe2a6af428eea7c`
- bootstrap: `8e31bce933687c4b71521618c2ef5da410837a52d9c61e6ca370b358bec0f1b7`
- fragment JSON: `9bbb18be63c39df351e9e70e4ab1961072a4d3c8dcba94da0048effbf5999d10`

This is the second native Linux bundle proof at this intermediate 1.1.0
commit, not a five-target apply-ready subject. Windows x64 and both macOS
targets remain unrun on their native hosts. No release host, public Git or
image registry was touched.
