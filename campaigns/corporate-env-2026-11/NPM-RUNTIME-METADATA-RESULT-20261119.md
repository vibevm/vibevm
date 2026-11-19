# npm runtime binding and streaming metadata — candidate evidence

2026-11-19. Architect independently accepted the exact ten-file metadata implementation at manifest `5f5e1e10a12c73e457cccd27e8500be95265606a082e7f53d0177b2daaafe473`, committed as `307e619617026d4144fdea64fe2d0f9b47899fdd`. Automatic Node installation, consent and target continuation are not implemented or accepted by this report.

## Contract and owners

Optional installer-option `npm_runtime` references an existing step ID; a `sources` step additionally requires its exact named input, while an archive step forbids an input name. The existing acquisition declaration supplies the concrete SHA-256. The binding also names exact npm package/stable version and `engines.node`; no new URL, resolver or shell command is admitted. Absent-field serialization is unchanged. The initial range subset is exactly `>=major.minor.patch`, stable Node candidates only. Unknown/cross-step/unpinned inputs, mismatched package metadata and unsupported ranges refuse.

The application verifier streams compressed SHA-256 and decoded gzip/tar through the same read chain, with checked u64 counters and bounded resident memory independent of package size. Only one regular `package/package.json`, at most 1 MiB, is retained. It is not trusted until the entire gzip stream/footer and input digest pass. No extraction or lifecycle execution occurs. Non-padding data after tar end markers is refused, including a concatenated tar hiding another metadata member. Existing operation interruption is polled during reads and final drain; policy interruption is not a successful verification or a provider install. There is no fixed package-size ceiling by default.

Earlier 64 MiB package-cap and buffered decoded candidates were superseded and not accepted. Their tests/logs remain historical negative evidence. A tar read limit alone was insufficient because artificial EOF could hide oversized/corrupt trailing input; final streaming code explicitly consumes and validates the remaining stream.

## Actual checks

- Core runtime binding tests: 2 PASS, including old-field absence, TOML round trip, source/archive address rules, exact identity/range and unsupported range refusals.
- Real CLI `--on-missing-command=defer` with a valid runtime binding and empty acquisition cache: 1 PASS; target HTTP request count remains zero, no installed receipt/proxy/work/version file. Fresh invocation with the needed synthetic command succeeds. This is not Node installation.
- Final application verifier unit suite: 5 PASS; two large-input tests are ignored by the default suite but were explicitly executed separately below. Cases include digest/identity/metadata uniqueness, compressed/decoded budget, truncation/corruption, cancellation, checked counters crossing 4 GiB and overflow, and hidden concatenated metadata refusal.
- Exact upstream `@qwen-code/qwen-code@0.28.0` tarball: npm SHA-512 integrity verified, SHA-256 `32ca2695069210bae82410a8712d8658a7da5779ecbfcd67160adce415a4e1d9`. Compressed 43,527,360 bytes; upstream metadata reports unpacked 158,997,434 bytes. Actual internal `package/package.json` matches identity and `engines.node = ">=22.0.0"`. Final production verifier test PASS, terminal exit 0, approximately 0.34 seconds, maximum RSS 16,468 KiB.
- Generated tar with a 4 GiB + 1 KiB member, metadata after that member: final production verifier test PASS, terminal exit 0, approximately 9.65 seconds, maximum RSS 16,928 KiB. Generated privately at execution; no giant repository fixture. This is decoded-size proof. A physical compressed archive over 4 GiB was not measured; common compressed/decoded read-counter tests establish the numeric boundary separately.
- Strict scoped clippy (`vibe-core`, `vibe-app`, `vibe-cli`, all targets, `-D warnings`) PASS and workspace format PASS on final candidate.

Final exact candidate manifest: private `evidence/runtime-stream-final-candidate.json`, SHA-256 `5f5e1e10a12c73e457cccd27e8500be95265606a082e7f53d0177b2daaafe473`, ten source/spec/format-note files. Final tasks `bnt10wfn8` (unit/lint) and `bdjhdcefn` (real/large explicit tests) both terminal exit 0. Earlier manifests are historical, not acceptance of later tail-validation bytes.

Private proof root `/data/git/vibevm-corp-qwen-20261119T183921Z/`: retained exact npm metadata/tarball, final unit/core/CLI logs, `runtime-real-qwen-final.log`, `runtime-multigib-final.log`, corresponding final RSS receipts and candidate manifest. No npm package was installed, no provider/model/auth was invoked, no archive was published or added to a corporate drop.

## Next bounded atom

Return typed remediation intent from the existing single preflight, release target lock, use the admitted Node supplier with explicit consent, re-probe trusted node/npm identities and versions through the target's effective PATH, revalidate unchanged source/option/answers and continue once. Preserve independent Node result if target fails. The current planner drops resolved answers on an unmet-need error; an ephemeral prepared snapshot is required rather than repeating questions or inferring old consent. This report does not claim that continuation is implemented.
