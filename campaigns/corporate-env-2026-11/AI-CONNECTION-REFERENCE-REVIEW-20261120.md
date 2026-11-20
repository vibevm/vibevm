# First durable AI metadata reference — bounded candidate

Baseline accepted package review `a21caaefb`; first-consumer contracts integrated `a7a635c86` and `238ab2dfb`. CORP-FMT-052 records layout before activation. Candidate, not provider setup or full AI connection completion.

## Ownership and transaction

New `vibe-connections` is the sole AI metadata owner. `vibe-settings` remains layered UI preferences, not an additional reference store. Generic `vibe-secrets` has no AI crate dependency or AI config parser. Its fixed trusted participant descriptor `ai` points to a strict generic schema1 index under secrets/ai: required items maps connection IDs to immutable current_revision, logical secret_ref and metadata SHA256. config.toml contains label/endpoint/api_kind/default_model but no duplicate authoritative reference or credential.

A trusted in-process fixed-descriptor writer accepts only bounded metadata and reference actions, not root handles, arbitrary paths or removal callbacks. It is not caller authentication or proof that supplied bytes are nonsecret; trusted owner code must validate/classify them, and no generic model/tool API exposes it. No plaintext getter is introduced. The generic reader verifies held private metadata bytes/digest and live references without parsing AI payload meaning.

Inventory lock precedes store.lock. Before any first binding the ai participant is registered durably; item liveness is checked while both guards cover immutable metadata publication followed by atomic reference index selection. Interrupted registration can leave a safely unknown missing owner index; it cannot authorize empty inventory. Registration is never removed after last unbinding. Valid empty inventory requires an explicit required items table; missing items is corruption, not serde-default empty. Schema1 keyring stores are not promoted. Old pre-consumer schema2 clients refuse the durable unknown participant.

Index and metadata files are each bounded; aggregate loaded metadata is capped at4MiB and writer checks the same total before publishing a new revision, avoiding unreadable self-publication. Reserved IDs/config revisions remain after unbinding. Key deletion independently reads current registered inventory under the same serialization. Missing/corrupt/unknown participant state refuses, not zero references.

CLI is only `vibe ai connections list|details|add|remove` through existing dispatch/settings admission. Add supplies explicit reference and metadata; remove needs observed index/current revision plus explicit consent and only unbinds. Endpoint metadata rejects userinfo, query, fragment, controls and non-HTTPS except numeric loopback HTTP for synthetic fixtures. No network validation, model query, inferred native harness capability, default binding or provider process occurs.

## Evidence and limits

Private evidence root `/data/git/vibevm-keyring-storage-20261120T0100Z` retains logs. Public `ai-reference-tests-r2.log`2 PASS establishes separate-process create/list/details, stable logical ref through key rotation, referenced-key deletion refusal with unchanged key index, stale unbinding rejection, explicit unbinding then key deletion, retained key revision and foreign neighbor. URL/unknown-ID negative control refuses without binding. New corruption cases additionally cover missing registered index, missing required items and changed config digest; final logs bind these tests before acceptance.

Architect independently verified actual lifecycle on candidate2ead22e8 and20 concurrent actual CLI bind/delete pairs: exactly one mutation won (14delete/6bind), fresh CLI checked key existence iff bound, and all six bound cases made the separately retained pre-AI21e0cd85 CLI refuse deletion with unchanged key index. These are distinct historical gates, not a substitute for final-source digest/negative review; no duplicate runs solely to inflate counts. The old binary was preserved before this atom under the private evidence root with its exact hash.

Static review found the optional items decoder hazard; required table and explicit-empty serialization fix it. Fixed paths do not sandbox arbitrary same-user callers, and metadata masking does not isolate unrestricted same-user processes. Final candidate manifest binds source/binary and scoped tests/lint. No real credentials, inference or network was used. Remaining: independent final review, complete default/model/agent bindings, native credential delivery, user setup/UI, Windows DACL and distribution; NST is not complete.
