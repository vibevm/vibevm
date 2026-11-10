# Sys-prompt campaign log

Append-only. Newest entry last. One entry per phase step, spike or decision
that a later session would otherwise have to re-derive.

## 2026-11-10 — campaign planned

`PLAN.md` and `FORMAT-CHANGES.md` written against `main` at `e2508fc1d`;
`spec://org.vibevm.core/vibevm/common/PROP-076#root` written from the owner's
research proposal of 2026-11-07
(`campaigns/next-semantic-transfer-2026-11/architecture/SYSTEM-PROMPT-SPEC.md`,
`system-prompt.json`) and its three reviews into a settled design with every
open choice resolved. **Nothing implemented.** No product code was touched;
`PROP-076` is the only new file under `vibevm/vibespecs/`.

**The design's one real contribution over the research: four bands, not two.**
The research framed the choice as "stable prefix versus live suffix" and then
had to argue away the obvious objection — that the eight INDEX rows of this
repository are all unconditional, so leaving 57,894 bytes outside the prefix
looks like leaving cache on the table. The resolution is that the unconditional
continuation is **byte-stable without being frozen**: its bytes change only
when those files change, exactly as the prefix's do, so a runner that places a
second cache breakpoint at its end caches it too. `vibe prompt prepare`
therefore returns bands P (prefix), U (unconditional), G (gated) and leaves T
(task) to the caller — **separately, never concatenated** — and the boundary
costs nothing in cache terms while `kind = "static"` keeps meaning read timing
rather than linkage. That is `PROP-076#SYS-BANDS` and `#SYS-CONTINUATION-CACHEABLE`,
and it is why the plan does **not** offer a "freeze the whole INDEX" profile.

**The second decision worth not re-deriving: the segment order is forced twice
over, and both forces agree.** Template → entrypoint → lane is PROP-009
`#SESSION-START-ORDER` with the user's own frame prepended, so no authored
instruction moves relative to another. It is *also* cache-optimal, because a
provider cache matches a prefix and so an edit to an early segment invalidates
every later one: the lane changes most often (any `vibe install` that moves the
resolution) and the template least often (a human edits it deliberately), so
the lane belongs last. The reverse order would make every lane change
invalidate the template and the entrypoint as well. Recorded in
`#SYS-BAND-ORDER` precisely so a later "optimisation" cannot quietly flip it.

**The third: the sidecar carries no digest for any continuation row.** A hash
of `90-user.xml` in a committed generated file would dirty that file on every
edit of a user boot snippet and demand a rebuild — destroying the one property
dynamic linking exists for. So the committed witness covers the prefix inputs
plus the *plan* (its order, paths, conditions, and the digest of `INDEX.md`'s
bytes that fixes them), and the continuation's digests are request-local
observations in `prepare`'s response. The user-visible consequence is the pair
of commands in the plan's predictions: editing `90-user.xml` leaves
`vibe prompt check` at `equal`; editing one byte of `AGENTS.md` makes it
`stale` naming the entrypoint.

### Facts established while planning, which `PLAN.md` §2 records in full

- **The self-inclusion hazard is real at HEAD, not hypothetical.**
  `install/bootgen/own_boot.rs:167` `legacy_entries` admits every immediate
  boot-directory child that `vibe_specdoc::is_spec_source` accepts, and that
  predicate is `Some("md" | "xml")` **by extension alone**
  (`crates/vibe-specdoc/src/load.rs:76-81`), excluding only the three generated
  names. Both proposed `.md` paths would be enrolled today. Spike S3 exists to
  demonstrate it, including the recursive-growth case on a second run.
- **The boot transaction's production-role set is closed and validated**, so
  adding two outputs is a real change to that owner and nothing about a new
  filename is transactionally covered by default:
  `boot_artifacts/transaction/journal.rs:21-23` names the three targets,
  `:62-86` validates them, `:75-78` requires the selected/stale `STATIC` pair
  exactly, and `transaction.rs:259-282` requires one shared parent and
  distinctness. Spike S2 is sized at finding out whether the rollback path
  (`boot_artifacts/inverse.rs`) needs an arm per role; if it does, P4 goes M → L.
- **The durable record is not called `Transaction`.** `journal.rs:7-10` aliases
  the wire type `BootArtifactTransaction` as `Journal`, with `JournalEntry`,
  `RecordedState` and `JournalMode`; the write input is `ArtifactWrite`
  (`transaction.rs:61-67`).
- **There are three independent lane prepare-and-compile sites, not one** —
  `boot_artifacts/compile.rs:86`, `boot_artifacts/native_managed.rs:232` and
  `boot_artifacts/native_managed/explicit.rs:22`, each with its own
  `BootSectionSource`, its own `build_with_providers_bound`, its own plan and
  its own `entries.is_empty() → None` gate. The research assumed one. Spike S4
  decides whether v1 supports one with named refusals for the others or must
  handle all three.
- **`analyze_node_lane` is provider-free and effect-free; its bound/native
  siblings are not.** `install/bootgen/analyze.rs:80` takes no provider and
  ends in an in-memory compile at `:216`; `:270`, `:554` and `:415` each accept
  an `OwnerNativeCompileProvider` (mandatorily at `:415`) and hand control to a
  caller-supplied trait that may spawn a process. So "we called analyse" proves
  nothing about read-only-ness — naming the exact function does. That is why
  `PROP-076#SYS-READ-ONLY-CHECK` is written per function.
- **The host project's band G is empty.** All eight `INDEX.md` rows are
  `kind = "static"` with no `when`, so the OS matrix of P7 must be proved on
  purpose-built fixtures and cannot be proved on this repository.
- **Line endings are already pinned**, which the determinism claim silently
  depends on: `.gitattributes:7` is `* text=auto eol=lf`, and `core.autocrlf`
  is unset on this machine. A project without that attribute would get a prefix
  that differs by checkout.
- **Measured sizes, the reason the feature exists.** `STATIC.xml` 251,817
  bytes, `AGENTS.md` 26,121, the eight INDEX files 57,894 — 335,832 bytes of
  boot material, 75% of it the one compiled lane and 92% of it the prefix.
- **`SYSTEM` and `prompt` are unoccupied names.** `grep -rn 'SYSTEM'` and
  `grep -rni 'prompt'` over `boot_artifacts/` and `install/bootgen/` are empty,
  and the CLI's `Command` enum has no `Prompt` variant, so
  `vibe prompt build|check|prepare` is free.
- **Only one registered format touches boot today**,
  `[format.boot-artifact-transaction]` (`formats/REGISTRY.toml:1202`); the
  generated lanes themselves are unregistered spec documents. Protocol tags
  like `vibe-app-env/1` have no central registry — a constant at the emit site
  plus an untyped `protocol` member in the schema.

### Seven wrong paths and three wrong behavioural claims in the research

Recorded in full as `PROP-076#SYS-BASELINE-CORRECTIONS`; listed here so a
session reading the 2026-11-07 documents knows not to trust their file
references. Wrong paths, all under `crates/vibe-workspace/src/`: `own_boot.rs`
is `install/bootgen/own_boot.rs`; `conditions.rs` is
`install/bootgen/conditions.rs`; `publish_node_boot` is in
`install/bootgen/publication.rs:13`, not `boot_artifacts/publication.rs`; the
unit fingerprint is `boot/hybrid/fingerprint.rs`, and
`install/bootgen/fingerprint.rs` does not exist; owner-plan digest frames are
`install/bootgen/owner_plans.rs:15`; static-zone membership is `boot/hybrid.rs`
with hoisting in `boot/hybrid/hoist.rs`; the node-dependency projection is
`install/bootgen/node_dependency.rs:14`. Wrong behaviour: `legacy_entries` does
not exclude "only" the three generated names (it also requires
`is_spec_source` and hard-errors on a name collision); there is not one lane
compiler seam but three; and "StaticTransitive forces descendants **except
residual OS gates**" is false at HEAD — `boot/hybrid.rs:244-288` `descend`
never consults `when` and says so at `:263-265`, while four doc comments
(`boot/hybrid.rs:17-20`, `:118-121`, `boot/hybrid/fingerprint.rs:10-11`,
`boot/hybrid/hoist.rs:16`) still claim the old behaviour and are stale. The
research's own refusal to trust a source comment was right for a different
case too: `boot.rs:352-354` claims the unresolved-link fallback is `static`,
while `unwrap_or_default()` plus `#[default]` on `LinkType::Dynamic`
(`crates/vibe-core/src/manifest/package.rs:405`) makes it **Dynamic**.

### How `PROP-076` was validated, and a defect found in `PROP-075`

There is **no spec-XML validation gate** in this repository: `vibe check` does
not parse the dialect, and no `cargo xtask` subcommand or test walks
`vibevm/vibespecs/**`. The authoritative validator is the PROP-045 projector
`vibe_specdoc::project_spec_text` (`crates/vibe-specdoc/src/load.rs:140`),
reached per file by

```sh
./target/debug/vibe refactor convert-source --from xml --to md --dry-run \
  vibevm/vibespecs/common/PROP-076-system-prompt-prefix.xml
```

which **always exits 0** — the verdict is the `dry-run …` line, and the
prebuilt binary at `target/debug/vibe` was used so the concurrently-edited
workspace did not have to be rebuilt. `PROP-076` reports **`dry-run
byte-stable`**: it parses, and the reverse projection reproduces the source
bytes exactly. Two findings along the way, both worth keeping:

1. **`PROP-075` does not parse.** The same command on
   `vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml`
   reports `source parse failed: line 210: fact id `@fact:changelog` is defined
   twice in this file: lines 208 and 210`. Its section 7 carries two
   `<changelog>` facts with the same id — the second entry appears to have been
   appended without removing the first. **Filed for the app-versions campaign
   to fix; not touched here**, because `campaigns/app-versions-2026-11/` and its
   spec are another campaign's files and workers are live in them. It is also
   the concrete argument for adding the dialect check that the app-versions log
   already filed as a candidate: a spec document can sit unparseable in the
   tree for a day and nothing notices.
2. **Blank lines between elements cost `byte-stable`.** A document with blank
   lines inside a section round-trips as `ir-stable-loss` (the reverse
   projection drops them) rather than `byte-stable`; `PROP-069` is in that state
   today. `PROP-076` was written without blank lines for that reason.

### Open to the owner

`PROP-076#SYS-OWNER-QUESTIONS` Q1–Q6, each with the default the design takes.
Q1 (the unconditional band as a cacheable continuation rather than baked in)
and Q3 (a portable prefix with gated continuation rather than a target-bound
one) are the band model itself, so a rejection after P3 lands is rework rather
than a tweak — they want an answer before P3 starts. Q4 (the entrypoint
verbatim with one in-place substitution, rather than filtered) shapes P2 and P3
the same way, and spike S1 is designed to give the owner evidence for it. Q2
(override authority in the API instruction role) blocks P6 and nothing earlier.
Q6 (both artifacts committed) shapes P4. Q5 (no measured saving required before
close) blocks nothing.
