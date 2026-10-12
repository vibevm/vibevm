# M-01-C.2 captured-source membership seam review

2026-10-12, source checkpoint 04:53:56 UTC. Read-only research over the named incumbent owners and accepted B.2/C.1 receipts. No tests/builds, provider/cache operations, source edits or new candidate-module discovery. The actor is the sole implementation writer. No C.2 acceptance is claimed.

**The bounded recipe-zero owner already supplies the hard part of a small complete source-tree check. C.2 still needs an independent expected-pin comparison and a retained immutable membership index; neither a slot-record success nor a computed hash by itself supplies those.**

## Minimum existing owner integration

`vibe_registry::compute_content_hash_legacy0_bounded_exact` is publicly exported by `lib.rs` and implemented at `shippable/bounded.rs:36–82`. It:

- validates an exact qualified package pin and a complete bounded row list;
- opens through existing SafeFS, enumerates children against that exact list, rejects unlisted/conflicting children, links and incomplete sets, and compares captured per-file size/SHA;
- parses captured `vibe.toml` and requires its package identity to equal the pin;
- reopens/correlates root identity and repeats the complete observation;
- hashes retained `(path, bytes)` in the existing frozen recipe-zero order with `path || NUL || bytes || NUL`, returning `sha256:<digest>`.

Its current limits are 8 files, 2 MiB/file, 4 MiB aggregate, 12 children per directory, depth 4 (`:19–23`). It is an intentionally bounded source admission, not arbitrary package traversal. It rejects unlisted excluded children rather than promising general build-tree support. Preserve those limits or explicitly refuse unsupported capture; do not quietly truncate membership.

The existing public function returns **only the computed ContentHash**. It does not compare an independently selected expected hash, and it does not return the file bytes it hashed. Its expected per-file rows are explicitly treated as untrusted and checked against actual no-follow observations. Therefore use it as follows within the already allowed install/workspace seam:

1. The trusted preparation owner supplies a bounded complete **original source** capture, including retained file bytes/snapshots and exact package identity. Derive its expected file rows from those retained bytes, not from an editable slot record.
2. Use the incumbent bounded replay against that capture root to establish completeness and correspondence to the retained rows/bytes.
3. Compare its result to the **separately selected** expected package hash and recipe supplied by the current prepared selection/epoch. A hash taken from that same editable capture or freshly generated as its own expected value is not independent evidence.
4. Only on equality seal an immutable path→captured-byte/digest membership index, retaining package/version/expected hash/recipe. The query compares the requested original path and observed bytes with that index; it does not reopen mutable source to fill a missing entry.

No registry implementation change or new dependency from workspace into registry is required by this recommendation: `vibe-install/src/slot_verify.rs` already imports registry hashing and is the existing composition bridge allowed by C.2. A narrow proof constructor there can consume the incumbent public replay. Workspace carries the result and proof strength through its source_query owner. Exact API shape remains the executor's implementation judgment; no new general proof framework is proposed.

## Why incumbent “Verified” is not source-proven

`RegistrySlotVerifier::verify_record` (`slot_verify.rs:76–163`) compares the record's claimed source hash with the expected fetched hash, verifies recorded payload files, and for transformed output compares the payload-derived digest with the record's derived hash. It does **not** reconstruct original source bytes under the independently selected package recipe on that record path. `ValidatedDescriptorMap` similarly binds local manifest/output observations and mapping, not independent original-source membership.

Concrete counterfeit **local-consistency** positive: start from a selected package pin H and an otherwise valid Mixed slot record; alter document D's bytes and update D's recorded SHA accordingly, leaving `record.source_hash = H`. Its recorded file verification can agree and `verify_record` can return `SlotCheck::Verified`. For a transformed slot, also update the local derived-hash claim coherently and retain the expected recipe/overlay facts. This is not a new defect allegation against the verifier's installation role; it is the precise reason C.2 must **not relabel that result source-proven**.

The C.2 deciding case holds the trusted expected pin and verified original capture fixed, changes both slot payload and record digest, and requires the result to remain at most record-consistent or refuse a strict source-proven request. Include record-only, payload-only and coordinated mutations as the commissioned matrix. Add an unrecorded source path and require membership absence rather than permitting it because its bytes are readable or its new row agrees locally.

The positive must actually verify the original capture against H before returning source-proven. A fixture may compute H once from the pristine baseline before tampering; recomputing the expected pin from the mutated candidate at verification time would test self-consistency only.

## Recipe and representation boundaries

- `ContentHash::parse` accepts two labels and intentionally permits short/nonempty hex test spellings. Successful parsing is not successful hash verification. A computed full recipe result must equal the independent full selected pin before granting source-proven; short fixture hashes cannot become proof by shape alone.
- `hash_recipe.rs::order_entries` is the incumbent ordering authority. Recipe 0 uses component-wise Path ordering; recipe 1 uses normalized byte ordering. The current test `the_recipes_diverge_on_a_sibling_below_slash` concretely distinguishes `spec/inner/a.md` versus `spec-x.md`. Do not treat `sha256:` and `sha256-tree/1:` as interchangeable, or replace recipe-zero ordering with a BTreeMap's lexical ordering.
- The existing bounded replay supports recipe 0 only. It is legitimate for an initial bounded C.2 route to return explicit unsupported/unavailable for Tree1 rather than invent a second algorithm, provided the commissioned source-proven positive genuinely runs a supported verified capture. `formats/hash_recipes/1.toml` is recipe-1 data, not instructions to relabel recipe-zero output.
- Original source paths/bytes belong to the verified membership index. Materialized XML/Markdown output may differ from original source; its slot-record `source` mapping alone does not prove conversion. C.3 owns the actual converter/overlay/source-output derivation proof. C.2 can report original-source membership or weaker output record agreement without pretending those are identical.

## Already-provided proofs and limits

Inspected bounded-registry test bodies include frozen legacy parity on nested paths; changed/extra/missing bytes refusal; expected symlink refusal; oversized file and forged-row refusal. No current execution of them is claimed by this review. They establish the intended bounded replay behavior; they do not perform the independent selected expected-pin comparison or return an immutable member index.

Accepted C.1 provides raw bytes/digest from one held observation; accepted B.2 supplies finite governing continuity and selected-world binding. Neither independently authenticates original package membership. Reuse them without widening their claims. Development/self observations can honestly remain observed; local slot agreement can remain record-consistent; strict absent-capture requests must return unavailable offline without fetch/hydration.

No C.2 proof implementation module was named beyond the incumbent files, so no transient missing registration or unreviewed candidate is called defective. The concrete risk above is conditional only on elevating the existing weaker verifier; it is not an assertion that the actor has done so. No further defining owner is essential for this bounded recommendation.

## Reviewed hashes

- `crates/vibe-registry/src/shippable/bounded.rs`: `25ace1acf5dddaccd2e1946cfc4b8f187c5de9fd9d478bc13294d46800beeae6`
- `crates/vibe-registry/src/hash_recipe.rs`: `2eef415dcc1a3a39974acde3a505364b78fc855ebdc7c1c033e8b2988879b0f2`
- `crates/vibe-install/src/slot_verify.rs`: `dd5ceeafcf6f7c43c49f9f85c5763fde9d08fe2b469053877e28ac148cc75d7b`
- `crates/vibe-workspace/src/source_query/capture.rs`: `42454063160aed0637140507aa6c9a7d0589ec8065dc62d5cdbc7029e532898b`

## Actual candidate public-API review — 05:01:08 UTC

Read the newly granted membership implementation, workspace case, install verifier/export and supplied initial logs. **A concrete source-proven bypass exists in this stage. The install wrapper performs the right replay, but it is optional; the public workspace constructor can manufacture the same proof-capable index from scalar equality alone.** No tests were executed by this reviewer and this is not acceptance.

### Reachable proof promotion

`SourceMembershipIndex::from_verified_replay` is public in `source_query/capture/continuity/membership.rs`. It validates recipe prefix/length, `selected_hash == replayed`, exact pin shape, duplicate paths and captured manifest pin. It does **not** recompute a complete-tree hash from `captured` or require an owner-minted result of the replay. Its `replayed` argument is an ordinary `ContentHash`, the same publicly constructible value type as the expected hash.

This is not an inert index: public `WitnessedSourceWorld::output_strength` accepts it and returns `SourceStrength::SourceProven` when its pin/hash equal the selected lock and its indexed member bytes equal the observed output, with Mixed representation and identity source path. That branch performs no mandatory original-tree replay.

A concrete normal composition counterexample:

1. Keep the independently selected package pin and source hash H unchanged.
2. Mutate a Mixed slot's D payload and update the local record/map's D digest coherently. Prepare actual snapshots and a B.2 world from that locally agreeing state. Local record agreement does not reconstruct H, as established in the earlier review.
3. Build `CapturedSourceFile`s from the same tampered document and a manifest that still names the correct package pin.
4. Call `SourceMembershipIndex::from_verified_replay(pin, H.clone(), H.clone(), captured)` directly. The scalar equality passes although H was never replayed over those captured bytes.
5. Pass that index to `world.output_strength(Some((group,name)), D, Some(&index), true)`. The index matches unchanged selected H and tampered observed D, so the inspected branch can return SourceProven.

The independent expected pin is **not forged** in this scenario. The proof API simply accepts a copy of it as “replayed” and treats it as authentication of different captured bytes. No hostile Rust memory access, kernel attack or false snapshot hash is needed. Trusted preparation does not make an ordinary public scalar parameter an actual completed verifier result; the commissioned C.2 contract specifically requires replay before source-proven promotion.

The supplied workspace test `original_index_retains_exact_snapshot_members_and_rejects_wrong_pin` already demonstrates the construction gap: it seals real snapshot members using an arbitrary `sha256:aaaa…` as both selected and replayed hash and expects success. Its comment describes a shape test, but the resulting production type is the one consumed as source-proven authority. Separate green integration of an optional verifier cannot seal that alternate public route.

### What is correct in the install wrapper

`vibe_install::verify_original_source_capture` derives bounded rows from captured per-file byte digests/sizes, invokes the incumbent complete-tree recipe-zero replay against the original root, then compares its result via the index constructor. That path does not fetch/hydrate or duplicate the recipe. The initial install log reports 3/3 including verified capture, wrong pin/changed source and unsupported recipe; workspace log reports the shape case 1/1. These logs do not exercise the public bypass to SourceProven.

### Smallest incumbent-owner correction

Make only the actual replay owner able to mint the **proof-capable** value. The clean existing dependency direction is an install-owned opaque verified wrapper whose private construction follows the canonical replay/expected-pin comparison; keep the workspace index as inert captured membership data. Place source-proven promotion at that same install composition boundary, or accept a genuinely owner-minted sealed replay result if an incumbent type already supplies one. Do not let a public workspace scalar constructor or arbitrary verifier callback produce the proof type. A `pub(crate)` constructor alone cannot be called from install across the crate boundary; moving ownership of the proof seal/promotion is preferable to introducing a registry→workspace cycle or copying the hash recipe.

This is the same narrow input-versus-authority distinction already accepted in B.1, not a new framework. Local observed/record-consistent results can remain workspace-owned; they must not expose a bypassable `SourceProven` route that only checks unverified index equality.

One deciding regression: keep an independently selected pristine H, use real captured tampered D plus matching current record, attempt the ordinary public proof-construction/promotion path, and require refusal or inability to construct the verified wrapper. Keep the genuine `verify_original_source_capture` positive. The commissioned tamper matrix is being written; its temporary absence is not independently counted as a failure here. The issue is the inspected reachable production API.

Hashes at this stage:

- `crates/vibe-workspace/src/source_query/capture/continuity/membership.rs`: `01faf9ad1cdff615a485a6018223221857aa619b02c8c244490828aa3d4a3c2f`
- `crates/vibe-workspace/src/source_query/capture/continuity/m_01_c_2_cases.rs`: `d0a54ad4eb4df63e27b8d80107311a16b8fb2e0a31753ec4b571331b3f423fcf`
- `crates/vibe-install/src/slot_verify.rs`: `bf6a07c5acd9e2d7cef7f669983ba5cacfbca6a4e59f40e16261994acf4495f7`

Later actor edits may close this finding; those are not covered by this checkpoint. No acceptance or deployment/public query claim is made.

## Final corrected candidate review — 05:10:03 UTC

Read the corrected public API, installed-owner tamper cases, workspace catalogue case and supplied red/green logs. **The prior public proof-minting bypass is closed. No further concrete C.2 proof-binding defect was found in this reviewed state.** No tests executed by this reviewer; Root acceptance remains separate.

### Proof mint and promotion are now owned together

`SourceMembershipIndex::from_captured` is an inert catalogue: it retains checked captured file values and package-manifest shape, but accepts no selected/replayed hashes and exposes no source-proven promotion. Workspace `WitnessedSourceWorld::output_strength` can return only Observed or RecordConsistent. A caller constructing arbitrary inert catalogue data cannot pass it where the install strength API requires a verified proof.

`VerifiedSourceMembership` now lives in the install hash-owner module with private pin/hash/index fields. In the inspected module, its production construction occurs only in `verify_original_source_capture`, after mandatory `compute_content_hash_legacy0_bounded_exact(...)` and explicit `replayed == selected_hash`. There is no public raw-hash constructor, deserializer, mutable proof field or public promotion method accepting an inert index. `lib.rs` exports the verified type and checked functions, not a bypassing constructor.

The verifier builds bounded expected file rows from the retained capture values, replay checks their complete actual tree and package manifest, and the sealed index consumes those same owned captures. CapturedSourceFile fields remain private with immutable snapshot access. Thus the checked per-file byte/digest rows correspond to the index that is sealed; scalar equality alone no longer authenticates arbitrary captured contents.

At use, install `source_strength` first obtains local current-world evidence. Its private `promote_output` requires the verified pin/hash to equal the world-selected pin/hash, exact indexed source member bytes to match the observed snapshot, Mixed representation, and identical source/output paths. Passing a proof verified against another independently chosen hash cannot elevate the current world. `original_source_member` likewise checks the world's selected pin/hash before reporting index membership. No provider/cache path or implicit verification retry was added.

Transformed outputs do not satisfy the Mixed/source-path condition and therefore remain locally consistent or refuse strict proof until the later derivation owner. Development/self output remains Observed and strict proof refuses. This is the commissioned finite trust boundary; no further arbitrary-Rust, kernel or converter-proof requirement is inferred here.

### Actual deciding evidence inspected

- `public-seal-red.log`: old public promotion counterexample **0 passed / 1 failed**, with the failed comparison showing SourceProven where the negative expected it not to be. This is executed evidence of the original API defect, not merely a hypothetical.
- `install-c2-sealed.log`: corrected matrix **4 passed / 0 failed / 0 ignored**. Source cases cover coordinated slot+record tamper remaining RecordConsistent and strict refusal; genuine verified original Mixed output becoming SourceProven; record-only and payload-only changes refusing current-world checks; and unrecorded document membership/read refusal.
- The genuine fixture computes the selected hash from a separate pristine original tree, verifies that original capture, and holds that selected hash fixed while constructing/mutating the installed slot. It does not recompute the expected pin from the tampered slot. Its positive case also covers missing-proof non-strict/strict behavior, self Observed/strict refusal, and absent original membership.
- `install-replay-final.log`: **3 passed / 0 failed / 0 ignored** for real replay positive, changed source/wrong independent pin refusal and unsupported recipe refusal.
- `workspace-c2-sealed.log`: inert catalogue case **1 passed / 0 failed / 0 ignored**. It is now explicitly shape/membership data and no longer grants proof strength.

The matrix's record/payload-only mutations occur after a prepared world; coordinated tamper is prepared as a fresh locally consistent world while the pristine proof remains fixed. That distinction is appropriate and necessary. The supplied logs support the bounded cases above, not broad platform qualification. `CANDIDATE-REPORT.md` was not present at the permitted presence check; its transient absence is not treated as a defect or missing acceptance evidence verdict.

### Reviewed final-stage hashes

| File | SHA256 |
|---|---|
| `crates/vibe-install/src/slot_verify.rs` | `2a8db2d17b1ff7594bfe962e270a1ca9344a9f9e4630c046f686288d0265fc6c` |
| `crates/vibe-install/src/lib.rs` | `7e16ee95d93cded18d7ba8bd55c4074a378398e472dba394b32a4b3b74fd8a57` |
| `crates/vibe-install/src/slot_verify/m_01_c_2_cases.rs` | `da0984fe78b5fdc2f8097ac8ac04b2e08c367961be8a6bb447663364b887722b` |
| `crates/vibe-workspace/src/source_query/capture/continuity/membership.rs` | `101e647bc8406eeeeee217a12f3e0758e8be1477b2a6d05a587d198a37c45525` |
| `crates/vibe-workspace/src/source_query/capture/continuity/m_01_c_2_cases.rs` | `e3f9cb51c61bb32f015033b090bb8207b118b19d141b5fd442c93b94b80fe81c` |

Earlier findings above remain historical and are superseded for these exact bytes. Source review recommends no additional C.2 fix; Root still owns final evidence integration and acceptance.
