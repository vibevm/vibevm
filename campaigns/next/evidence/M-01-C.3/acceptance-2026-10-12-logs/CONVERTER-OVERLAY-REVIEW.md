# M-01-C.3 converter and overlay proof boundary

Read-only seam research, 2026-10-12 05:19:08 UTC. C.2 accepted behavior retained. No candidate C3 proof module was granted/read, no tests/builds/effects performed, and no implementation/acceptance failure is inferred from work still in progress. Canonical hash/locator work belongs to the separate reviewer and is not duplicated here.

**Minimum path:** reuse the incumbent descriptor-selected pure SpecDoc parse→overlay→emit kernel over bytes held by the opaque C2 verified original capture, compare its exact output with the B2/C1 retained materialized observation, and mint derivation proof only after every selected identity agrees. Do not call the materializer or an ambient loader during proof, and do not expose a bypassing public recipe/hash-only constructor.

## Existing owners and typed data

- `vibe_install::VerifiedSourceMembership` is the accepted private-field proof capable wrapper. It is minted only by actual complete bounded recipe replay matching the independently selected package hash. The raw workspace `SourceMembershipIndex` is inert. C3 must consume that verified wrapper or an equally mandatory owner-issued projection of it; ordinary captured bytes plus `source_hash` string are not a substitute.
- `derived/selection.rs::classify` selects source interpretation/media **on the original package-relative source path** via the captured package manifest's descriptor owner. VibeSpec supports Markdown/XML; VibeDoc supports XML with Doc package kind. CommonMark, XML-data and opaque sources remain Raw. Unsupported layout/media/ambiguous descriptor selection refuses. Reuse this law rather than admitting any `.md` or parseable XML as Vibe.
- `derived/transform.rs::transform_declared_file` already performs the intended conversion after capturing bytes. Its kernel uses `from_xml_with(text, Vocabulary::Spec|Doc)` or `from_markdown`, then existing `apply_overlay`, then `to_xml` or `to_markdown`. Its return is `Option<(output_relative_path, Vec<u8>)>`. `None` includes legitimate no-conversion cases, so it must not be mistaken for a new derivation proof by itself.
- That wrapper performs a filesystem capture and the legacy `transform_file` performs `fs::read_to_string`; do **not** invoke either whole wrapper from query proof. Extract/share only the existing bytes-to-IR-to-output work or call its already-public pure parser/emitter primitives while preserving the same descriptor/overlay rules. `vibe_specdoc::load_spec_text` reads disk; its pure `project_spec_text_with` sibling is a projection convenience, not arbitrary target-format materialization.
- `derived/overlay.rs::apply_overlay` is already pure. It walks preamble, nested sections and fact-bearing paragraph/quote/list/table/Doc note/figure-caption units, preserving non-status authored fields; fences and other non-fact blocks are untouched. It looks up statuses using a package/document address prefix plus fact ID. Use this implementation rather than a second textual replacement pass.
- The converter recipe has one owner: `vibe_specdoc::CONVERTER_RECIPE == "specdoc/4"`, reexported by `derived.rs`. The slot schema carries converter_recipe, overlay_hash, derived_hash, per-output source and disposition; those are record claims that must be matched to independently selected proof inputs and recomputed bytes. `derived_hash` alone is local record agreement, not derivation authentication.

## Mandatory proof join

Keep the proof mint/promotion in the existing install composition layer, which already owns the opaque original proof. A narrow read-only accessor to the verified captured original member can supply the pure kernel without reopening the source. Before a successful derived result, bind:

1. Verified original capture's package pin/hash to the selected current world, as C2 already does.
2. Exact original member path and its bytes; descriptor selection must derive from the verified/captured package declaration for that source, not merely a compatible descriptor supplied separately.
3. Named supported builtin converter recipe and requested target representation. An unknown/native/custom recipe must remain unavailable; no registry-driven custom execution is allowed by this slice.
4. Authorized overlay for this selected package/context, including explicit absence. Bind the **same parsed overlay value** used by the pure kernel to its captured/hash identity; do not take authorization from the mutable slot record's overlay_hash.
5. Expected source→output path and record disposition, retained selected descriptor map and output observation.
6. Exact recomputed output bytes/digest equal the retained materialized bytes. Keep original raw, resulting representation and later canonical semantic hashes distinct.

A public helper returning recomputed bytes is harmless as an inert transformation. A public constructor that takes those bytes, a caller's recipe label and matching output hash and returns an authoritative “derived” token would repeat C2's bypass unless the mandatory verified original/overlay/world join is inside that constructor's owner. The later query must consume the sealed result, not optionally call the verifier.

## Concrete boundary risks and deciding cases

These are owner-seam constraints, not claims that the currently writing candidate violates them:

- **Optional original verification:** same package labels plus forged original bytes can be converted into a locally matching tampered output. Deciding case keeps the selected pristine original proof fixed, coherently changes output+record, and requires derivation refusal even though converter syntax succeeds. Genuine verified Markdown→XML conversion supplies the positive.
- **Overlay value/hash split:** materialization currently calls `Registry::load(workspace_root)` to get `package_overlay`, then separately calls `overlay_file_hash` (`derived.rs:162–176`). Reusing these two ambient observations directly as a query proof can pair one overlay's statuses with another overlay's hash. C3 should instead consume one prepared authorized overlay value bound to its captured identity. Deciding case keeps original and output fixed but supplies a mismatching overlay value/witness or changes selected overlay authority; no derivation proof may be minted merely because record.overlay_hash matches one supplied string. A no-overlay positive must explicitly prove that absence is the selected input, not silently ignore an authored overlay.
- **Recipe/source-output claim only:** exact output bytes can remain unchanged while a record changes converter recipe or source path. Deciding case alters each selected record field independently and requires refusal against the proof's actual builtin recipe and original member. No broad converter registry is needed.
- **Custom/native fallback:** unsupported conversion must return unavailable and leave any existing native/provider counter at zero; never reroute it through materialization to obtain a passing proof. A supported pure parser error is likewise a refusal, not copied output labeled derived.

The existing C2 limit to small Legacy0 captures remains accepted. It need not be widened for C3; the commissioned positive can use that real supported capture. Transformed output is currently RecordConsistent under C2 precisely until this join is established.

## Exact missing overlay owner

The packet includes pure overlay application but not the definitions/load/hash semantics of `vibe_facts::PackageOverlay`, `Registry` and `overlay_file_hash`. Root was asked for exact **`crates/vibe-facts/src/lib.rs`** to identify their actual defining imports before any further read. Without that file I cannot claim an existing prepared overlay constructor already binds authorization and raw bytes. Do not invent a new authorization subsystem: inspect and reuse that incumbent owner, adding only the bounded capture/join necessary here.

Reviewed hashes:

- `derived/transform.rs`: `7ceea671d9a7c76e23615b7e79744957dc0f38f65a4d4a4e529fff1b40154d54`
- `derived/overlay.rs`: `bb2c6a8883e5d277729789e60b5e2314db33fdab9a1bc12e90e86c1f0664bed5`
- `vibe-install/src/slot_verify.rs`: `2a8db2d17b1ff7594bfe962e270a1ca9344a9f9e4630c046f686288d0265fc6c`
- workspace membership owner: `101e647bc8406eeeeee217a12f3e0758e8be1477b2a6d05a587d198a37c45525`
- `schemas/slot_record.jtd.json`: `56480b324e1c18fde95e26ee19b91acdf3ed61e4e8a904baca2deb6d06117ea3`

No additional framework, native-platform proof or canonical-locator review is requested here. Root owns the final exact candidate review and acceptance.

## Overlay owner follow-up — 05:21:09 UTC

Read the newly granted facts owner files. **The incumbent already has a same-read parsed-registry/witness preparation seam: `Registry::load_with_witnesses`. Prefer that exact snapshot over `Registry::load` plus a later `overlay_file_hash` call. No signing, human attestation or new authority subsystem is needed.**

### Actual authority and byte binding

`Registry::load_with_witnesses` (`vibe-facts/src/store.rs:51–132`) reads each sorted facts TOML file once as raw bytes. It creates its SourceFileWitness from those bytes and feeds **the same bytes** into `parse_file_bytes`. The parse validates UTF-8/TOML/schema, each FactEntry and duplicate addresses (`:234–271`); load additionally requires each entry to live in its canonical registry filename and rejects duplicate addresses across files. `FactEntry::validate` (`lib.rs:199–218`) binds package-origin rows to the package in their fact address. `registry_file_name` (`:221–235`) derives the canonical package file.

`Registry::package_overlay(package)` (`overlay.rs:43–55`) projects only `origin=Package` rows for that exact package into a private-field `PackageOverlay`. Entries with an indeterminate status remain present but do not override a fact status. The authority is the trusted consumer's selected facts-registry context, not the package payload and not the slot record. The rules do not supply cryptographic authentication; that is outside the agreed MVP.

`overlay_file_hash` (`overlay.rs:62–73`) separately reads the package facts file and hashes its raw bytes. It maps **both absence and read failure to None**. Materialization currently loads registry values first and calls this helper later. Consequently the materializer can observe overlay A's parsed statuses and subsequently record hash B (or None on a read failure). This is a concrete two-observation boundary, not a claim that such a race has been reproduced here. C3 must not reuse that split sequence as derivation proof.

`RegistrySnapshot` (`store.rs:210–217`) carries the validated Registry and per-file SourceFileWitness vector. It is a preparation result with public fields, not an opaque OS authority or source-byte capture. Under trusted host preparation, selecting both package projection and package-file witness from the **same actual snapshot** is the minimal current owner composition. The C3 proof-facing adapter should retain those joined values together with the selected consumer context/package, rather than let the output record choose their identity. Query-time proof uses the retained pair and performs no Registry load, facts scan or overlay-file reread.

### Minimal adapter

At preparation, use the existing actual one-read registry snapshot; derive the exact selected package overlay and its canonical package-file witness from that snapshot before splitting it. Preserve explicit absence when the snapshot proves no canonical package file. An absent file and an unreadable file must not become the same successful no-overlay proof; the snapshot loader already reports read/parse failures rather than hiding them.

At the proof boundary, take this prepared package/context-bound overlay input together with the actual opaque C2 verified original capture. Reuse the pure `derived/overlay.rs::apply_overlay` on the parsed original SpecDoc, use the builtin emitter, require record.overlay_hash to equal the retained overlay-file witness (or both explicit absent), and compare recomputed bytes with the held materialized observation. A matching slot-record hash alone cannot select or authorize the overlay. A helper accepting arbitrary `(PackageOverlay, hash_string)` without their preparation join would recreate the split-input problem.

If preparation needs to consume caller-retained **raw facts bytes** without any loader call, the existing private `store.rs::parse_file_bytes` is the pure parse owner to factor/reuse through a narrow facts-owned adapter; also retain the filename/package-membership checks performed by `load_with_witnesses`. Do not copy its TOML parser/validation into the query module. Such a helper would be a literal facts-owner perimeter addition requiring Root's scoped grant; it is not necessary merely to consume an already prepared RegistrySnapshot.

The exact witness fields are defined in imported `crate::scan::SourceFileWitness`; **`crates/vibe-facts/src/scan.rs`** is the next literal read path only if the executor needs that type's representation/accessor details. It was not read here. No broader discovery is needed.

### One deciding overlay-drift fixture

Prepare snapshot A containing a known package fact status and derive its overlay/witness. Change the local facts file to valid bytes B with a different status, and arrange a slot record/output coherent with B. The proof supplied with retained A must not accept B's record hash/output, even though both overlays parse and each local record is self-consistent. A companion positive keeps overlay A, A's witness and output produced by applying A to the verified original, and must succeed. The proof function must make no facts-file read: changing current disk to B must not silently replace its supplied prepared A input. This tests the exact independent-overlay join rather than requiring an atomic global facts registry.

Reviewed facts-owner hashes:

- `crates/vibe-facts/src/lib.rs`: `c7f36d15e244aa43c474cdca9967b0ae68bc9cce3e460feade7a4974e81bb69e`
- `crates/vibe-facts/src/overlay.rs`: `e174d72c2f8d91bf68e6a37690ba4d87c925a630527c4a4575bc9f046200d4e2`
- `crates/vibe-facts/src/store.rs`: `0e137f58ac5b0eec0411b526eb97b5e00fd6a1517f5288a60012e0d0a4fbb549`

No test execution, candidate verdict or acceptance is claimed by this follow-up.

## Final derivation candidate review — 06:04:09 UTC

Read the final granted derivation owner, actual seven-case integration fixture, current shared converter/recorded-output owners, candidate report/hash list and the four supplied selected logs. **No concrete remaining defect found in the reviewed converter/overlay proof boundary under the agreed trusted prepared-process input model.** This closes the initial seam questions for these bytes; Root still owns overall C.3 schema/integration acceptance. No tests/builds executed by this reviewer. Canonical hash/locator audit remains with the separate reviewer.

### Mandatory proof route confirmed

`VerifiedSourceMembership::derive_builtin_output` in `vibe-install/src/slot_verify/derivation.rs` requires the actual C2 private-field verified wrapper as its receiver. It checks the current selected package pin/hash against that wrapper, then reads both original manifest and addressed source member from its sealed capture index. The manifest is parsed from verified original bytes; `replay_declared_captured` applies the incumbent source-descriptor classifier to that manifest/path before conversion. There is no public raw-index/hash-string substitute for the receiver.

`DerivedMaterializationProof` has private fields and read-only accessors. Its construction is at the end of the mandatory method after all comparisons. A caller of the public pure conversion helper gets only `(output path, bytes)`, not this proof type. Thus the old C2 optional-verifier/public-scalar-seal error is not repeated.

Before conversion the method requires a retained selected record with non-Mixed target, exact current builtin `CONVERTER_RECIPE`, Converted disposition, matching prepared overlay package and matching retained overlay raw hash. It obtains the original member via the record's bound source path, recomputes the expected output path and bytes, requires exact path/byte equality with the retained materialized observation, then runs final world continuity and selected pin/hash comparison. Original raw file hash and representation hash are returned separately; package content hash is not substituted for either.

### Pure kernel and overlay binding

`derived.rs::replay_declared_captured` (`:50–75`) calls the existing `classify(package_meta, source)` and passes selected interpretation/media plus supplied verified bytes into `transform_declared_bytes`. The materializer now uses that same bytes→SpecDoc→overlay→emitter kernel after its own capture. The kernel performs no filesystem reads/writes, Registry reload, provider call or custom/native conversion dispatch. The old filesystem materializer is not called by proof.

`PreparedPackageOverlay::from_registry_snapshot` selects the exact canonical package facts-file witness and the same snapshot's `registry.package_overlay(package)`. It rejects duplicate package-file witnesses and a nonempty overlay without its witness; an actual empty/no-file snapshot remains explicit no-overlay. The proof compares this retained input to the current record, rather than choosing an overlay from that record. Per Root's stated MVP boundary, RegistrySnapshot is trusted prepared-process input; its publicly representable fields are not treated here as a new adversarial attestation problem.

The API does not reload current facts during proof. Therefore a retained overlay A remains A even if disk was later changed to B; coherent B output/record cannot satisfy A's hash and replay. This is the intended prepared-context contract, not a promise that an old prepared input continuously tracks every later facts edit.

### Evidence scope inspected

`install-proof-after-media-fixed.log` records **7 passed / 0 failed / 0 ignored**, all seven actual integration cases in `vibe-install/tests/m_01_c_3_cases.rs`:

- real verified original Markdown→XML with matching authorized overlay;
- independently authored valid Vibe XML→Markdown, proving exact source/output mapping, recipe and Converted disposition, original raw/representation/package hash distinctions, and applied status in output;
- coherent tampered output+record cannot borrow the pristine original capture;
- retained overlay A versus coherent published overlay B refuses;
- `native/1` recipe refuses before builtin conversion/promotion;
- explicit no-overlay positive;
- selected source→output mapping mutation refuses B.2 continuity.

The fixture seals the pristine original through actual `verify_original_source_capture` **before** changing source bytes for materialization. It prepares overlay A via actual `Registry::load_with_witnesses` before writing the independently chosen publication overlay. These are genuine independent inputs, not matching fake hash labels. Materialization is fixture setup only; the tested proof method does not invoke it.

`install-xml-original-valid.log` additionally records the exact original-XML case **1/1**. `workspace-kernel-compat.log` and `workspace-kernel-media-compat.log` each record **1/1** existing descriptor/materializer compatibility. Logs are execution evidence supplied by the actor and inspected here, not fresh reviewer runs. The native-recipe test demonstrates refusal and the production method has no native dispatch port; it is not described as a measured native syscall counter.

### Reviewed source hashes

These match the corresponding entries of `SOURCE-HASHES.sha256` for the reviewed owner/test paths:

| File | SHA256 |
|---|---|
| `crates/vibe-install/src/slot_verify/derivation.rs` | `c03b2f5ceca6f18f84f4cd312e79bdef6d512e29682846cbb4eb9a2ca2cf9849` |
| `crates/vibe-install/tests/m_01_c_3_cases.rs` | `167a86e9c4dcd88fee1a9d21e9b94a015ca002054c4475da0e8e972e54ab495f` |
| `crates/vibe-workspace/src/vibedeps/derived.rs` | `6c033fae79084f7903c6872f61529ccf2c7d7318e6c2102e4dc06993413c0e00` |
| `crates/vibe-workspace/src/vibedeps/derived/transform.rs` | `c1a62827416e385e44afb6c87a8d782df701908f2758b41e2df2bb8c749ccb34` |
| `crates/vibe-workspace/src/source_query/capture/continuity/membership.rs` | `ed94fb543fdf1fd092ad25c40a6ebff74356f7b36083e006101765db4439f9b9` |

The accepted small Legacy0 original-capture limit and finite B.2/C.1 observation window remain explicit. No hostile-ABA/native-platform guarantee, broader read-service integration, or final schema/generator acceptance is inferred. No additional converter/overlay correction is recommended by this review.
