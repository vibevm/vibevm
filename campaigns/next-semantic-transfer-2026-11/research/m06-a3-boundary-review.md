# A3 materializer / installed-reader boundary review

No concrete P1/P2 finding in the bounded source reviewed. This is not whole-A3 acceptance, runtime verification, or a verdict on generated projector/carrier integration. Root owns acceptance.

No Cargo, tests, Conform, Git, network, new agents or product edits were performed. Worker reports were read as evidence, not rerun proof.

## Reviewed source facts

- `SourcePolicy` binds the full supplied Manifest when either side declares policy, captures manifest plus finite published regular-file inventory, and rechecks identity/bytes/permissions before publication. Missing-manifest legacy compatibility remains separate from explicit empty/export-only policy.
- Inventory walks through retained directory capabilities, rejects path-selector failures without converting Unix literal backslashes into separators, rechecks named directory identity, excludes incumbent `.git`/generated output paths, and requires exact file rows to occur in that payload. Selected links/specials refuse. Directory defaults apply to this finite published payload; no original/global census is claimed.
- Declared preparation parses only selected native inputs by media, keeps raw/CommonMark/XML-data/opaque/unclassified bytes/path/observed permissions, and keeps original native address separate from converted output. Native manifest conversion refuses. Output/native-address aliases and output ancestor collisions refuse during preparation; native fragment invention refuses while literal path `~r` remains valid. VibeDoc Markdown is explicitly refused.
- Recorded observed-file reconciliation compares permissions as well as hashes; foreign byte-matching unrecorded files retain their incumbent inode/mode. Captured Copy and fallback preserve observed bytes/mode; linked candidates compare identity/bytes/mode and never chmod shared source.
- Declared new and recorded-but-absent payload paths use CreateNew, reaching persist_noclobber or atomic hard_link instead of replacement. First record publication also uses persist_noclobber. Recognized recorded replacements retain cooperative semantics. A new-record failure explicitly reports already-published payload paths; no global rollback/no-effects inference was made.
- Workspace preparation remains the sole record parser. It binds supplied held slot, selected coordinate, declaration presence, full declared Manifest, independent package hash/materialized format, current converter recipe, manifest digest and output digests. Recorded manifest coherence precedes legacy downgrade.
- Lower checked rows validate original/output relation, exact-file coverage, native media/address and raw preservation. The known Copied-native mismatch concern is already rejected by this constructor and tested; it is not a finding here.
- `recheck_in` binds logical root, supplied capability path/identity, held descent and manifest. Actual lower reads check workspace/slot/opaque record metadata before and after the chosen buffer and compare that buffer's recorded digest. Bad overlay bytes cannot fall back to disk contents. Ordinary shared source files remain permitted.

## Test sources inspected

Read relevant materializer preservation/conversion/collision/directory tests and lower reader tests, including wrong-suffix media, full Manifest drift beyond PackageMeta, missing independent hash/format/recipe, copied-native mismatch, empty/export-only policy, record/output drift, and bad overlay. No execution claim is made. Parent-provided no-clobber public-apply and hook receipts were treated as existing evidence, not independently reproduced.

## Limits and drift

All 12 owner hashes in the sibling JSON were identical at initial/final capture. Scope was the named materializer/read owners and selected `source_policy.rs::recheck_in`, current PROP-072 laws and integration design, plus relevant direct tests. Generated projector/carrier semantics and complete installer call graph were intentionally outside scope.

No publisher authentication, hostile ABA/CAS, whole-tree transaction, single-link source ownership or expanded native-platform qualification is inferred or requested. Later races may still produce ordinary partial publication under the stated cooperative boundaries.
