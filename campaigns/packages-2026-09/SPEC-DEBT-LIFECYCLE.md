# Lifecycle/extension campaign — authoritative-spec amendment queue

_Draft only, 2026-08-25. This file is not authoritative and changes no PROP
status. The owner applies or rejects each amendment at the named owning anchor.
Anchors are deliberately kept unchanged._

## 0. Scope and evidence boundary

This first entry is the R1.5 hand-off for the materialisation wave. The evidence
that is safe to cite as landed is:

| Step | Landed evidence | What it proves |
|---|---|---|
| R1.1 | `6d606ef2 feat(vibe-workspace): persist slot ownership` | typed, strict `.vibe-slot.toml`; source/representation identity and per-file SHA-256 rows; record written last; legacy readers retained |
| R1.2 | `1cf4f189 feat(vibe-workspace): reconcile recorded slot footprints` | record-to-incoming diff; unchanged files and mtimes retained; stale owned files removed; unrecorded paths retained; hardlinks replaced without mutating the source/cache; record committed atomically last |
| R1.3 | `6a7f750d feat(vibe-install): let source hashes earn mutable skips` | freshly fetched mutable-source hash compared with the valid record; equality skips payload writes; verify mode still checks payload drift |
| R1.4 seam | `4503fdb6 refactor(vibe-workspace): report exact slot reconciliation` | neutral accounting of actual writes/removals, migration, identity change and repair-only evidence; no hook-policy choice |

R1.4 is **not complete at this checkpoint**. Neutral `MaterialiseReport`
plumbing is landed, but install deliberately does not consume those facts to
select hooks yet. This draft therefore does not claim a final hook-rerun policy.
The policy fork is recorded in §3 for an owner ruling.

## 1. PROP-011 — record-aware materialisation and the mutable hash gate

Owning document:
`vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml`.

Exact owning anchors:

- `spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#materialise-diff`
  — `##SLOT-SKIP`, `##DIFF-ONLY`, `##TRUST-PRESENCE`;
- `spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#local-mutable-source`
  — `##MUTABLE-FRESHNESS`, `##MUTABLE-MATERIALISATION`.

### 1.1 Ready replacement at `##SLOT-SKIP` / `##DIFF-ONLY`

Keep both fact names. Replace their bodies with:

```xml
<SLOT-SKIP fact="true" status="impl/done">**Decision, revised by PROP-054 §9.**
An identity-current dependency slot is skipped. When a slot must be refreshed,
its `.vibe-slot.toml` is the ownership boundary: materialisation reconciles the
old recorded footprint with the incoming shippable tree instead of replacing
the whole directory.</SLOT-SKIP>

<DIFF-ONLY fact="true" status="impl/done">A recorded refresh writes only new or
changed owned files, removes only previously recorded files absent from the new
footprint, and never removes an on-disk path outside the old record. Equal
per-file hashes leave bytes, inode and mtime untouched. A legacy slot without a
record pays one final full replacement and receives a record.</DIFF-ONLY>
```

Status: both remain `impl/done`; this is a successor ruling, not a downgrade.
Evidence: `1cf4f189`, especially `vibedeps/slot_diff.rs` and its red/green
oracles in `vibedeps/tests_diff.rs`.

### 1.2 Ready successor beside `##TRUST-PRESENCE`

`##TRUST-PRESENCE` remains true for immutable sources, but presence is no
longer the complete refresh law. Append this fact immediately after it:

```xml
<RECORDED-REFRESH fact="true" status="impl/done">Presence remains the default
fast-path proof for an immutable, representation-current resolved version.
When that proof does not hold, a valid slot record turns refresh into an owned
footprint diff; a missing legacy record triggers one final full migration, and
a malformed record is a hard error rather than authority to wipe unknown
paths.</RECORDED-REFRESH>
```

Status: new fact lands as `impl/done`. Evidence: `6d606ef2` + `1cf4f189`.

### 1.3 Ready replacement at `##MUTABLE-MATERIALISATION`

Keep `##MUTABLE-FRESHNESS`: resolution still reports an in-workspace `file://`
source stale so the source is re-read and re-hashed. Replace only
`##MUTABLE-MATERIALISATION`:

```xml
<MUTABLE-MATERIALISATION fact="true" status="impl/done">**Materialisation
(§2.3), revised by PROP-054 §9.3.** An in-workspace `file://` slot is never
trusted by version-presence alone. Resolution re-fetches the mutable source and
supplies its current shippable-tree `content_hash`; a valid slot record carrying
the same `source_hash` earns the materialisation skip. A missing, malformed or
mismatched record cannot earn that skip and flows to record-aware
reconciliation. Under `slot_integrity = "verify"`, equality of source identity
does not hide payload drift: the recorded files are still verified and any
divergence is repaired through reconciliation. External immutable registries
retain the ordinary presence fast path; `in-place` packages retain their
dedicated git-native update path.</MUTABLE-MATERIALISATION>
```

Status transition: `impl/work → impl/done`. Evidence: `6a7f750d`, including the
two-run CLI oracle and the verify-after-identity-gate regression test.

### 1.4 Collateral wording check for the owner

At `##trust-presence-why` and `##BYPASS-REINSTALL`, replace “re-copies” or
“overwrites the slot” with “re-fetches and reconciles the materialiser-owned
footprint”. `vibe reinstall --force` is still the bypass; it no longer grants
permission to destroy unrecorded build output. No status transition is needed.

## 2. PROP-022 — copy/hardlink reset now rides the slot record

Owning document:
`vibevm/vibespecs/modules/vibe-workspace/PROP-022-materialization-modes.xml`.

Exact owning anchors:

- `spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#snapshot`
  — `##SNAPSHOT-PIPELINE`, `##HOOK-RESET-REMATERIALISE`;
- `spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#hardlink`
  — `##HARDLINK-MODE`, `##HARDLINK-CONTRACT`.

### 2.1 Ready replacement at `##HOOK-RESET-REMATERIALISE`

```xml
<HOOK-RESET-REMATERIALISE fact="true" status="impl/done">For `copy` and
`hardlink` slots, update/reinstall restores the materialiser-owned payload by
diffing the incoming shippable tree against `.vibe-slot.toml`: changed owned
files are replaced, stale owned files are removed, equal files stay untouched,
and unrecorded build output is preserved. Whether hooks run after a repair is
the PROP-020 §2.1 policy recorded in the lifecycle campaign's owner-ruling
queue.</HOOK-RESET-REMATERIALISE>
```

Status remains `impl/done` for the materialisation/reset mechanism. The final
hook sentence stays policy-neutral until §3 is ruled and R1.4 lands.

### 2.2 Ready hardlink corrections

The old `(size, mtime, hash-for-small-files)` wording does not describe the R1
slot record. Replace the affected sentences without renaming the facts:

```xml
<HARDLINK-MODE fact="true" status="impl/done">For packages big in bytes but
modest in file count. Initial placement may hardlink source/cache files into the
slot, falling back to copy when linking is unavailable. Refresh uses the common
`.vibe-slot.toml` path→SHA-256 footprint: equal files are left in place and a
changed destination is unlinked before its replacement is staged, so writing a
slot can never mutate the source/cache inode through a hardlink.</HARDLINK-MODE>

<HARDLINK-CONTRACT fact="true" status="impl/done">The slot still presents a
full shippable tree and its source identity remains `content_hash`. The slot
record is reconciliation and integrity metadata, not a second source identity;
unrecorded paths such as `target/` remain outside the materialiser-owned
footprint.</HARDLINK-CONTRACT>
```

Status remains `impl/done`. Evidence: `1cf4f189` hardlink replacement and
source/cache immutability oracles.

## 3. PROP-020 §2.1 — owner selected payload-diff hook scheduling

Owning document:
`vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml`.

Exact owning anchor:
`spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases`, facts
`##RESET-THEN-RERUN` and `##RESET-IS-MODE-PROPERTY`.

The shared part is settled: for copy/hardlink slots, “reset” no longer means a
whole-slot replacement. It means restoring recorded payload bytes through the
PROP-054 §9.3 diff while leaving every unrecorded path alone. What happens next
had two incompatible contracts; the owner selected 3.A on 2026-08-25.

### 3.A SPEC-WINS — selected and implemented

Any nonempty payload diff, including a `SlotIntegrity::Verify` repair, reruns
the declared hooks once. Empty diff runs none.

```xml
<RESET-THEN-RERUN fact="true" status="impl/done">**On update, reinstall or
integrity repair, hook effects are reset, then hooks re-run if and only if the
materialiser changed the recorded payload.** For `copy`/`hardlink`, reset is the
PROP-054 §9.3 record diff; for `in-place`, it remains the mode's git-native
reset. A zero-change result runs no hook, preventing effects from compounding;
any nonempty result, including verify repair, runs the hook once so the final
slot again represents materialised payload plus declared hook effects.
</RESET-THEN-RERUN>
```

This ruling preserves PROP-020's effective-state
law. If a hook intentionally rewrites a recorded file and that file later
drifts, verify repair first restores the pristine materialised byte. Skipping
the hook at that point leaves the slot in a third state—neither the previously
prepared package nor the declared package-plus-hook result—until some unrelated
future source change happens to rerun the hook. Rerunning once after every
nonempty payload change restores the pure-function model without compounding.

### 3.B TZ-DEMO — rejected

Source/update diffs rerun hooks, but a report whose only disposition is
`repair_only` skips them.

```xml
<RESET-THEN-RERUN fact="true" status="impl/done">**On source update or
reinstall, hook effects are reset and hooks re-run when the source-driven
payload diff is nonempty.** A verify-only repair restores divergent recorded
files but does not run hooks; a zero-change result also runs none. For
`in-place`, reset remains the mode's git-native reset.</RESET-THEN-RERUN>
```

This matches the literal R1.4 demo (“one healed line, hook did not run”), but
accepts the effective-state hole described above. If selected, the exception
must be explicit in PROP-020, not hidden as an implementation detail.

### 3.1 Status and evidence gate

Owner evidence is now complete: `9c545f0d` implements 3.A over `4503fdb6`, with
red-proven repair/identity/in-place/hardlink/reinstall oracles and the full green
panel. The draft replacement above may therefore move to `impl/done` in the
authoritative owner session. `##RESET-IS-MODE-PROPERTY` remains `impl/done`,
with record diff and exact git-native change reports named as implementations.

## 4. PROP-045 — `.vibe-derived.toml` is absorbed by `.vibe-slot.toml`

Owning document:
`vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml`.

Exact owning anchors:

- `spec://org.vibevm.core/vibevm/common/PROP-045#materialisation` —
  `##SETTING`, `##HASH-LAW`;
- `spec://org.vibevm.core/vibevm/common/PROP-045#GENERATED-ARTIFACTS-OUTSIDE-DERIVED`.

### 4.1 Ready replacement for the derived-state sentences

At `##SETTING`, replace “the derived manifest below” with “the slot record
defined by PROP-054 §9.2”. Keep `SETTING` at its current status; R1 alone does
not re-audit the whole setting fact.

Replace `##HASH-LAW` with:

```xml
<HASH-LAW fact="true" status="impl/done">**The hash law under transformation.**
Source identity is unchanged: lockfile `content_hash` and the machine store hash
the source form. A transformed slot is a derived artifact whose identity and
owned footprint are recorded in the single `.vibe-slot.toml`: `source_hash`,
`spec_format`, versioned `converter_recipe`, optional `overlay_hash`,
`derived_hash`, and per-file source/output/disposition/SHA-256 rows. The legacy
`.vibe-derived.toml` is no longer written. Mixed slots verify their recorded
source identity and payload; transformed slots additionally verify recipe,
representation and `derived_hash`. Missing, stale or mismatched state causes
honest record-aware rematerialisation, and changing `spec_format` cannot earn a
presence skip.</HASH-LAW>
```

Proposed transition: `spec/work → impl/done` only if the owner accepts the
already-landed S3–S5 converter evidence together with `6d606ef2`; otherwise keep
`HASH-LAW` at `spec/work` and add the following narrower fact as `impl/done`:

```xml
<SLOT-RECORD-ABSORBS-DERIVED fact="true" status="impl/done">The typed
`.vibe-slot.toml` now owns transformed-slot identity and per-file provenance;
materialisation no longer writes `.vibe-derived.toml`, whose schema-1 reader is
retained only for legacy compatibility.</SLOT-RECORD-ABSORBS-DERIVED>
```

At `##GENERATED-ARTIFACTS-OUTSIDE-DERIVED`, change “the same exclusion genre as
the derived manifest itself” to “outside the slot record's owned payload, just
as the record file itself is outside content identity”. Status stays
`impl/done`.

### 4.2 Legacy transformed-slot migration plan

1. Read preference is `.vibe-slot.toml`; if absent, schema-1
   `.vibe-derived.toml` remains a read-only compatibility source for format and
   verify decisions.
2. Do not eagerly rewrite every vendored slot merely to migrate metadata.
3. At the next real rematerialisation (source/format/recipe/overlay change,
   verify divergence, or forced reinstall), a slot without `.vibe-slot.toml`
   takes the one-time legacy full-replace path and writes `.vibe-slot.toml`
   last. The new materialiser does not write `.vibe-derived.toml`.
4. A valid new record always wins; a malformed new record is a hard error and
   must not silently fall back to the legacy file.
5. Removal of the legacy reader is a separate compatibility ruling after the
   supported-slot corpus has migrated; it is not part of R1.

Evidence: `6d606ef2`, `derived.rs::read_derived_manifest`,
`derived.rs::format_is_current`, and transformed-slot compatibility oracles.

## 5. PROP-054 R1 fact-status queue

Owning document:
`vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml`.

These are proposed authoritative movements after owner review; this draft does
not perform them:

| Exact anchor | Current | Proposed now | Evidence / reason |
|---|---:|---:|---|
| `##SLOT-RECORD` | `spec/plan` | `impl/done` | `6d606ef2` |
| `##REF-SLOT-RECORD` | `spec/plan` | `impl/done` | strict schema, record-last write, generated JTD wire in `6d606ef2`; atomic replacement in `1cf4f189` |
| `##DIFF-MATERIALISE` | `spec/plan` | `impl/done` | `1cf4f189` |
| `##MTIME-LAW` | `spec/plan` | `impl/done` | unchanged-file byte/inode/mtime oracles in `1cf4f189` |
| `##MUTABLE-GETS-A-GATE` | `spec/plan` | `impl/done` | `6a7f750d` |
| `##SELF-HEAL` | `spec/plan` | `impl/done` | `9c545f0d`: 3.A, one-shot post plan, COW hardlinks, exact in-place change report |
| `##AMENDMENT-PLAN` | `spec/plan` | **no move yet** | this file is only the draft; authoritative owners have not applied it, and later lifecycle amendments remain |
| `##R1-DIFF` | `impl/plan` | **no move yet** | code/gate complete through `9c545f0d`; owner amendments still must land |

`##WIPE-TODAY` may retain its historical status but should gain a dated
successor sentence naming `6d606ef2`, `1cf4f189`, and `6a7f750d`.
`##MATERIALISE-CODE-MAP` should now include `slot_record`, `slot_diff/report`,
`slot_cow`, consuming `PostInstallPlan`, registry in-place change detection and
record-aware `vibe-install/src/slot_verify.rs` (evidence through `9c545f0d`).

## 6. Remaining `##AMENDMENT-PLAN` obligations — deliberately deferred

These entries are listed so none silently disappears, but R1 does not prove
their implementation.

### 6.1 PROP-020 §2.3 trust gate

Owning anchors:
`spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#trust-gate`
(`##ALLOW-LIST`, `##FIRST-RUN-CONSENT`, `##NON-INTERACTIVE-SAFETY`) and
PROP-054 `##INSTALL-IS-CONSENT` plus lifecycle §3.5 observability.

Deferred ruling text: installation plus explicit host activation is the
authorisation for declared lifecycle/native code; allow-list, first-run prompt,
CI abort and `--allow-hooks` cease to be the governing extension gate, while
provider/point/handler/inputs/results become visible through narration,
`vibe extensions`, JSON and explain output. **Do not apply yet:** that
observability and activation surface belongs to R2/R6, not landed R1.

Proposed transition later: existing trust facts remain `impl/done` as the
current shipped contract until the replacement implementation lands; then mark
them explicitly superseded at the same anchors rather than rewriting history as
if the prompt never shipped.

### 6.2 PROP-024 §2.3 in-slot native builds

Owning anchors:
`spec://org.vibevm.core/vibevm/common/PROP-024#build`, especially
`##BUILD-CONSUMER-SIDE`, `##HOOK-BUILD`, `##HOOK-RESET-UNAFFECTED`; successor:
PROP-054 `##IN-SLOT-BUILD` and `##BUILD-PHASE-OWNS-IT`.

Deferred replacement: native extensions and declared binaries build under the
package's own slot/workspace, with `target/` and `node_modules/` outside the
shippable tree and outside `.vibe-slot.toml` ownership. R1 makes preserving such
unrecorded output safe; `03f4371d` now maintains the generated vibedeps-root
ignore rules for the already-shipped PROP-025 binary build. The lifecycle build
phase and native build orchestration remain later work. Keep the PROP-024 facts
and PROP-054 native-build facts at their current statuses for now.

### 6.3 PROP-025 build consent and refresh wording

Owning anchors:
`spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#build`
(`##BUILD-CONSENT`, `##SLOT-RESIDENT`, `##REFRESH-INVALIDATES`) and
`spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#staleness`
(`##TRUST-CURRENT-SLOT`); successors: PROP-054 `##INSTALL-IS-CONSENT`,
`##BUILD-PHASE-OWNS-IT`.

R1-ready wording correction: a same-slot record-aware refresh preserves the
unrecorded `target/` and lets Cargo invalidate only affected fingerprints; a
version change still selects a different current slot, so artifacts from an old
slot are never trusted for the new resolution. This corrects
`##REFRESH-INVALIDATES` / `##TRUST-CURRENT-SLOT` without claiming lifecycle
work.

Deferred consent replacement: the lifecycle build phase narrates each declared
artifact and treats installation as consent; the PROP-025 first-build prompt and
allow-list-equivalent gate are retired only when that phase and its
observability ship. Existing `vibe bin` consent behavior is still the current
implemented contract. Do not move its status on R1 evidence.

## 7. Owner application checklist

- 3.A selected; R1.4 landed as `9c545f0d` with a full green panel.
- Apply the SELF-HEAL / RESET status movements now supported by that evidence.
- Apply §§1–4 at their owning anchors without renaming facts.
- Regenerate specmap and run the full repository panel in the authoritative
  owner session.
- Leave §6 trust/native/build amendments deferred until their corresponding
  lifecycle waves land.
- Close PROP-054 `##R1-DIFF` only after the owner applies §§1–4; this draft
  remains evidence and replacement text, never the authoritative edit itself.

## 8. R2/R3 normative conflict queue

This queue records implementation-blocking choices discovered after R1. It
changes no authoritative status. Policy-independent evidence already landed:

| Evidence | What it proves |
|---|---|
| `8d91ccf3`, `eb862d84`, `336db5cf` | nine-phase order, clean-prefixed chains, and all 17 typed extension points |
| `9b27e7b4`, `d62e9652`, `23d0d439` | cycle-free vocabulary ownership, manifest-validation seam, recursive comment preservation |
| `b580bd1d` | strict declaration-only `[[extension]]` grammar and real `vibe check` commissioning boundary |
| `016f0fab` | policy-free, byte-preserving XML-minify kernel; no production binding |
| `03f4371d` | race-safe generated ignores for existing in-slot binary builds |

### 8.1 Lifecycle-state example omits `generate`

Anchors: PROP-054 `##LIFECYCLES`, `##INVOKE-RUNS-PRIORS`,
`##REF-LIFECYCLE-TOML`. The normative state example records requested `build`
as `validate, install, build`, contradicting the closed phase table.

**Owner ruling (accepted 2026-08-25):** persist exactly `inclusive_chain(requested)`, therefore
`validate, install, generate, build`. Treating `chain` as completed-work-only is
rejected because §14.2 calls it the whole requested chain. Blocks R2.5 goldens.

### 8.2 `[[extension.use]]` is not an independent TOML array

Anchors: `##CONTRIB-GRAMMAR`, `##HOST-ACTIVATION`, §14.1. TOML nests
`[[extension.use]]` under the current `[[extension]]` row; without such a row it
is invalid. `b580bd1d` therefore deliberately rejects it.

**Owner ruling (accepted 2026-08-25):** keep declarations as `[[extension]]`; spell activations
`[[extensions.use]]`; keep `[extensions].disable` in that plural namespace.
Alternatives `[[extension_use]]` and inline arrays are representable but less
coherent. The current spelling is not representable. Blocks R2.3 onward.

### 8.3 Host identity and ordering

Anchors: `##HOST-ACTIVATION`, `##ORDER-LAW`, `##REF-LIFECYCLE-TOML`.
Ungrouped projects cannot form `<group>/<name>#<id>`, and separate declaration
and use arrays cannot preserve a source-interleaved order through serde.

**Owner ruling (accepted 2026-08-25):** use an opaque typed host-provider identity rendered as
`__host__/<project-name>#<id>` (never parse it as `PackageRef`). Within the host
tier, direct declarations run in their array order, followed by activations in
their array order. A pure virtual workspace declares no extension. Blocks state
keys, attribution, registry output, disable, and the R2.8 scenario.

### 8.4 Effective stack versus preset tier

Anchors: `##ORDER-LAW`, `##PRESET-LAW`, `##STACK-CONTRIBUTES-PRESET`. A stack's
ordinary declarations otherwise look like dependency-tier contributions even
when the same rows are said to be preset tier.

**Owner ruling (accepted 2026-08-25):** classify `phase:*` contributions from the project's
effective active stack as preset bindings in effective-stack/lock order. Its
`slot:*` and `compile:*` declarations remain ordinary contributions. Blocks the
R2.3 order oracle and R2.7 presets.

### 8.5 Install is a world barrier

Anchors: `##ENGINE-ALGORITHM`, `##PHASE-INSTALL`, `##ORDER-LAW`,
`##ENVELOPE-LAW`. The algorithm currently collects contributions before install
even though install may create or change the installed world.

**Owner ruling (accepted 2026-08-25):** two ritual epochs. Epoch A runs bootstrap built-in
validate/install. After successful install, reload lock, slots, manifests and
effective world. Epoch B narrates and executes extension contributions over the
canonical requested slots, including validate/install contributions, without
repeating the bootstrap built-ins. Blocks R2.2–R2.8 and native bootstrap.

### 8.6 Clean is terminal and never fresh-skipped

Anchors: `##LIFECYCLES`, `##ORDER-LAW`, `##PHASE-FINGERPRINT`,
`##PHASE-STATE-HOME`, `##CHAIN-GENERAL`. Generic freshness could skip a
destructive clean; wiping first also removes providers needed by clean hooks.

**Owner ruling (accepted 2026-08-25):** run clean contributions once before the terminal
built-in wipe; never fresh-skip them; failure stops before the wipe. The wipe is
a terminator, not a preset contribution, and `.vibe/lifecycle.toml` survives.
Blocks generalized CleanChain, R2.5 and clean handlers.

### 8.7 What “nine no-op phases” means

Anchors: `##INVOKE-RUNS-PRIORS`, `##PHASE-VALIDATE`, `##PHASE-INSTALL`; TZ R2.2.
The TZ demo calls all nine rows no-op while validate/install have real built-ins.

**Owner ruling (accepted 2026-08-25):** every requested slot is traversed and narrated. “No-op”
means no extension execution or additional built-in binding in that slot;
validate/install still perform or report fresh, and the other seven rows may be
empty. Blocks the CLI output oracle only, not the phase table.

### 8.8 IR and artifact cardinality

Anchors: `##IR-LEVELS`, `##WHOLE-IR-WIRE`, `##IR-REFACTOR`,
`##REF-IR-UNFROZEN`; evidence `a7a04b69`. Current compiler APIs are one-seed
fragment compilers while plugin wording promises one whole compilation/artifact.

**Owner ruling (accepted 2026-08-25):** `SourceIr` and `DocumentIr` each represent one addressed
document; the parse worklist yields an explicit `Documents(Vec<DocumentIr>)`
batch; `ClosureIr` is the ordered multi-seed graph for one final artifact;
`LaneIr` and `EmittedIr` are each one final artifact including normal/simple
contributions and frame. Existing `compile_static*` remain one-seed compatibility
fragment wrappers, not external-pass invocations. Blocks R3.1 and all R4/R6
artifact-wide work.

## 9. R4 binding blocker discovered by the pure kernel

Anchor: `##TEST-XML-MINIFY`; evidence `016f0fab`. The kernel correctly enables
XML comment validation and refuses internal `--`. Current committed STATIC.xml
markers contain exactly that illegal sequence in preamble/tombstone payloads
such as `<origin-slug>--<original>`. Multi-root framing itself is supported.

**Owner ruling (accepted 2026-08-25):** repair the upstream marker encoding to an XML-valid,
reversible spelling and deliberately update the byte oracles before binding the
kernel. Do not weaken comment validation: that would make an “XML-safe”
transform bless output that is not XML. R4.1/R4.2 production binding remains
blocked after the activation and artifact-cardinality rulings as well.

The separate XML `vibe:end` escape caused by the historical `vibe:close` filter
was red-proven and fixed in `289260fe`; it is no longer part of the R4 blocker.

## 10. Owner ruling checklist for continuation

- 3.A and §§8.1–8.8 are accepted; implementation is authorised, not status edits.
- §9's XML-valid reversible principle is accepted; exact encoding remains open.
- Keep `##OPEN-CREATE-BUDGET` deferred unless R7 should enforce a budget.
- Choose `##OPEN-DEPLOY-TARGETS` before R8, as the TZ requires.
- §§11.1–11.3 are accepted; R7.2 merge/alias/absolute-path details remain open.
- Implement and fully gate first; authoritative PROP text/status
  changes remain an owner-session action using this queue as the draft.

## 11. Dependency-order and wire audit

This audit was run after `624c255d` to find policy-independent work before the
§10 rulings were accepted. It found none beyond the already-landed characterization
and pure kernels: TZ §6 makes R5 depend on R2+R4 and R7 depend on R2. An
experimental uncommitted `vibe-ext` sketch was removed in full after review.

### 11.1 R3 invocation cardinality follows §8.8

`##WHOLE-IR-WIRE` says one call per pass per compilation, while source/document
levels are necessarily per addressed document under the recommended model.

**Owner ruling (accepted 2026-08-25):** source and document transforms run once per
addressed document in the explicit worklist; closure, lane and emitted passes
run once per final artifact. “Whole IR” means the whole value of the level being
passed, never a partial handle. R3.1 remains blocked until §8.8 is ruled.

### 11.2 R5 native ABI must be schema-first and unwind-capable

Anchors: PROP-000 `##JTD-SSOT`, `##JTD-CODEGEN`; PROP-054 `##C-ABI-LAW`,
`##ABI-CRATE`, `##PANIC-AND-VERSION`, `##REF-WIRE-NATIVE`.

**Owner ruling (accepted 2026-08-25):** before the public helper crate, register JTD epoch-1
contracts for native context, native reply and extension manifest; generate
their Rust types into `vibe-wire`; let `vibe-ext` re-export/wrap those generated
types rather than hand-author JSON structs. The manifest root is
`{"extensions":[{"id","point","ir_schema?"}]}`. Context/reply artifact rows
are distinct: accumulated input carries engine-owned `phase`; reply declarations
do not. Native replies expose no `tasks` field. Envelope 1 is checked before the
safe handler runs. ABI return values promise only zero/non-zero; internal error
numbers are not public policy.

`catch_unwind` cannot contain `panic=abort`. `vibe_extension!` therefore emits a
compile refusal under `cfg(panic = "abort")`, with a compile-fail fixture; an
unwinding build gets the required catch boundary. The loader and helper remain
R5 work after R2.4 owns the canonical envelope and R4 owns activation/artifacts.

### 11.3 R7 provider wires and configuration

Anchors: PROP-000 `##JTD-IN-SCOPE`; PROP-054 `##AGENT-CLI`,
`##OPEN-CREATE-BUDGET`. OpenAI-compatible request/response wrappers are named by
the JTD law explicitly; handwritten serde request/response structs are illegal.

**Owner ruling (accepted 2026-08-25):** R7.1 registers and generates Chat Completions request
and response contracts, then implements a synchronous object-safe `chat` seam
with blocking reqwest and a mock transport. `[llm]` in the user config names
`openai-compatible`, model, the full endpoint URL, and a token-file path. Resolve
relative token paths under the config directory; never expand `~`; keyed traffic
requires HTTPS with redirects disabled; keyless HTTP is loopback-only. Secrets
are redacted from Debug/errors and response bodies are never echoed.

R7.2 still needs a ruling for per-field merge with the existing project
`[llm]` (`api_key_env`), provider aliases, and whether absolute token paths are
legal. Keep create-token budgets deferred. Per TZ dependency order, even the
provider seam lands only after the R2 engine exists.
