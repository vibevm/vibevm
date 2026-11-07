# Hosting observations — report-only foundation

Accepted bounded reporting capability for VIBE-NEEDS1. Need1/3 are not complete; accepted Need2 is unchanged. There is no new preference, exclusion or pin setting, and no transport selection or admission change.

## Public meaning

The current project install event and global application plan/list/describe reports expose optional `source_hosting` beside existing source evidence. Describe also exposes `installed_hosting`, derived separately from recorded installed history. Legacy, common-configuration and unresolved-configuration report branches use the same helper.

The generated report-only HostingView contains optional extensible strings:

- `selected_origin`: known hosting of recorded `source_url`.
- `differing_acquisition_route`: known hosting of recorded `fetched_from` only; never canonical fallback.

Both unknown means no view. Canonical GitHub with a recorded GitVerse mirror can show both labels. Store reuse does not invent a new contact. Missing legacy evidence is not reconstructed from current settings. These fields classify address spelling; they do not prove contact, reachability, authentication or permission to fetch.

## Common foundation

`vibe-core::remap::endpoint` observes supported HTTP(S), Git/SSH, one git+ wrapper and conservative SCP forms using URL/Host normalization. It distinguishes local, nonlocal-file/UNC, opaque and network forms, without DNS/config/env/network inspection. Default HTTP(S) ports normalize away; nondefault ports, IDNA and IPv6 are covered. Unsupported helper forms and SSH aliases remain unknown. Userinfo/path/query/fragment are not exposed by the observation.

Only exact normalized github.com and gitverse.ru currently yield labels github/gitverse. Other hosts are not inferred from substrings, corporate suffixes or presumed self-hosted product identity. Public repository forms were checked against the official GitHub remote and GitVerse cloning documentation linked in PROP-073.

The registry report helper derives both fields from held facts and is shared by all consumers. Existing Locator ordering, safe_remote redaction, offline heuristics and source resolution are unchanged.

## Compatibility and verification

Six report schemas add optional report fields and open label strings. Actual generated report readers remain permissive. SourceEvidence remains strict and unchanged; current index5/steps3/import3 schema/readers and existing corpora are unchanged. The retained-index CLI test verifies byte-identical persisted index contents after report reads. No internal persisted Plan/Index/Steps/Import model gains hosting data and no format epoch or authored vibe.toml grammar changes.

Evidence: core3, registry4, app2, real CLI15, wire8 and doctests1+4 pass. Strict all-target Clippy passes. Fourteen Conform scopes have0findings/0new, without suppression or baseline changes. Generation after formatting is idempotent across245 checked tracked generated files. Final fmt/diff gates pass. Independent classifier/report reviews and root source/receipt/map verification are separate acceptance evidence.

Known-host CLI cases use synthetic retained URLs following real local acquisition; they prove actual report projection, not acquisition from those public services. No external network/API call was made. Runtime evidence is Linux/local fixtures, not native Windows/macOS qualification.

The initial parser bug `github.com::[payload]` could misclassify a remote-helper spelling; the corrected delimiter check and regression preserve valid bracketed IPv6 SCP. A missed unresolved-report constructor and three missing public-type documentation edges were caught and corrected before acceptance; their failure receipts remain preserved.

## Remaining work

Real hosting exclusion requires immutable operation policy before index probes and at every managed transport hop, including redirects, raw-host rewrites, retries, native Git configuration and submodules. Hostname labels and Locator filtering alone cannot provide it. Preference must preserve version semantics and source/manifest coherence. Transitive pin/backtracking correctness and no-write preview remain explicit outstanding requirements.
