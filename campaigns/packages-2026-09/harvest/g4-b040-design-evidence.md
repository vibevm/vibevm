# G4 — B-040 design evidence: `spec/design/typed-seams.md` against the built tree

Measurement only. The verdict on each fact is the boss's to write; every block
below carries the placeholder token in that field, never a judgement. `match`
is the worker's read of how the claim sits against the tree today: `SUPPORTS`
· `PARTIAL` (one-phrase why) · `CONTRADICTS` · `NO-CODE` (a decision /
order-of-work claim, not a code line).

**Perimeter read.** Read in full: `spec/design/typed-seams.md` (90 lines), the census
`campaigns/packages-2026-09/harvest/g1-b040-seams-census.md` (332 lines),
`BACKLOG.md` (B-040 at :1038, B-061 at :111), `crates/vibe-publish/src/{creator,github,gitverse,direct_git,orchestrator}.rs`,
`crates/vibe-publish/tests/repo_creator_oracle.rs`, `crates/vibe-core/src/{content_hash,package_ref,capability_ref}.rs`,
`crates/vibe-actions/src/action.rs`, `crates/vibe-settings/src/events/mod.rs`,
`crates/progress-core/Cargo.toml`, `crates/progress-core/src/{cache,doc,seal,parse/mod,baseline/project,baseline/project/tests}.rs`,
`crates/vibe-cli/src/commands/workspace/tests.rs`, `sync-engines.toml`,
`spec/boot/90-user.md` (SCOPE-DISCIPLINE), `spec/modules/vibe-registry/PROP-002-…md`,
`spec/modules/vibe-progress/PROP-043-…md`, and the GUIDE
`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md`.
~1700 lines read across sources.

Re-measured this session (`crates/*/src` + `xtask/src`, census perimeter):
`pub trait` = **24**; `PhantomData` = **0**; sealed-pattern constructs = **0**
(only the domain `mod seal` — `progress-core/src/lib.rs:36`, `vibe-cli/src/commands/progress.rs:45`);
`#[must_use]` = **145** (vibe-cli 119 / vibe-actions 17 / vibe-settings 9);
`serde(try_from="String", into="String")` attributes = **9**; custom
`impl<'de> Deserialize<'de>` = **2** (`purl.rs:128`, `features.rs:74`).

The doc describes both **before** and **after** states (it was written and
built the same day and refuted itself four times). Landings now in the tree:
(1) `ValidatedOrg` scope gate — BUILT; (2) the four `serde(transparent)`
identity newtypes → `try_from`/`into` — BUILT; (3) builder presence into the
signature — BUILT; (4) `progress_core::Digest` — DECLINED on measurement,
five-line staleness fix built instead; (5) `Watcher` "Specified, not built"
annotation + backlog row — BUILT. Facts whose evidence is a "before" line that
a landing has since replaced carry a `note` saying so and pointing at the new
site; that is recorded as `PARTIAL`, never `CONTRADICTS` (per packet).

---

## companion-line

- **claim.** The design doc is companion lore to BACKLOG B-040, the g1-b040
  census, and the GUIDE `scaffold-b-typed-builders` anchor; non-normative
  (PROPs and backlog rulings win where they disagree).
- **evidence.** `BACKLOG.md:1038` (`### B-040 — рефакторинг-обзор собственных
  швов … {#b-040}`); census at `campaigns/packages-2026-09/harvest/g1-b040-seams-census.md:1`;
  the companion statement itself at `spec/design/typed-seams.md:5`; GUIDE
  section `- ##ln **B — Typed builders / typestate** (`scaffold-b-typed-builders`)`
  in `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md`.
- **match.** SUPPORTS
- **note.** `GUIDE-AI-NATIVE-RUST.md` is **not** at the repo root (no host-root
  copy); it lives in the rust-ai-native-lang package tree (and a vendored copy
  under `vibedeps/stack-rust-ai-native-lang/`). The anchor is the lowercase id
  `scaffold-b-typed-builders` inside the `##ln **B — …**` heading.
- **verdict.** PENDING

## basis-census

- **claim.** The census measured five idioms and found two absent: 24 `pub
  trait` (none sealed); one runtime-validating builder (`ActionBuilder`), zero
  typestate / `PhantomData` absent perimeter-wide; 146 `#[must_use]`, 82 % on
  `vibe-cli` TUI setter chains; mature validating newtypes on the `vibe-core`
  identity seam; ~140 serde types of which **7** validate at load.
- **evidence.** Census headline `g1-b040-seams-census.md:23-39`; the 146 at
  `:181`; the 7 validate-at-load at `:305-315`. Re-measured today:
  `rg "pub trait [A-Z]" crates/*/src xtask/src` → **24**; `rg "PhantomData"
  crates/*/src xtask/src` → **0**; sealed constructs → **0**; `rg -c
  "#\[must_use\]"` → **145** (vibe-cli 119 / vibe-actions 17 / vibe-settings
  9 → 119/145 = 82 %); `try_from = "String"` attributes → **9**; custom
  `impl<'de> Deserialize<'de>` → **2** → **11** validate-on-load today.
- **match.** PARTIAL — the absences and headline idioms hold (24 / 0-sealed /
  0-PhantomData / 0-typestate all confirmed), but three counts are
  census-time (pre-build) and the day's own landings moved them.
- **note.** `#[must_use]` is 145 today, not 146 — the builder landing removed
  the `invoke` setter (`action.rs` went 11→10 setters). Validate-on-load is 11
  today, not 7 — the wire landing converted 4 `transparent` newtypes
  (`ContentHash`, `PackageName`, `CapabilityNamespace`, `CapabilityName`) to
  `try_from`. The "~140 serde types" is not a clean recount: my raw
  `derive(…Serialize…Deserialize…)` line count is 224 (all items incl.
  private/test structs); the census's ~140 scoped pub cross-crate types, so
  only the order of magnitude matches.
- **verdict.** PENDING

## basis-absence-is-not-a-defect

- **claim.** An idiom's absence is a question, not a verdict; `PhantomData: 0`
  is a defect only where a wrong call is representable today; the design hunts
  calls that can be made wrongly rather than scheduling the five idioms.
- **evidence.** The fact itself at `spec/design/typed-seams.md:11`; the
  method shapes the whole doc — §2–§7 each examine one specific wrong-call
  site, not one idiom. `PhantomData` = 0 confirmed (see basis-census).
- **match.** NO-CODE — a methodological statement about how to read a count,
  not a claim about a code line.
- **note.** NO-CODE: the claim is a reading rule ("absence ≠ defect"); its
  only checkable predicate (`PhantomData: 0`) is confirmed, but the fact is a
  decision about interpretation, not a code assertion.
- **verdict.** PENDING

## basis-the-question-that-found-the-work

- **claim.** The build list came from "where does the tree state an obligation
  on a caller/implementor in prose with nothing checking it?"; that question
  crosses the five categories and found four sites, one security-relevant and
  invisible in every census count.
- **evidence.** The fact at `spec/design/typed-seams.md:13`; the four sites
  map to §2 scope (`creator.rs`), §3 digest (`progress-core`), §4 wire
  (`vibe-core` identity newtypes), §5 builder (`action.rs`). The
  security-relevant one is scope — PROP-002 §2.10 "Never escalate scope"
  (`PROP-002-…md:611`).
- **match.** NO-CODE — a decision/methodology claim (the question that
  produced the list), not a code-line assertion.
- **note.** NO-CODE: the "four sites" are the four landings (three built, one
  declined); "invisible in every count" = the scope obligation lived only in a
  doc comment until `ValidatedOrg` (`creator.rs:57-94`).
- **verdict.** PENDING

## scope-the-obligation

- **claim.** `RepoCreator` states an implementor obligation in a doc comment
  and enforces it nowhere; `validate_scope` is a default method with a correct
  body and nothing makes any implementation call it.
- **evidence.** Trait doc `creator.rs:103-113`; default `validate_scope`
  `creator.rs:206-227`. **But today the obligation IS enforced**:
  `repo_exists`/`create_repo`/`push_url` take `&ValidatedOrg`
  (`creator.rs:177`, `:181`, `:195`), and `ValidatedOrg` can only be minted by
  `validate_scope` (`creator.rs:91` `pub(crate) fn new`; `:216-227`).
- **match.** PARTIAL — the doc-comment obligation still exists, but "enforces
  it nowhere" no longer describes the tree.
- **note.** Describes the **before** state of the scope landing (order-list #1,
  BUILT); today the obligation is encoded in the type — the three
  side-effecting methods take `&ValidatedOrg` and the only constructor outside
  the crate is `validate_scope`, so an out-of-crate caller cannot reach a host
  method without the check.
- **verdict.** PENDING

## scope-why-it-matters

- **claim.** The rule is not stylistic — `##SCOPE-DISCIPLINE` and PROP-002
  §2.10 bind the publisher to its declared org; an impl that forgets the call
  compiles, passes review as "looks like the others", and escalates scope on
  its first real run.
- **evidence.** `spec/boot/90-user.md:28` (`##SCOPE-DISCIPLINE` … "Never
  escalate scope"); `spec/modules/vibe-registry/PROP-002-…md:611`
  (`##PUBLISH-NEVER-RULES` "Never escalate scope … adapters MUST refuse …").
- **match.** PARTIAL — both binding rules are present verbatim, but the
  escalation scenario ("forgets the call compiles … escalates on first run")
  is the pre-landing state.
- **note.** The two binding rules SUPPORT the claim's core. The escalation
  scenario is superseded for out-of-crate callers by the `ValidatedOrg` gate
  (forgetting does not compile — `creator.rs:177`); an in-crate adapter can
  still override `validate_scope` (direct_git.rs:73), which is exactly why the
  oracle table-test exists (`repo_creator_oracle.rs:84`).
- **verdict.** PENDING

## scope-the-hole-is-latent-not-live

- **claim.** Measured before the landing: every impl honours the obligation
  today; `github`/`gitverse` open `repo_exists`/`create_repo` with
  `self.validate_scope(org)?` as their first statement; `direct_git` overrides
  `validate_scope` to a no-op and its `repo_exists` returns `true`, so
  `create_repo` is unreachable; latent, not live.
- **evidence.** `direct_git.rs:73-87` (override → unconditional `Ok`) and
  `direct_git.rs:89-96` (`repo_exists` → `Ok(true)`, `create_repo` → loud
  error at `:98-115`) **still hold**. The `github`/`gitverse` half does not:
  `github.rs:173` and `gitverse.rs:141` take `&ValidatedOrg` and contain **no**
  internal `validate_scope` call.
- **match.** PARTIAL — the `direct_git` half and the "latent not live"
  conclusion hold; the "`github`/`gitverse` open with `self.validate_scope`"
  half is gone (the type enforces it now).
- **note.** Describes the **before** state for `github`/`gitverse`; today those
  two no longer call `validate_scope` inside their method bodies — the caller
  mints `ValidatedOrg` once (`orchestrator.rs:236`) and passes it to
  `repo_exists`/`create_repo`/`push_url`. `direct_git` still overrides
  `validate_scope` to a no-op (`direct_git.rs:73`) and `repo_exists` still
  returns `Ok(true)` (`direct_git.rs:89`).
- **verdict.** PENDING

## scope-the-typed-fix

- **claim.** Put the check in the type: a `ValidatedOrg` only `validate_scope`
  can mint; `repo_exists`/`create_repo` take `&ValidatedOrg`; private field,
  only constructor is the scope check; forgetting to validate does not
  compile.
- **evidence.** `creator.rs:76` (`struct ValidatedOrg(String)` — private
  field); `creator.rs:91` (`pub(crate) fn new`); `creator.rs:216-227`
  (`validate_scope` mints via `new`); `creator.rs:177`/`:181`/`:195` (the three
  methods take `&ValidatedOrg`).
- **match.** SUPPORTS
- **note.** SUPPORTS for the stated out-of-crate threat; precision: there are
  two constructors — public `validate_scope` (checks) and `pub(crate) new`
  (does not). For a caller outside `vibe-publish`, `validate_scope` is the only
  path (`new` is `pub(crate)`), so "no other way to obtain the argument" holds
  there. An **in-crate** adapter can still mint without checking —
  `direct_git.rs:86` does exactly that — and the oracle docstring states this
  explicitly (`repo_creator_oracle.rs:72-76`: the type forces the *call*, not
  the *behavior*).
- **verdict.** PENDING

## scope-the-cost

- **claim.** Cost is bounded: three prod impls (`github.rs:147`,
  `gitverse.rs:126`, `direct_git.rs:63`), one cross-crate test mock
  (`vibe-cli/src/commands/workspace/tests.rs:84`), and the orchestrator's call
  sites; the signature change is what forces each through the mint.
- **evidence.** `github.rs:147` (`impl RepoCreator for GithubRepoCreator`);
  `gitverse.rs:126`; `direct_git.rs:63`; `workspace/tests.rs:84` (`impl
  RepoCreator for MockCreator`, with `repo_exists`/`create_repo` taking
  `&ValidatedOrg` at `:89`/`:94`); orchestrator call sites
  `orchestrator.rs:236` (`validate_scope`), `:239` (`repo_exists`), `:268`
  (`create_repo`), `:282` (`push_url`) — all passing `&validated_org`.
- **match.** SUPPORTS
- **note.** none
- **verdict.** PENDING

## scope-coverage-gap

- **claim.** Scope test coverage is uneven: `github.rs` 8 mentions,
  `gitverse.rs` 3, `direct_git.rs` 3 — and `direct_git`'s is
  `assert!(c.validate_scope("anything").is_ok())`, the *absence* of scope; no
  test asserts the obligation itself (a side-effecting method REFUSES an
  out-of-org arg); after the newtype, one table-driven test over the impls for
  the half a type cannot hold.
- **evidence.** `direct_git.rs:164` (`assert!(c.validate_scope("anything").is_ok())`
  inside `validate_scope_is_a_no_op`) — confirms the "absence of scope" test.
  The table-driven refusal test landed as `scoped_adapters_refuse_a_foreign_org`
  (`repo_creator_oracle.rs:84`), parameterised over `github`+`gitverse` with
  `direct` intentionally absent (`:89-91`).
- **match.** PARTIAL — the substance holds (the `direct_git` absence-test, and
  the table-driven refusal test now exists for the half the type can't hold),
  but the 8/3/3 mention counts are pre-build and not reproducible, and the
  table test covers two scoped adapters, not "three implementations".
- **note.** 8/3/3 were census-time "scope" mention counts; today the files
  read differently — `github`/`gitverse` no longer call `validate_scope`
  internally (the type enforces) and all three carry expanded scope commentary,
  so case-insensitive "scope" now appears 14 / 4 / 12 times respectively. The
  "table-driven test over three implementations" landed as
  `scoped_adapters_refuse_a_foreign_org` but over **two** scoped adapters
  (`github`, `gitverse`); `direct_git` is deliberately out (declares no scope).
- **verdict.** PENDING

## digest-asymmetry

- **claim.** The same `sha256:` value is a validated newtype in `vibe-core`
  and a bare `String` in `progress-core`.
- **evidence.** `vibe-core` validated newtype: `content_hash.rs:42`
  (`ContentHash(String)`), `parse()` checks prefix+hex at `:54-68`. `progress-core`
  bare strings: `cache.rs:39` (`pub content_hash: String` on `FileRecord`),
  `doc.rs:82` (`Unit.content_hash`) / `:140` (`ParsedDoc.content_hash`),
  `parse/mod.rs:60` (`pub fn content_hash(s: &str) -> String`), `seal.rs:38`
  (`was: Option<String>`) / `:40` (`now: String` on `SealClaim`).
- **match.** SUPPORTS
- **note.** SUPPORTS; precision — `progress-core`'s digest is **bare hex
  without the `sha256:` prefix** (`parse/mod.rs:60-64` does
  `format!("{:x}", h.finalize())`), whereas `vibe-core::ContentHash` *requires*
  the prefix (`content_hash.rs:55`). So the two are not byte-identical strings;
  the asymmetry (validated newtype vs bare `String`) is real, the concept is
  the same, but "the same `sha256:` value" is approximate.
- **verdict.** PENDING

## digest-the-obvious-fix-is-forbidden

- **claim.** The first design ("`progress-core` imports
  `vibe_core::ContentHash`") was refuted by the separability law carried inline
  in the manifest; PROP-043 §2 makes Progress Control a standalone product
  with no `vibe-*` deps, so two spellings of one digest is the price of a
  deliberate architecture.
- **evidence.** `crates/progress-core/Cargo.toml:14` (`# Separability law
  (PROP-043 §2): NO vibe-* crates here, ever`); the dependency list `:16-26`
  has no `vibe-*` (specmark, anyhow, thiserror, serde, serde_json, toml,
  walkdir, glob, sha2, chrono). `spec/modules/vibe-progress/PROP-043-…md:55`
  (`##SEP-CORE` "core is its own crate … no vibevm subsystem").
- **match.** SUPPORTS
- **note.** none
- **verdict.** PENDING

## digest-what-is-still-wrong

- **claim.** What survives: inside `progress-core` the digest is
  interchangeable with any string; the staleness comparison is `String ==
  String`, as happy comparing a path or batch id; a crate-local newtype
  (`progress_core::Digest`) restores the distinction without importing.
- **evidence.** The comparison is string-level: `baseline/project.rs:259-266`
  — `record.campaign.get("processed_hash").and_then(serde_json::Value::as_str)
  != Some(doc.content_hash.as_str())`. Both sides are effectively `&str`; bare
  `String` at `cache.rs:39`, `doc.rs:82`/`:140`.
- **match.** PARTIAL — the diagnosis (the comparison is string-vs-string,
  interchangeable with any string) is accurate, but the proposed fix (a
  crate-local `progress_core::Digest`) was later **declined** on measurement.
- **note.** SUPPORTS the diagnosis; the prescription ("a crate-local newtype
  restores the distinction") is the plan that `digest-not-built-…` declined —
  at the one site that carried the argument, a one-sided newtype cannot
  type-check the comparison at all (the other side is JSON), so the yield was
  measured at zero.
- **verdict.** PENDING

## digest-the-newtype-does-not-catch-the-likelier-mistake

- **claim.** The newtype guarantee stops where the dangerous case begins: it
  would not catch comparing one file's hash against ANOTHER file's hash (both
  sides `Digest`, swap type-checks); a newtype forbids a hash confused with a
  different *kind* of string, not two same-kind values in wrong roles; the
  wrong-roles mistake is the plausible one here (`processed_hash` vs
  `content_hash` on one record); the tool for the role-swap is a test, not a
  type.
- **evidence.** The two hashes sit on related records as the same kind of
  string: `processed_hash` in the campaign map (`baseline/project.rs:259-263`)
  and `content_hash` on `ParsedDoc`/`FileRecord` (`doc.rs:140`, `cache.rs:39`).
  A `Digest` on both would leave the swap type-checking.
- **match.** SUPPORTS
- **note.** none — this is the fact that demoted the landing; it is
  self-consistent with the code.
- **verdict.** PENDING

## digest-parallel-not-duplicate

- **claim.** Two newtypes over one concept in two separable crates is parity,
  not duplication — same invariant, never same code; the duplication that
  would matter is two *hash calculators*, there is one, each crate wraps its
  own boundary.
- **evidence.** One hash calculator lives in `progress-core`
  (`parse/mod.rs:60-64`, via `sha2`); `vibe-core::ContentHash` does not hash —
  it validates a `sha256:` string (`content_hash.rs:54`). The separability law
  forbids sharing the type (`progress-core/Cargo.toml:14`).
- **match.** PARTIAL — the parity principle is sound, but it argues for a
  *second* newtype (`progress_core::Digest`) that was never built.
- **note.** SUPPORTS the parity argument; today the tree has **one** newtype
  (`vibe-core::ContentHash`) and **one** bare `String` (`progress-core`), not
  "two newtypes over one concept" — the second was declined
  (`digest-not-built-…`), so "two newtypes" is the declined plan, not the tree.
- **verdict.** PENDING

## digest-not-built-and-the-reason-is-the-comparison-itself

- **claim.** NOT BUILT: the staleness read (`baseline/project.rs`) takes
  `processed_hash` as untyped `serde_json::Value`, `and_then(as_str)`, compares
  that `&str` against `doc.content_hash`; a newtype on the `ParsedDoc` side
  cannot type-check it (the other side is JSON); typing both puts `Digest` on
  both sides = the role-swap case; yield is **zero** at the one site;
  remaining benefit (hash vs path/batch-id) has no census instance; cost ~60
  sites in `progress-core` + ~29 in `vibe-cli` + a serde form that must keep
  `cache.json` byte-identical. Recorded as a decision, not deferred.
- **evidence.** `baseline/project.rs:259-266` — exactly as described:
  `record.campaign.get("processed_hash").and_then(serde_json::Value::as_str) !=
  Some(doc.content_hash.as_str())`. The `processed_hash` side is untyped JSON
  → `&str`; a one-sided `ParsedDoc` newtype cannot type-check it, and typing
  both is the role-swap case. `progress_core::Digest` does not exist in the
  tree (no such type) → not built.
- **match.** SUPPORTS
- **note.** SUPPORTS; the ~60 / ~29 site counts are the design's cost estimate
  for a build that was declined — I did not recount them (they were the price
  of work not done, so they remain estimates).
- **verdict.** PENDING

## digest-what-the-site-actually-owed

- **claim.** Reading the comparison for the newtype's sake found a five-line
  defect: the read was `is_some_and(|h| h != content_hash)`, so a record
  carrying verdicts and **no** `processed_hash` came back false — projected as
  fresh; the sibling `verified_at` guard three lines above does the opposite;
  absence is now not a match, with a test pinning it reported stale and not
  undated.
- **evidence.** `baseline/project.rs:259-266` — the read is now
  `…and_then(as_str) != Some(doc.content_hash.as_str())`; absence → `None !=
  Some(…)` → `true` → `out.stale.push(...)`. The comment at `:252-258` records
  it "used to be `is_some_and`, which made the absence mean fresh". The
  sibling `verified_at` guard at `:243-251` reports absence → `undated`. Test:
  `baseline/project/tests.rs:334`
  (`a_record_with_no_processed_hash_is_reported_rather_than_assumed_fresh`)
  asserts `p.stale == ["a.md"]` and `p.undated.is_empty()` (`:348`, `:349`).
- **match.** SUPPORTS
- **note.** none
- **verdict.** PENDING

## wire-the-count

- **claim.** Of the identity newtypes that validate, exactly one validates on
  the wire: `Group` is `serde(try_from="String", into="String")`
  (`package_ref.rs:106`); `ContentHash`, `PackageName`, `CapabilityNamespace`,
  `CapabilityName` are `serde(transparent)` and accept any string off the wire,
  their `parse()` running only when a caller calls it.
- **evidence.** `Group` `try_from` at `package_ref.rs:106` ✓. **But today all
  four formerly-transparent newtypes also validate on load**:
  `ContentHash` `try_from` at `content_hash.rs:41`; `PackageName` at
  `package_ref.rs:201`; `CapabilityNamespace` at `capability_ref.rs:51`;
  `CapabilityName` at `capability_ref.rs:67`.
- **match.** PARTIAL — describes the **before** state of the wire landing
  (order-list #2); today "exactly one" is false.
- **note.** Describes state before the wire landing (BUILT); today all five
  identity newtypes validate on the wire. `content_hash.rs:22-25` records the
  change verbatim ("It was `serde(transparent)` until 2026-08-05, on a reason
  that only ever justified the wire SHAPE").
- **verdict.** PENDING

## wire-the-docblock-does-not-force-it

- **claim.** The recorded reason for `transparent` justifies the wire SHAPE
  and reads as if it justified the missing check: `content_hash.rs:20-22` says
  `transparent` keeps the bare string; but `try_from="String", into="String"`
  emits the same bare string (`Group` demonstrates); the constraint survives
  the change; what changes is only whether a malformed value is noticed at the
  boundary.
- **evidence.** `content_hash.rs:20-25` — the docblock now reads
  `serde(try_from="String", into="String")` keeps the bare string **and**
  states "It was `serde(transparent)` until 2026-08-05 … `transparent` and
  `try_from`/`into` emit the same bare string, and only one of them notices a
  malformed value arriving." `Group` demonstrates the same bare string
  (`package_ref.rs:106`).
- **match.** SUPPORTS
- **note.** SUPPORTS; the landing rewrote this docblock to state the argument
  itself, and the tree demonstrates it — `ContentHash` emits the same bare
  string via `try_from` (`content_hash.rs:41`) that `Group` does
  (`package_ref.rs:106`).
- **verdict.** PENDING

## wire-the-landing

- **claim.** Four attributes — the four newtypes adopt `Group`'s spelling; if
  fixtures/lockfiles then fail to load, that is a malformed value the tree was
  already carrying silently (the outcome to want, recorded not smoothed);
  `from_validated` stays (the in-process trusted path from `vibe-index`'s
  hasher, not a wire path).
- **evidence.** All four now `try_from="String", into="String"`:
  `content_hash.rs:41`, `package_ref.rs:201`, `capability_ref.rs:51`,
  `capability_ref.rs:67`. `from_validated` retained: `content_hash.rs:72`,
  `package_ref.rs:218` (`PackageName::from_validated`), `capability_ref.rs:81`
  (in the `kebab_newtype!` macro).
- **match.** SUPPORTS
- **note.** SUPPORTS; order-list #2 records that the landing surfaced "five
  values in the tree turned out not to be hashes" — I did not independently
  re-locate those five.
- **verdict.** PENDING

## builder-today

- **claim.** The tree's only real builder enforces its required fields at
  runtime: `ActionBuilder` (`action.rs:333`) keeps `name`, `description`,
  `invoke` as `Option<…>` (`:335`, `:336`, `:341`) and turns their absence into
  `MissingName` / `EmptyPresentation` in `build()` (`:449-460`); the obligation
  is a `Result`, callers `.unwrap()` or propagate.
- **evidence.** Today: `ActionBuilder` is at `action.rs:312` (not `:333`);
  `name`/`description` are `Msg` (not `Option`) at `:314-315`; `invoke` is
  `build()`'s argument (`:431` `pub fn build<F>(self, body: F)`);
  `ActionBuildError` has **only** `EmptyPresentation` (`:290-304`) — no
  `MissingName`/`MissingDescription`/`MissingInvoke`; `build()` checks only
  emptiness (`:437-448`), not presence.
- **match.** PARTIAL — describes the **before** state of the builder landing
  (order-list #3); today presence is in the construction signature.
- **note.** Describes state before the builder landing (BUILT); today
  `Action::builder(addr, name_en, description_en)` (`action.rs:198-204`)
  requires name+description, `build<F>(self, body)` (`:431`) requires the
  invoke body, `name`/`description` are `Msg` not `Option`, and the
  `Missing*` variants are gone. The struct also moved `:333 → :312`.
- **verdict.** PENDING

## builder-the-landing

- **claim.** The three obligations move into the type, encoding **not**
  typestate; what stays a runtime error is *emptiness* (a supplied-but-blank
  string is a value check, not a presence check, and no type sees it); saying
  which half moves is the decision; claiming the whole error goes away would
  be false.
- **evidence.** `Action::builder(addr, name_en, description_en)` at
  `action.rs:198-204` (name+description required at construction);
  `build<F>(self, body: F)` at `action.rs:431` (invoke required at `build`);
  `build()` still runtime-checks emptiness → `EmptyPresentation`
  (`action.rs:437-448`). The `ActionBuildError` docstring at `:282-285`:
  "Presence … is enforced by the construction signature … the only runtime
  check left is the value check."
- **match.** SUPPORTS
- **note.** none
- **verdict.** PENDING

## builder-typestate-is-the-wrong-tool-for-presence

- **claim.** Typestate answers ordering; this is presence, and presence has a
  cheaper encoding — three obligations would need three type params and
  `Set`/`Unset` markers making all setters generic and rebuilding a ten-field
  struct; the constructor gives presence free; the shape: all seventeen chains
  supply name/description through the `_en` pair and end in `invoke`, so the
  two presentation obligations become constructor args and the third becomes
  `build`'s arg — `action(addr,"Name","Description").icon().params().build(|ctx|
  …)`; the `Msg`-explicit setters stay for override.
- **evidence.** The chosen shape landed exactly: `Action::builder(addr, name_en,
  description_en)` (`action.rs:198`), `build<F>(self, body)` (`action.rs:431`),
  **no** type parameters on `ActionBuilder` (`action.rs:312`); `_en` override
  setters retained (`action.rs:354` `name_en`, `:370` `description_en`, with
  the note at `:345-352` "no callers as of 2026-08-05"). "Seventeen chains"
  confirmed today: `rg "\.build\(\|"` → 15 in `vibe-actions` (`action.rs` 5,
  `aiui.rs` 4, `invoke.rs` 3, `registry.rs` 2, `gate.rs` 1) + 2 in `vibe-cli`
  = **17**.
- **match.** SUPPORTS
- **note.** SUPPORTS; "eleven setters" in the rationale is the pre-build count
  (the typestate cost it is arguing against); today `ActionBuilder` has 10
  `#[must_use]` setters (`action.rs:354-425`) — `invoke` moved into `build`'s
  signature.
- **verdict.** PENDING

## builder-the-blast-radius

- **claim.** Measured, smaller than the idiom's reputation: `ActionBuilder`
  appears 10× and `ActionBuildError` 12× across `crates/`; the `.build()` call
  sites are all inside `vibe-actions` (its tests, `aiui.rs`, `gate.rs`,
  `invoke.rs`, `registry.rs`); the private constructor (`action.rs:347`)
  already fixes the address before chaining.
- **evidence.** Re-measured today: `rg -c "ActionBuilder" crates/` → **12**
  (design said 10); `rg -c "ActionBuildError" crates/` → **7** (design said
  12); `ActionBuilder.build()` call sites (`.build(|`) → **15 inside
  `vibe-actions`** + **2 in `vibe-cli`** (`commands/tree/tui/search/catalogue.rs:172`,
  `commands/prefs/tui/catalogue.rs:213`). Private constructor `fn new` today
  at `action.rs:325` (design said `:347`).
- **match.** PARTIAL — the "`.build()` all inside `vibe-actions`" claim is
  **false** today (2 are in `vibe-cli` catalogue files), exactly as the packet
  pre-noted; the `ActionBuilder` and `ActionBuildError` counts also moved.
- **note.** Re-measured today — `ActionBuilder` 12× (design 10),
  `ActionBuildError` 7× (design 12; the landing removed
  `MissingName`/`MissingDescription`/`MissingInvoke`, leaving only
  `EmptyPresentation`, `action.rs:290`), and the `.build()` call sites are
  15 in `vibe-actions` + 2 in `vibe-cli` — so "all inside `vibe-actions`" is
  wrong. (The other 3 bare `.build(` in `vibe-cli` — `vvm/install.rs:90`,
  `aiui/cdp.rs:29`, `aiui/control.rs:269` — are not `ActionBuilder`, they are
  other builders.) Private `fn new` is at `action.rs:325` today.
- **verdict.** PENDING

## builder-must-use-stays-as-is

- **claim.** `#[must_use]`'s distribution is already correct and gets no
  landing: it pays on `-> Self` setter chains where dropping the return is the
  bug — exactly where the 146 sit (the 11 `ActionBuilder` setters, the
  `KeyMeta` setters, the TUI widgets); the seams that lack it return `Result`
  or a value whose drop is legitimate; 146 concentrated in one crate looks
  like imbalance and is correct distribution.
- **evidence.** Today `#[must_use]` = **145** (vibe-cli 119 / vibe-actions 17
  / vibe-settings 9); concentrated on `-> Self` setter chains: `action.rs`
  setters (`:354-425`, 10 today), `KeyMeta` setters (`vibe-settings/src/schema/types.rs`),
  TUI widgets (vibe-cli `commands/{tree,prefs}/tui/**`). 119/145 = 82 %.
- **match.** PARTIAL — the distribution thesis holds (must_use sits on `-> Self`
  setter chains, absent on traits/wire/newtypes), but the count is 145 not 146
  and the `ActionBuilder` setters are 10 not 11.
- **note.** SUPPORTS the distribution thesis; the count drifted 146→145
  because the builder landing removed the `invoke` setter's `#[must_use]`
  (`action.rs` 11→10). 119 of 145 (82 %) still concentrate in `vibe-cli`'s two
  TUI trees — the "looks like imbalance, is correct" reading still fits.
- **verdict.** PENDING

## watcher-the-measurement

- **claim.** One pub trait in the perimeter has zero production
  implementations: `Watcher` (`vibe-settings/src/events/mod.rs:432`) carries
  `specmark::spec(implements = "…/PROP-040#file-watch")` on the trait and its
  method, and the only implementations are a test `Noop`
  (`events/tests.rs:250`) and a doc-comment example.
- **evidence.** `Watcher` trait today at `events/mod.rs:444` (design said
  `:432`); `specmark::spec(implements = "…#file-watch")` on the trait at
  `:441-443` and on `watch` at `:449-451`; `rg "impl Watcher for" crates/` →
  only `Noop` at `events/tests.rs:250` (inside test fn `:248`) plus the
  doc-comment `MockWatcher` text at `mod.rs:419`. No prod impl.
- **match.** SUPPORTS
- **note.** SUPPORTS; the trait is at `:444` today, not `:432` — the Watcher
  landing's own annotation paragraph ("**Specified, not built (→ B-061),
  measured 2026-08-05.**", `mod.rs:392-403`) sits above the trait and pushed it
  down 12 lines. The doc-comment example is at `mod.rs:418-440` (MockWatcher).
- **verdict.** PENDING

## watcher-the-landing

- **claim.** This is "Specified, not built"; the trait is the declared shape
  of a promised capability, so the landing is an explicit annotation at the
  seam plus a backlog row carrying the build — not a deletion, not a silent
  leave-as-is.
- **evidence.** Annotation landed: `events/mod.rs:392` "**Specified, not built
  (→ B-061), measured 2026-08-05.**" with two paragraphs (`:392-403`)
  explaining the missing prod impl and the `implements`-edge false-coverage.
  Backlog row: `BACKLOG.md:111` `### B-061 … Watcher`, `filed by` "босс-дизайн
  B-040 по цензусу швов, волна Г, 2026-08-05" (`:119`).
- **match.** SUPPORTS
- **note.** none
- **verdict.** PENDING

## sealing-the-count

- **claim.** None of the 24 pub traits is sealed, and the census found no
  sealed-pattern construct anywhere — no private supertrait, no
  module-private gate; the word "seal" in this tree is the domain concept of
  verdict sealing.
- **evidence.** 24 `pub trait` (measured); `rg ": Sealed|sealed::|seal::Sealed|mod seal|private::Sealed"`
  → only `pub mod seal;` at `progress-core/src/lib.rs:36` and `mod seal;` at
  `vibe-cli/src/commands/progress.rs:45` — both the **domain** verdict-sealing
  module (`progress-core/src/seal.rs:1` "Sealing — recording that a file's
  verdicts hold…"), not trait sealing.
- **match.** SUPPORTS
- **note.** none
- **verdict.** PENDING

## sealing-would-break-the-architecture

- **claim.** The seams are extension points by construction and the extensions
  live in another crate: `vibe-cli` implements `SearchProvider` 5×, plus
  `PlanObserver`, `VendorObserver`, `RedirectSyncObserver`, `InstallSource`,
  `EvidenceProvider`; `vibe-publish`'s test `RepoCreator` mock lives there too;
  sealing deletes that wiring.
- **evidence.** Census Q1 table: `SearchProvider` ×5 cross-crate `vibe-cli`
  (`g1-b040-seams-census.md:67`); `PlanObserver` CtxObserver (`:75`);
  `VendorObserver` CliVendorObserver (`:71`); `RedirectSyncObserver`
  CliRedirectSyncObserver (`:79`); `InstallSource` InstallResolver (`:74`);
  `EvidenceProvider` SpecmapEvidence (`:70`). The test `RepoCreator` mock:
  `vibe-cli/src/commands/workspace/tests.rs:84` (`impl RepoCreator for
  MockCreator`).
- **match.** SUPPORTS
- **note.** SUPPORTS; the cross-crate impls are catalogued in the census Q1
  table (all confirmed), and the test `RepoCreator` mock is at
  `workspace/tests.rs:84` (now taking `&ValidatedOrg` at `:89`/`:94`).
- **verdict.** PENDING

## sealing-buys-nothing-in-a-workspace

- **claim.** A sealed trait buys non-breaking evolution against **external**
  implementors; these crates are workspace-internal and published nowhere, so
  adding a method is a compile error the same commit fixes; adopting would
  trade a real capability for a benefit measured at zero.
- **evidence.** The crates are workspace members under `crates/` (single
  workspace, 19 crates); none is consumed outside the workspace. The extension
  impls live across workspace crates (see sealing-would-break-the-architecture).
- **match.** SUPPORTS
- **note.** SUPPORTS on the structural premise (workspace-internal membership
  verifiable — all touched crates are `crates/` members); "published nowhere"
  rests on the workspace's `publish = false` posture
  (`progress-core/Cargo.toml:8` `publish.workspace = true`, inherited).
- **verdict.** PENDING

## sealing-the-one-tempting-case-answered-better

- **claim.** The single seam with a written implementor obligation is
  `RepoCreator`, and §2 serves it better than sealing: sealing forbids a
  foreign implementation but would not make an in-crate implementation call
  `validate_scope`; the newtype forbids the actual mistake and keeps the test
  mock legal; where the obligation is "call this", the fix is an argument only
  that call can produce.
- **evidence.** `RepoCreator` carries the doc-comment obligation
  (`creator.rs:103-113`, `:206-215`); the newtype gate (`ValidatedOrg`) landed
  and sealing was not adopted; the test mock stays legal (`workspace/tests.rs:84`,
  taking `&ValidatedOrg`).
- **match.** SUPPORTS
- **note.** SUPPORTS; precision — "an argument only that call can produce"
  holds for **out-of-crate** callers; an in-crate adapter can still override
  `validate_scope` to mint unconditionally (`direct_git.rs:73-86`), and the
  oracle docstring says so (`repo_creator_oracle.rs:72-76`). The newtype forces
  the call; the oracle table-test (`:84`) guards the behavior.
- **verdict.** PENDING

## sealing-recorded-not-postponed

- **claim.** Recorded as a decision, not deferred as work — the
  "задокументировать где сознательно нет" half of the owner's ruling;
  re-judging `##SCAFFOLD-B-TYPED-BUILDERS` against practice: the guide's claim
  is right about builders and typestate and does not fit a workspace-internal
  trait surface.
- **evidence.** The decision is recorded in §7 of the design
  (`spec/design/typed-seams.md:71-81`); the GUIDE scaffold-B section is
  `scaffold-b-typed-builders` in
  `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md`.
- **match.** NO-CODE — a recording/decision fact; its subject is documentation
  (the §7 decision and the GUIDE re-judgment), not a code line.
- **note.** NO-CODE: the claim is that a *decision* was recorded (not deferred
  as work); nothing in code asserts or falsifies "recorded vs deferred" — that
  lives in the design doc's own §7.
- **verdict.** PENDING

## order-list

- **claim.** Five landings ordered by payoff over cost, each landable and green
  alone, the order holding except where measurement moved it: (1) `ValidatedOrg`
  + conformance test — security-relevant, the only one closing an unchecked
  obligation — **BUILT**; (2) the four `serde(transparent)` newtypes →
  `try_from`/`into` — whatever breaks is a find, five values weren't hashes —
  **BUILT**; (3) the builder's three presence obligations into the signature —
  **BUILT**; (4) `progress_core::Digest` — demoted, then **declined on
  measurement**, the five-line defect built instead; (5) `Watcher` annotated
  "Specified, not built" + backlog row — **BUILT**.
- **evidence.** (1) `ValidatedOrg` `creator.rs:76` + oracle
  `repo_creator_oracle.rs:84` — BUILT; (2) `try_from` on `content_hash.rs:41`,
  `package_ref.rs:201`, `capability_ref.rs:51`, `capability_ref.rs:67` — BUILT;
  (3) `Action::builder` `action.rs:198` + `build(body)` `:431` — BUILT; (4)
  `progress_core::Digest` absent (declined); the five-line fix at
  `baseline/project.rs:259-266` + test `baseline/project/tests.rs:334` — BUILT;
  (5) annotation `events/mod.rs:392` + backlog `BACKLOG.md:111` (B-061) — BUILT.
- **match.** SUPPORTS
- **note.** SUPPORTS; the "five values turned out not to be hashes" (landing 2)
  is the design's own record — I did not independently re-locate those five.
- **verdict.** PENDING

## order-gates

- **claim.** Every landing carries the same gate set — `cargo check` + `cargo
  test` + `cargo clippy --all-targets -D warnings` for its crate; every
  touched/created `.rs` under the 600-line budget after `cargo fmt`; the
  boss's panel is the real gate, and `cargo xtask sync-engines` does not apply
  — none of these crates is a vendored engine, verified against
  `sync-engines.toml`, whose sources are the `core-ai-native-*` crates only.
- **evidence.** `sync-engines.toml`: every `[[sync]]` `source_root` is under
  `packages/org.vibevm.ai-native/`, and every `crates` entry is a
  `core-ai-native-*` / `rust-ai-native-*` / `typescript-ai-native-*` /
  `go-ai-native-*` crate (`:14-20`, `:34-41`, `:48-56`, `:61-68`, `:74-85`,
  `:97-105`, `:110-121`). None of the five touched crates (`vibe-publish`,
  `vibe-core`, `vibe-actions`, `vibe-settings`, `progress-core`) appears as a
  source or target.
- **match.** SUPPORTS
- **note.** SUPPORTS (point C): the conclusion follows because every
  `[[sync]].source_root` and `.crates` entry names a crate under
  `packages/org.vibevm.ai-native/` (the neutral engines
  `core-ai-native-{conform,specmap,specmark,specmark-grammar,mcp}` plus the
  three language stacks' `*-ai-native-*` toolchain crates); the five touched
  host crates live under `crates/` and appear nowhere in `sync-engines.toml`,
  so `cargo xtask sync-engines` has nothing to sync for them.
- **verdict.** PENDING

## order-what-this-does-not-do

- **claim.** What this build deliberately leaves alone: the
  `RelPath`/`PathBuf`/`String` path-and-URL asymmetry (a newtype there touches
  every seam constructor and buys distinction, not safety); the ~133 serde
  types that derive `Deserialize` plainly (structural fields, post-load
  `validate()` where semantics matter); and the `#[must_use]` distribution.
- **evidence.** `RelPath` exists (`rel_path.rs` per census `:35`) but
  neighbouring seams still carry `PathBuf`/`String` — `orchestrator.rs:59`
  `with_defaults(source_dir: PathBuf, org_url: String)`. `#[must_use]` got no
  landing (still 145, concentrated as before). No path/URL newtype was added.
- **match.** SUPPORTS
- **note.** SUPPORTS; the "~133 serde types" is the census arithmetic (~140 −
  7 validating); today the validating count is 11 (not 7), so the plain-derive
  remainder shrank by the four the wire landing converted — but none of these
  three areas (path asymmetry, plain serde, must_use) received a landing.
- **verdict.** PENDING
