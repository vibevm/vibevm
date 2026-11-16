# UP-PRIVACY-SNAPSHOT-01 — distribution source admission

Date: 2026-11-16. Bounded builder implementation brief. Existing relayed direct
USER-PROJECT mandate governs execution separately. Architect owns contract/review.
No release/build publication, network, credentials, cleanup, toolchain change,
source commit or provider launch is authorized.

## Exact base and one-atom classifier input exception

Committed source base `6a83be1dd35d7bec2de4911ee375c2be3cb8686a` plus exact committed
version of this brief. Admit ONLY the two classifier files from frozen
`/fast/tmp/up-privacy-export-v4-review-s20tpcfi/CLASSIFIER-INPUT.json`, SHA-256
`0bc93e43463fb3990b8ca08e76988ee9cbe1475f421fa99880d4f2ac3abe6b4a`:
- `crates/vibe-core/src/layout.rs`, SHA-256
  `813e1c27bcb0c397a805411d19f4991681f6bf671e2e18ed42422ed75b43a71b`;
- `crates/vibe-core/src/layout/personal.rs`, SHA-256
  `b266aa3d9a04feb3887249804f80c18df22b9b1bfe3d5dd0a21d4cc7ea0842a5`.
Files reside at matching relative paths directly under that frozen directory.
Architect verified these hashes and the v4 sixteen-file manifest
`3d46ee0a67d439eb073b920bad94beaa9983cb3425e5544cc1f2220589852330`.
Only these TWO files are admitted inputs; no other privacy publisher/checker,
artifact/root/epoch overlay, moving dirty source or private runtime state.

Verify exact file types/hashes/containment/base classification and minimal compile;
record assembled product inventory. Missing prerequisite stops for exact review,
not dirty imports. This exception is for this atom only, not integration/acceptance
or external execution permission. Full Windows runtime remains unrun; classifier
host-aware tests remove a known contradiction without claiming Windows success.
Existing source freezes remain protected. Reuse unchanged valid evidence.

## Exact output perimeter

- `xtask/src/dist/snapshot.rs`;
- `xtask/src/dist/snapshot/personal_tests.rs` (new focused tests, registered from
  snapshot.rs; do not move unrelated existing tests just to enlarge the atom).

Core classifier inputs remain unchanged. No Cargo dependencies, hash recipes,
archive format/schema/version changes, release manifest edits or other producers.
Needed additional owner requires precise review. Existing xtask vibe-core dependency
supplies the shared classifier; do not duplicate layout strings/classification.

## Required ordering and identity

1. In committed_identity, resolve/validate selected immutable commit/tree and admit
   the complete tree NAMES before status or any selected file/blob read. Include
   tree entries (including empty reserved directory trees), not only blobs.
2. If require_clean runs Git status, admit cached INDEX names first as well: clean
   HEAD may coexist with a staged personal path. Strict NUL-delimited/lossless names,
   successful command and component classification are mandatory. Corruption,
   unsupported names and failed enumeration refuse; never treat failure as empty.
   Existing status excludes untracked files; do not newly scan untracked personal
   data. Preserve genuine untracked ignored-file behavior and clean-build policy.
3. In materialise_pinned, validate identity.commit and identity.tree and their actual
   relationship, then repeat names admission for that exact immutable selected tree
   before git archive, ZIP reading, hashing, extraction or source output creation.
   A later HEAD/ref movement cannot change the admitted selection.
4. read_commit_file is a content reader too. Before its selected blob read, admit
   selected commit/tree names and requested relative name using the shared classifier.
   Refuse reserved tree/name before reading any private blob; do not read contents
   to determine whether the name is private. Preserve nonreserved legitimate callers.
5. Reserved selected paths cause refusal, NOT stripping followed by an assertion
   that original commit/tree identity still describes the archive. Names-only tree
   object enumeration is permitted; private blob reads are not needed. No secret
   regex, ignore-file edit or filesystem fallback for missing committed content.
6. Canonical ZIP normalization, portable source topology, regular-file/submodule/
   symlink policy, executable modes, size limits and deterministic timestamp rules
   remain unchanged. Existing content_hash recipes and advertised identity untouched.
   New admission may reject prohibited names before legacy unsupported-type parsing,
   but must not weaken those checks for ordinary entries.

## Gates

Use isolated synthetic Git repositories only, no actual project release/archive.
- Selected tree containing reserved tracked blob whose object is unavailable:
  privacy-specific refusal before archive/show/blob read, not incidental missing-blob
  failure. Test nested current/legacy layouts and empty reserved tree.
- Clean HEAD with staged reserved path refuses before status/content discovery;
  missing working-tree personal file still refuses from index evidence.
- Moved HEAD after pinned identity preserves original clean tree/archive identity;
  supplied commit/tree mismatch refuses. Clean source archive stays deterministic.
- Malformed/failed tree/index census refuses. Nonreserved lookalikes and shared
  boot sources remain valid; untracked ignored personal contents are not read.
- read_commit_file cannot bypass privacy through direct requested private path or
  a different selected commit containing reserved data.
- Existing snapshot/archive tests, focused nonzero selected tests, fmt/clippy/Conform
  for changed owner; report actual failures/unrun platforms honestly. No whole build
  or publication required merely to test admission ordering.

Return exact frozen diff/manifest, input inventory, command/count outcomes and
negative evidence that admission precedes content reads, not merely final omission.
This closes only the distribution snapshot frontdoor. Mirror checkout/history
bundles, other exporters, composed artifact producers and global privacy remain
separate gates; no personal producer activation or whole USER acceptance follows.
