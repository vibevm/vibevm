# UP-ROOT-01 architectural review

Date: 2026-11-15. Current verdict: **v4 architecturally cleared for the root-only
atom; R1-R6 resolved**. The v3 findings below remain the historical review record.
Committed integration and owner acceptance remain separate.
This is architectural review evidence, not owner acceptance or publication authority.

## Exact candidate

- Contract/base: `75ea074313c0942cbd2e196276a49d294169e98b`.
- Candidate: uncommitted frozen v3, 28 product files.
- Manifest SHA-256: `b8a95c1163e93c46b48f83de609165eecc8b82ae28003c99d2a9c92bfd7e77f5`.
- Local evidence locator: `/fast/tmp/up-root-review-v3-8sydhceh`.
- All 28 listed file hashes verified independently. v2-to-v3 comparison verified
  that only the alphabetical position of `user` in the command inventory changed.
- No candidate was integrated into the architect checkout. No implementation edits,
  build, runtime reproduction, cleanup or external publication were performed here.

The following findings result from static inspection of the frozen implementation
and retained filesystem APIs. Deterministic runtime fixtures are requested from
builder; their execution is not claimed by this review.

## Blocking findings

### R1 — Unavailable status reports `ok=true`

`crates/vibe-core/src/user_project.rs:230-234` treats every `Observed` outcome as
success. Workspace `refusal` at `user_project.rs:283-285` correctly sets unavailable
status to `Observed`, but CLI `commands/user.rs:203` derives wire `ok` from that
predicate. A malformed manifest or nonempty foreign root therefore produces a
failure envelope whose nested user-project observation claims success.

Make success depend on the admitted state as well as the outcome. Keep unavailable
status `observed`, with `ok=false`. Cover actual stderr nested reports for malformed,
foreign and unsafe roots, not just outer exit status or envelope `ok`.

### R2 — Definite no-write failure becomes unknown mutation

`crates/vibe-workspace/src/user_project.rs:195-200,213-220,247-261` sets the token to
publishing before a create attempt, discards `ExclusiveChildError` into a string,
and treats the token state as proof of an uncertain mutation. Existing
`ExclusiveChildError::NotCreated` explicitly proves that this attempt created
nothing. On the first creation failing with that variant and no earlier mutations,
the report incorrectly claims incomplete/unknown/recovery-required.

Track actual operation-owned mutation evidence independently of cancellation intent.
Retain distinctions among no creation, creation without verified reopening, retained
verified owned entries and uncertain publication. Use existing token semantics to
record proven restoration/no surviving effects; do not weaken the cancellation
contract or infer ownership from names. Add deterministic no-write create failure,
known surviving-directory failure and genuinely ambiguous failure cases.

### R3 — A newly foreign root is adopted after empty admission

At workspace `user_project.rs:178-183`, an existing root is admitted as empty. The
checkpoint at 212 and following directory preparation do not revalidate its entry
set before manifest publication. A checkpoint poll that adds `foreign.txt` and
returns false demonstrates the missing check: initialization then succeeds in that
nonempty foreign directory rather than refusing the observed input movement.

Revalidate the expected root inventory at the publication boundary, distinguishing
owned prepared entries from foreign additions. Preserve foreign bytes. Test with a
deterministic interleaving, not only simultaneous initializers.

### R4 — Prepared authored directory identities are not verified

Workspace `user_project.rs:218-220` drops the new `vibespecs` capability. The root
identity check at 225-228 proves nothing about the prepared authored directories.
A poll at checkpoint 221 can remove or replace `vibevm/vibespecs` (or rename and
replace `vibevm`), and initialization still publishes a manifest and reports created.

Retain and verify the expected directory identities and layout before declaring
publication successful. On observed movement, preserve ambiguous resources and
report incomplete state rather than success. Cover removal and replacement with
foreign content/symlink sentinels; no unverified cleanup.

### R5 — Root relocation during publication yields stale success

The selected-root identity is checked only before `publish_new_in`
(`user_project.rs:225-232`). That safe-filesystem method then stages and publishes
through a held capability. Its `before_link` seam occurs after the check; moving the
selected root there still allows publication and verification in the renamed held
directory. The operation reports success at the old selected path even when it is
absent or points elsewhere.

After publication, verify that the selected path still identifies the expected root
and prepared layout. Keep publication evidence accurate if bytes were published but
path verification fails; do not undo another process's data. Add a deterministic
relocation/replacement test using the existing race seam. This asks for detecting
observed movement, not an impossible promise of permanent path stability against
arbitrary concurrent writers.

Disposition clarified with builder: if `publish_new_in` returned success, mark the
token committed immediately. A subsequent selected-path/layout verification failure
reports `ok=false`, `state=unavailable`, `outcome=incomplete`,
`publication=committed`, with effects supported by evidence (unknown where survival
cannot be established). Do not downgrade a proven publication to recovery-required.
Cancellation likewise retains committed. This follows the existing report definition
of publication, not a new rollback or repair permission.

### R6 — Plain mutating failure omits the known root

Workspace `user_project.rs:251-256` supplies generic failure reason/remedy strings.
`UserProjectRefusal::Display` (`core/user_project.rs:281-286`) renders only those
strings, so plain/quiet output loses the known root that is present in JSON.
Include the admitted root in the ordinary failure diagnostic and verify plain and
quiet failure output using isolated synthetic paths.

## Cross-cutting disposition

The candidate is bounded to root initialization/status, with ordinary manifest
parsing, separate MCP root, existing safe-filesystem APIs and schema-generated wire
consumers. No global routing, boot composition, memory or override implementation
is claimed. No safety-crate or Cargo dependency change appears in the frozen
inventory. These positive scope observations do not discharge the findings above.

Builder reports focused core 14, workspace 43, CLI user units 16, cancellation 4,
root end-to-end 27, adjacent init 25 and documentation 9 passed; broad regressions
core 1038, workspace 967 with one ignored, and CLI 1298 with one ignored passed.
Strict clippy passed after fixing the inventory ordering defect found by broad CLI
regressions. Code generation repeated with 237 files byte-identical; wire gate
passed with registry shift; conformance reports no new findings, not zero total
findings. These are builder-reported results, not architect reruns.

Full self-check and native Windows/macOS execution are unrun here. Builder reported
an additional project-check environment issue from an omitted derived dependency;
its environment correction and rerun are separate evidence, not a product fix.
Passing tests do not invalidate concrete untested transition failures.

## V4 re-review and final architectural disposition

Frozen v4: `/fast/tmp/up-root-review-v4-7zcf_ep6`, same contract/base, 28 files,
manifest SHA-256
`29834dabaa803e71145bafdc16a498ca240a6b259ba40253cebb6eaaf425a9df`.
All file hashes were independently verified. Exactly five paths differ from v3:
core report/predicate and tests, workspace initialization and failure tests, and
CLI transport tests. The remaining reviewed content is unchanged.

- **R1 resolved:** observed success now excludes unavailable state. Core and actual
  nested CLI failure-report regressions assert false.
- **R2 resolved:** creation distinguishes NotCreated from CreatedNotReopened;
  successful creations retain path/identity evidence. Failure reobserves owned
  entries and separates known surviving changes from uncertainty. A proven first
  no-write refusal reports none/not-started; token intent is terminally restored
  without retrying the same transaction.
- **R3 resolved:** the root is rechecked immediately after the checkpoint, before
  authored writes, with exact expected inventories checked before and after
  publication. The foreign-entry interleaving refuses without adopting its bytes.
- **R4 resolved:** root, authored and specs identities are retained and verified
  with no-follow child operations. Replacement and symlink-sentinel fixtures cover
  the observed defect; ambiguous resources are preserved.
- **R5 resolved:** successful publication marks committed before postpublication
  selected-path/layout verification. Relocation returns incomplete/committed,
  retaining uncertainty rather than downgrading publication or rolling back.
- **R6 resolved:** ordinary failure reason includes the admitted machine-encoded
  root; plain, quiet and JSON regressions inspect that diagnostic.

Builder reports final frozen-v4 affected gates passed without content drift:
core 15, workspace 51, root CLI 29, strict clippy all targets with warnings denied,
and workspace conformance with no new findings. Adjacent init 25 passed with
unchanged CLI production wiring. Previously reported broad regressions, wire and
codegen results apply to their earlier tested revisions; they are not represented
as rerun against v4. Project check passed with zero errors and 55 warnings after
restoring the hash-identical omitted derived dependency fixture. Runtime checks
remain builder evidence, not architect reruns. The architect inspected the final
v4 focused/CLI/clippy/conformance logs and confirmed their reported terminal results.
Builder has started a separate broad post-repair rerun; it is pending, not passed.
The permission fixtures ran under effective uid 1000, as reported by builder.

No remaining blocking defect was established in the five-path re-review. This
clears the reviewed root-only content architecturally, not the whole USER-PROJECT
campaign, final owner acceptance, a committed integration, publication or cleanup.
The candidate is still uncommitted; a committed delivery must preserve and verify
this reviewed content identity and account for accompanying reports separately.

Nonblocking verification limitations: chmod-based permission-denial tests assume
an unprivileged effective user; privileged executions may bypass those mode bits.
The publication-cancellation fixtures prove library/token semantics and explicitly
acknowledge cancellation with a later checkpoint; they are not actual CLI
postpublication cancellation-envelope tests. Native Windows/macOS and full
self-check are not claimed. No guarantee of permanent path stability against
arbitrary external writers is inferred from bounded identity checks.

The next routing contract and all remaining USER-PROJECT obligations stay open.
