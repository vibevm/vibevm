# DS-PROBE-01 — bounded generic probe review

Date: 2026-11-16. Verdict: bounded generic probe code architecturally cleared for
exact v2 below, not live runtime compatibility or whole-package acceptance.

Initial snapshot `/fast/tmp/claude-ds-probe-review-f8x10ai3`, manifest
`824bedabb0d8707de80bfdc1a0220dd3db8c42b391d7fecf9e832372dd735749`.
Successor `/fast/tmp/claude-ds-probe-v2-review-z6w6dbqw`, manifest
`70ddd957eb4354ceb0c9b00238ae6450373484364a90b53c3aa39312ca041d0d`.
Architect independently verified both manifests and all twelve listed regular-file
content hashes, inspected initial implementation/config helper/test boundary and
exact v1-to-v2 changes. Builder reports 90 synthetic tests passed; architect did
not rerun tests or perform live calls. Config reader hash remains unchanged.

## Finding resolved in v2

Initial credential() checked held-file metadata and ancestor-chain continuity but
not the leaf name's identity, used one potentially short read, and did not close
the leaf fd. V2 reads exactly the bounded initial length plus EOF/growth check,
compares no-follow leaf state relative to retained parent against the held file,
rechecks private-leaf policy, and closes fd. Deterministic replacement/short-read
tests assert refusal before exchange. This is finite observation, not credential
rotation CAS or immunity to arbitrary post-observation changes.

## Reviewed boundaries

- Explicit service/model modes, no harness startup. Service never reads auth bytes;
  model sends only synthetic ping to configured selected route with fixed protocol.
- HTTPS default certificate validation, no automatic redirects/proxies/retries;
  response bounded to 64 KiB and model HTTP200 requires Messages-like text content.
- Endpoint/config/auth values passed to isolated Python child on stdin, not argv;
  allowlisted child environment and closed unrelated descriptors. This is a local
  subprocess boundary, not a filesystem sandbox against unrelated host secrets.
- Parent subprocess timeout bounds child DNS/connect/read work and kills/reaps that
  child. Real tests use blocking and stdout-dripping child substitutes, not actual
  DNS/TLS fault injection. No descendant process spawn is part of probe code.
- Output reconstructed as status/http/elapsed only; raw response/header/exception
  contents excluded. Config failures on probe path are sanitized; this review does
  not repair or accept other legacy config/check-config diagnostics.
- Narrow payload allowlist addition and two exact-set test expectations remain
  aligned. Generic probe inputs do not establish private runtime compatibility.

## Runtime and execution limitations

Builder reports a diagnostic invocation before requested review clearance; both
branches refused before HTTP/auth transmission. Service rejected configured prefix
mapping; model rejected credential ancestor permissions under existing reader.
These are admission failures, not endpoint/model availability results. No successful
live service or model verification exists. No extra calls, reader bypass, chmod,
credential copy, prefix stripping or new schema field is cleared here. Any compatible
runtime/configuration change or policy relaxation needs its own applicable authority.

Unchanged scaffold/config/launcher/installer remain unaccepted outside this delta.
No actual credential contents were inspected by architect. Builder's reported private
value scan is not independently rerun or a universal absence-of-secrets guarantee.
No package promotion, host installation, source integration/commit, release or
publication follows. Artifact-capture repair and C0 remain separate tasks/blockers.
