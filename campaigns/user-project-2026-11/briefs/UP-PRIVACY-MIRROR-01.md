# UP-PRIVACY-MIRROR-01 — selected trees and history closure

Date: 2026-11-16. Bounded builder implementation brief. Existing relayed direct
USER-PROJECT mandate governs execution; architect owns contract/review. No actual
remote calls, publication, credentials/config changes, toolchain changes, source
commit or cleanup authority. Synthetic local Git tests only.

## Exact inputs and perimeter

Base `6a83be1dd35d7bec2de4911ee375c2be3cb8686a`, plus exact committed delivery of
this brief. One-atom input exception: ONLY two core classifier files bound by
`/fast/tmp/up-privacy-export-v4-review-s20tpcfi/CLASSIFIER-INPUT.json`, SHA-256
`0bc93e43463fb3990b8ca08e76988ee9cbe1475f421fa99880d4f2ac3abe6b4a`:
layout.rs SHA `813e1c27bcb0c397a805411d19f4991681f6bf671e2e18ed42422ed75b43a71b`,
layout/personal.rs SHA `b266aa3d9a04feb3887249804f80c18df22b9b1bfe3d5dd0a21d4cc7ea0842a5`,
both under crates/vibe-core/src/. Verify types/hashes/containment/base classification,
minimal compile and record assembled inventory. No other privacy publisher/snapshot,
artifact/root/epoch overlays, moving dirty prerequisites or private runtime inputs.
This is not integration or permission for external execution.

Read-only census `/fast/tmp/up-mirror-privacy-census-qdw7xvwm/CENSUS.md`, reported
SHA `d5cfdde80bb263ddf1bd40e1e1fbfff89215d7a5ef29a1d47fc43eaca6ec3e16`, is evidence,
not product overlay. Architect inspected existing store, listing, fatal-error and
history selection owners. Existing release/source freezes remain protected.

Allowed exact output paths:
- `crates/vibe-cli/src/commands/registry/mirror/git_source.rs`
- `crates/vibe-cli/src/commands/registry/mirror/git_source/personal.rs` (new)
- `crates/vibe-cli/src/commands/registry/mirror/git_source/personal_tests.rs` (new)
- `crates/vibe-cli/src/commands/registry/mirror/source/api_listing.rs`
- `crates/vibe-cli/src/commands/registry/mirror/source.rs`
- `crates/vibe-cli/src/commands/registry/mirror/plan.rs`
- `crates/vibe-cli/src/commands/registry/mirror/history.rs`

Inline tests in admitted owners permitted; focused new modules above avoid oversized
cells. No new dependencies, wire/report fields, general Git backend/framework,
contact/source-policy changes or namespace/schema redesign. Further consumer/test
owner needs exact review, not token storage tricks to evade ownership boundaries.

## Selected version and read boundaries

Store belongs to one invocation. Pass/retain its existing invocation cancellation
token through all new admission, checkout and bundle subprocess work using existing
controlled output. SourceReader::open already receives progress; its sole GitStore
constructor may pass an explicit cloned cancellation token owned by that store.
No ambient global token or unnecessary mirror.rs parameter plumbing is needed. Preserve network admission/proxy/CA policy and credential
redaction; no fallback launcher/remote/transport or environment dump.

After fetching a tag into local scratch, resolve full immutable object/commit/tree
identities. Names-only enumeration of the selected tree, including directory entries
and empty reserved trees, precedes checkout destination creation, checkout-index,
manifest/blob read or hashing. Strict raw Git wire names: reject literal backslash
BEFORE native Path interpretation, invalid UTF8, malformed census, unsupported
components and reserved names. Never read a personal blob to classify it. No error
as empty inventory. Preserve source tag facts and commit/tree identity, not changed
ref tips. No content strip followed by original-identity claim.

read_blob becomes Result<Option<String>> (or equivalent typed distinction): None
only when an admitted selected tree genuinely lacks the requested ordinary path.
Privacy refusal, failed names/object census, malformed object and cancellation are
errors. Direct requested path is admitted too. API listing propagates privacy
errors, never treats them as undeclared coordinate. plan.rs treats typed privacy
refusal as fatal rather than per-version Unavailable; source-policy ContactRefusal
and cancellation retain existing fatal/terminal behavior. Type and error diagnostics
must not dump blobs, private filesystem paths or credential-bearing URLs.

## Keep-history selection and immutable export

Preserve CURRENT selection/advertisement: source heads and tags fetched into
`refs/history/{repo_key}/heads/*` and `refs/history/{repo_key}/tags/*`. Existing bundle
advertises those destination names; do not silently rename them to source heads,
restrict to semver tags or expand to unrelated allrefs.

After fetching, capture exact destination refname -> full object OID census for
this repository namespace. Strictly validate names/OIDs and complete census command.
Refuse unexpected/ambiguous mapping. Operation-owned store is the sole cooperative
writer during admission/export; this is not protection against hostile same-user
object-store mutation. No later fetch/ref mutation may alter admitted roots.

Admit the COMPLETE object closure actually exported:
- Every parent commit, including all merge parents, has its full tree names admitted.
- Annotated tag chains are followed with type/OID validation, retaining original
  tag objects/facts. Commit and direct tree roots can be admitted; direct blob or
  tag-to-blob roots refuse with explicit typed unsupported/privacy reason because
  there is no path context. Do not scan arbitrary blob contents for secrets.
- Tree names include nested/empty directories. Gitlink name is classified; its foreign
  repository history is not implicitly part of the local Git object closure.
- Missing/corrupt/unavailable reachable objects or failed enumeration refuse before
  output creation. Do not follow replace/graft/shallow semantics that omit physically
  exported ancestry; either establish complete actual closure or explicitly refuse
  those unsupported store postures. No shallow/latest-only acceptance.

Before bundle creation, verify exact selected ref mappings still match the captured
census. Export the EXACT admitted ref list, not a wildcard that can include late refs;
retain original advertised destination names. With exclusive operation-owned writer
posture this preserves mapping; if implementation allows intervening mutations, use
an immutable export store preserving the names instead. Do not substitute bare OID
arguments if that drops bundle head names. No global ref edits or arbitrary allrefs.

Verify produced bundle heads against admitted refname/OIDs before returning it and
before downstream publication. Bundle internal private staging creation may happen
only after closure admission; failure preserves truthful effects and does not claim
external publication. Existing bundle size/history comparison policy remains. A
refused history must stop the invocation, not quietly skip the offending repository.

Git metadata (tag/commit messages) already read for history facts is not personal
boot content classification and is not a universal secret scan. Local source fetch
can transfer objects before name admission; scope is preventing personal blob
checkout/read/export, not asserting zero private bytes ever enter a scratch pack.

## Gates and delivery

Synthetic local Git repositories, no live host/network/push:
- Missing reserved blob selected tree refuses before read_blob/checkout, no dest;
  absent ordinary manifest still returns None only after successful admission.
- Latest tree clean but ancestor private; merge second parent private; annotated
  tag -> tree and tag chain -> private tree; blob target explicitly refuses.
- Empty reserved tree, current/legacy/nested/case and raw Git backslash paths.
- Unrelated refs excluded, exact existing head names/OIDs retained; captured mapping
  mutation/late extra ref cannot change export closure, HEAD movement irrelevant.
- Missing/corrupt closure, failed/malformed names census, replace/graft/shallow posture
  stop before bundle output. Clean complete history remains a readable exact bundle.
- API listing and plan propagate typed refusal as fatal, not None/Unavailable.
- Actual cancellation during slow Git child, acknowledged terminal behavior and
  no false bundle success; source-policy and ordinary mirror adjacent tests unchanged.

Record commands/nonzero selected counts, actual failures/unrun gates, exact output
manifest/diff, closure algorithm/namespace proof and frontdoor matrix. Reuse unchanged
valid evidence. Architect reviews complete atom before acceptance. Mirror source and
history closure are only this remaining frontdoor, not global privacy/USER acceptance
or permission to activate composed personal producers. No unrelated automation.
