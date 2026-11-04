# Source-selection stage1 — current public runtime reproduction

Base/HEAD `0e98486dae7ea39e6b553839162256b945ace4a0`, branch `fix/source-selection`, worktree `/fast/git/wt/source-selection`. **TEST ONLY**: the sole working-tree change is untracked `crates/vibe-spec/tests/source_selection_probe.rs`. No src/manifest/dependency/spec/Git/main edits, new workers, network, services or toolchain changes. Four intended-invariant tests remain deliberately RED; this candidate is neither accepted nor ready to ship.

## Exact execution and identity

Command (own target set to this worktree's target):

```text
cargo test --offline --locked -p vibe-spec --test source_selection_probe -- --nocapture --test-threads=1
```

Final: **exit101,10tests,6passed/4failed/0ignored**, wall 0.84seconds. The linked integration executable was `/fast/git/wt/source-selection/target/debug/deps/source_selection_probe-c44f5f83d9bd4c2c`; SHA256 `e5b4bfcc764d1459f6f7f6ac4a27288a2de53d07f4e0a21ec620c0bfc8397561`. The probe SHA256 is `c8d4b7175d82b304daaea4f125d20f59627eca5ce454dfddae8eb04579112b23`. Nine relevant production source files were checked byte-identical to committed HEAD; their hashes and the exact command result are in `/tmp/nst-source-selection-evidence.json`. This is a compiled current-source public library probe, not an assumed-fresh installed CLI.

Raw final log: `/tmp/nst-source-selection-probe.log`. Twenty-one per-case observations extracted from that log are in `/tmp/nst-source-selection-observations.json`. Initial raw log/command/observations are retained with `-initial` suffix. Scoped rustfmt and git diff --check exited0. No production tests were disabled to manufacture a green result.

## Observed outcomes

### Requested anchors

`invariant_requested_duplicate_anchor_refuses_in_section_and_embed` is RED because **Markdown** `#same` returns the FIRST_ONLY body via both FsSectionSource::section_text and actual expand_embeds. It does not return an ambiguity error.

The equivalent duplicate-ID **XML** already refuses during XML projection with “fact id … is defined twice”, before section selection. The current XML behavior is stronger and belongs to its existing parser. A separate unique XML nested-section control reads VALID_XML_BODY through section and embed successfully.

Markdown controls all succeed despite duplicates elsewhere: requested `#unique` returns UNIQUE_BODY, `#left.leaf` returns LEFT_BODY, and `#right.leaf` returns RIGHT_BODY while both branches have a leaf ID. Each corresponding embed agrees. A blanket check that any requested segment appears in the document's global duplicate list would incorrectly reject these currently unambiguous qualified Markdown paths. The prototype A.3 guard has exactly that potential mismatch; do not copy it uncritically.

For the XML control containing duplicate leaf/other IDs, **all three selections** (unique and either qualified leaf) refuse at whole-document validation. That is recorded as existing-policy characterization, not a demand to admit invalid XML or a new source-reader defect.

### Pattern failures

`invariant_matched_document_failures_do_not_disappear_from_pattern` is RED in all three deterministic shapes:

| Matched package/document | Direct point result | Pattern result |
|---|---|---|
| DOC.md + DOC.xml | PairCollision | Ok([]) |
| PROP-042-one.md + PROP-042-two.md | AmbiguousDoc | Ok([]) |
| valid slot/version/spec but common is a regular file | SourceObservation / Not a directory | Ok([]) |

Actual missing documents, unmatched names and a completely empty installed set remain legal empty vectors in the passing controls. A regular file occupying the entire matching slot name also yields point SourceObservation versus pattern Ok([]), but that is a separate **characterization**: whether an incidental non-directory occupant should be treated as an installed wildcard member still needs a policy ruling. No meaningless permission test was used.

### Selected-world membership

`invariant_selected_world_drives_membership_instead_of_unselected_disk_slots` is RED. The resolver is built with selected plugin-a@1 outside the workspace and plugin-only@3 outside the workspace. Point reads return SELECTED_A_V1 and SELECTED_ONLY_V3 correctly. Disk also has unselected plugin-a@9 and an unselected plugin-stray@7.

The pattern emits plugin-a and plugin-stray, omits plugin-only, and reads plugin-a from the selected v1 root while plugin-stray's concrete read refuses UnselectedPackage. Removing only the unselected v9 document removes plugin-a from the pattern result while its selected v1 source/point read remains available. This demonstrates dependence on the wrong membership world. **It does not prove unauthorized stray content was read**: the downstream selected point resolver still refuses that address.

`invariant_selected_document_absence_cannot_borrow_unselected_newer_presence` is also RED. Selected plugin-a@1 lacks DOC; unselected installed plugin-a@9 carries it. The pattern emits a concrete address, but reading it through the same selected resolver returns DocNotFound under the selected root. Presence was tested against one source while the emitted address denotes another.

### Group identity (characterization, not approved policy)

With only org.foreign/plugin-a providing DOC, pattern `spec://org.demo/plugin-*/common/DOC#root` emits `spec://org.demo/plugin-a/...`; reading that concrete address fails because no org.demo slot exists.

With org.alpha/plugin-a lacking DOC and org.zeta/plugin-a containing ZETA_BODY, the org.zeta pattern initially returns empty because lexical same-name dedup chose alpha for membership. Adding DOC only to alpha makes the pattern emit the zeta address; actual reading returns the unchanged ZETA_BODY, not ALPHA_MEMBERSHIP_BODY. These are address-to-membership mismatches in real public paths. The old source comments explicitly prescribe group-blind matching/lexical collapse, so whether the new policy filters the declared group, emits actual groups or refuses ambiguity must be settled by root rather than hidden in a refactor. Merely importing prototype's group-blind duplicate refusal would not fix the single-foreign-group false positive.

### Explicit version (characterization, not approved policy)

Point `plugin-a@1.0.0` reads EXACT_V1. Pattern `plugin-*@1.0.0` emits an **unversioned** address whose actual read returns NEWEST_V2. Even a requested nonexistent `@99.0.0` emits that same unversioned address and reads v2. No unsupported-pin error occurs. The address's explicit selector is discarded by build_address/freshest_spec_root, not by source reading.

The incumbent wildcard comment says every member uses newest installed, while PROP-035 URI-VERSION-OPTIONAL says an explicit pin selects its exact slot. Root must choose honor-pin versus a truthful unsupported-pattern-pin refusal. Silent erasure cannot be described as exact-version selection. No revision-pin policy was tested or changed here.

## Initial test assumption corrected, separately from product RED

Initial run: exit101,9tests,4passed/5failed, wall 40.13seconds. Four failures were the intended invariants above. The fifth was my mistaken cross-format control assertion that XML with globally duplicate IDs should behave like Markdown and return its unique/qualified subsection. The XML parser correctly rejected it under current validation. I retained that initial output, changed the test to explicitly characterize XML's existing refusal, and added a valid unique XML control. Final10tests have only the four intended RED families. There were no fixture-construction or Rust compiler failures; the mistake was my interpretation of an existing format-validation boundary.

## Suggested next fix boundary, subject to root ruling

1. Define ambiguity at the selected Markdown tree-path search scope: the initial unqualified search must not pick the first duplicate; each subsequent child selection must be unique within its resolved parent. Preserve the observed unique/unrelated/qualified controls unless root explicitly changes source semantics. Keep XML's existing global validation intact.
2. Make pattern membership use the same root/version/group that the emitted concrete address will read. With a selected map, enumerate only that map's applicable coordinates/roots; an unrelated disk mutation must not change the set. Preserve “selected package genuinely lacks the document” as nonmembership.
3. Route pattern member lookup through the current F01/F03 safe boundaries and propagate nonabsence document failures; do not use is_ok to erase pair/ID/structural failures. Keep legal empty/unmatched controls and separately decide incidental slot-file handling.
4. Set group/pin compatibility explicitly and convert the characterization cases into the chosen new invariants. Do not keep the known-bad characterization assertions as acceptance or add ignore attributes. No implementation is authorized by this report alone.

F01/F03 compatibility remains required: literal percent paths, exact overlay keys, explicit-root aliases, shared hardlinks, no-follow descendants, custom suffix context, held shared-source bytes and ordinary-file layout fallback. This probe does not revalidate all of them, claim source authenticity/whole-world capture, or justify another pathname/two-read reader, single-link requirement, arbitrary64MiB cap or host/proof framework.

The stage1 file is frozen and intentionally failing; all owned build/test processes have ended. Final /proc cwd/exe/fd inspection found no users of this worktree. Root owns the next semantic decision, continued candidate work or archival/removal.
