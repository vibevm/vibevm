# APP-VERSIONS Linux acceptance — 2026-11-18

## Measured result and boundary

The current full Linux scenario passes all **13 legs**, exit0. Its unskipped
nvm-sh differential leg passes all **18 cases**, covering **76/76** extracted
command/flag pairs, with **zero exclusions** and verdict0. This is a current
execution, not a reuse of the historical acceptance report.

This establishes the Linux scenario only. The original requirement for actual
**Windows migration remains unverified**. Conditional fish/zsh tests do not become
runtime evidence on this machine. Whole-task owner acceptance, external publication
and release are separate; this report grants none of them.

## Subject and prerequisites

- Product/test source pin: `8ccfc13ed9fcfe7d3871c75b858f3efd6cec127a`.
  Documentation-only integration through `e7b19f01d0b5fdbc6af0a7d02f725d55e85068f6`
  changes neither the running implementation nor acceptance scripts.
- Executed host binary SHA256:
  `c0d152ea875660121b46edc5af42c54516fc61a1a89c7af8dd96c0fb8604c32d`.
- Node package source: `905b6db079a420792d451dd5658a00e293a7987c`.
- nvm shim package source: `39dc54e592a9d458cb72f2573816ae5fbeb8964c`.
- Upstream nvm-sh: v0.41.0, tag object
  `43d410caa64ba7928cef750e31fccbf5790c210c`, commit
  `4e8758a801eea9479b32a904f31093e410ddaba4`.
- Existing `mirror.lock` SHA256:
  `68ceb29701157f96b384964256fed3dd08e8605538efcf6baf976b47490f6e3b`.
  All17 pinned files verify; the lock was not changed.

The explicitly authorized prerequisite retrieval used the existing fetch script,
without `--force` or `--lock`. Both local package copy surfaces contained only
tracked regular files, with no unexpected files or symlinks; selected secret-marker
checks found none. The fresh isolated local registry exactly matched their reviewed
file hashes. This is a bounded copy check, not a universal secret-detection claim.
No credential/configuration change or external package publication was performed.

## Thirteen actual legs

| Leg | Outcome |
| --- | --- |
| Node template installation at a channel | PASS |
| Version listing, origins, channels and pattern filter | PASS |
| Second version and shell environment switching | PASS |
| Prompt hook switching after directory change | PASS |
| Execution at another version without changing selection | PASS |
| Global npm package migration | PASS |
| Template snapshot into a local registry | PASS |
| Closed-network installation from that snapshot | PASS |
| Full nvm-sh differential suite | PASS,18/18cases,76/76pairs,0exclusions |
| Shared keyed version file for two application slots | PASS |
| User extras applied to a new version | PASS |
| Recommendations and explicit decline | PASS |
| Judged post-install steps and unmet-need refusal | PASS |

The unchanged harness runs in a network-isolated namespace. A separate scan of
48 retained outer/differential connect traces found187 INET connection attempts,
all loopback; no non-loopback attempt was found. There are432 retained per-step
observation records. Observed runtime versions include Node20.18.0,22.9.0,22.11.0
and npm10.8.2,10.8.3,10.9.0; absent-runtime observations are also retained.

## Earlier failure is not erased

The first full run exited1:12/13legs passed, but five differential cases failed
with14step mismatches and59/76coverage. Separate retained reproductions of
install-alias and source-build-flags then passed7/7 and11/11steps respectively;
their wrapper exit2 means intentionally incomplete whole-inventory coverage.

Before the successful full retry, only superseded inactive compiler incremental
sessions were removed under the existing cleanup authorization. Latest per-crate
sessions, binary, source, failed-run evidence and incremental compilation remained
intact. No implementation, package, expected result, exclusion or normalization
change was made to obtain the successful retry. Minimum sampled free space during
that retry was2,868,523,008bytes. Resource pressure is a plausible explanation for
the first failure, **not a proven root cause**: its primary installation stderr
was not retained, so an ENOSPC diagnosis cannot be asserted.

## Local reproduction and evidence

With the pinned mirror and reviewed local package registry prepared:

```sh
ACCEPT_TMP=/fast/tmp ACCEPT_KEEP=1 VIBENVM_REGISTRY=/absolute/local/registry \
  bash campaigns/app-versions-2026-11/accept.sh /absolute/path/to/vibe
```

Do not set `ACCEPT_SKIP_VIBENVM`. Allow headroom for all retained per-case
installations; the suite retains their scratch until terminal teardown.

Machine-local evidence, not shipped inputs:

- `/fast/tmp/app-versions-prerequisite-source-receipt-20261118.json`
- `/fast/tmp/app-versions-full-accept-failure-receipt-20261118.json`
- `/fast/tmp/app-versions-full-accept-20261118.log` (first failed subject)
- `/fast/tmp/app-versions-full-accept-retry-20261118.log`
- `/fast/tmp/vibe-appversions-accept.Ja3llf/logs/9.out`
- `/fast/tmp/app-versions-full-accept-retry-evidence-20261118/verified-receipt.json`
- `/fast/tmp/app-versions-full-accept-retry-evidence-20261118/differential/`
- `/fast/tmp/app-versions-full-accept-retry-evidence-20261118/disk.jsonl`

The verified receipt contains subject, package, script, lock and output hashes.
Detailed observation capture was performed by an external read-only supervisor;
it did not edit the running cases or change their verdict rules.
