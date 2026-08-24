# DRIFT-024 — the scope stops observing what it must not mark {#root}

<status stage="impl" state="plan" ref="DRIFT-024"/>

**Status:** ready — owner said do it, 2026-07-26
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** progress-core (scope) + the campaign's own config
**Unit-stability check:** one PROP-043 anchor is *added* by the reviewer after
this lands (§5). No existing anchor moves.

## 1. Goal {#goal}

`vibe progress check --exhaustive` can reach green honestly, because the two
classes of file that must never carry markup are no longer observed.

## 2. Contract {#contract}

> @fact:ZONE-EXCLUDED Excluded from markup scope, from packaging, and from
> registries — always.
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#campaign-zone`

Findings realised: **F-070** (33 `LICENSE.xml`, 264 paragraphs of verbatim
UPL text) and **F-071** (three `spec/cards/INDEX.md`, each declaring itself a
derived index whose *"hand edits are a defect"*).

## 3. Current state {#current}

Measured 2026-07-26 — do not re-discover, but contradict me if a number is wrong:

- `DEFAULT_EXCLUDES` (`crates/progress-core/src/scope.rs:11-20`) is
  `["vibedeps", ".vibe", "refs", "fixtures", "campaigns", "target",
  "node_modules", "vendor"]`, applied by `is_excluded` as a **path-component**
  match (`:95-100`) — so it can express a directory name, and it can express a
  bare filename only by accident, since a basename is also a component.
- **`check --exhaustive` reports 8 992 unmarked paragraphs.** 264 of them are
  in 33 `LICENSE.xml` files — the same UPL-1.0 text once per package version
  slot.
- The three `spec/cards/INDEX.md` (rust v0.7.0, typescript v0.6.0, go v0.1.0)
  say of themselves: *"Generated/maintained as a derived index (A2/R-030);
  hand edits are a defect."* Markup written there dies at the next
  regeneration — the exact reason wave 1 kept `spec/boot/STATIC.xml` and
  `INDEX.md` out of scope.
- `progress.toml` **cannot express either exclusion today**: PROP-043 §4 is
  include-only by design, and no include glob can say "everything under
  `packages/**` except these three files".
- The audit that found them also proved the rest clean: across all 344
  observed files there are two top-level roots and no path segment named
  `target`, `vibedeps`, `node_modules`, `vendor`, `.vibe`, `refs`,
  `campaigns`, `fixtures`, `dist`, `build`, `generated` or `.git`.

## 4. Required behavior {#behavior}

**Two mechanisms, because the two classes are genuinely different. The split
is decided — do not collapse them into one.**

### 4.1 `LICENSE.xml` — project-neutral, so it goes in code {#license}

Add a **file-level** default exclusion (a new `DEFAULT_EXCLUDE_FILES`, matched
against the **basename only**) containing exactly `LICENSE.xml`. Always-on, like
`DEFAULT_EXCLUDES`, and applied even under an explicit include.

Justify it in the code comment on the same footing as the existing entries: a
licence is verbatim third-party text that the observing project neither
authored nor is the source of truth for — the same reason `refs` is excluded.
`progress-core` is a project-neutral engine (PROP-043 §5) and this rule is
neutral: every project has licence files and none of them are its contracts.

### 4.2 The three derived indexes — project-specific, so they go in config {#exclude}

Add an optional `exclude` list of glob patterns to `ScopeConfig`, applied
**after** the include globs. Then set it in `progress.toml` to
`["packages/**/spec/cards/INDEX.md"]` with a comment saying why.

**This is a deliberate amendment to §4's include-only design, and the task
owns saying so.** The design's purpose is that nothing is observed by accident;
an enumerated exclude list serves that purpose exactly as well as an enumerated
include list, because both are explicit and reviewable. What it must NOT become
is a wildcard escape hatch — so:

- an `exclude` pattern that matches **nothing** is a **warning naming the
  pattern**, not silence. A stale exclusion that quietly protects nothing is
  how a scope rots.
- the exclusions actually applied are **reported by `scan`**: one line, the
  count of files dropped by config-side excludes. A file leaving the corpus
  must never be invisible.

### 4.3 Both {#both}

Order is: expand includes → drop `DEFAULT_EXCLUDES` (components) → drop
`DEFAULT_EXCLUDE_FILES` (basenames) → drop config `exclude` (globs).

Edge cases: no `exclude` key ⇒ unchanged behaviour, silent. An `exclude`
pattern that is not a valid glob ⇒ a clean error naming the pattern, not a
panic and not a silent skip.

Error paths: unchanged everywhere else.

## 5. Boundaries {#boundaries}

- **Never edit `spec/**`.** §4 gains an anchor describing the `exclude` key —
  the **reviewer** writes it under sync-from-code and seals its verdict.
  Record in §9 the exact wording you would propose, for that pass.
- Do not touch the include globs themselves.
- Do not exclude anything not named here. In particular do **not** exclude all
  `INDEX.md` or all `cards/` — the cards are authored contracts and must stay
  observed; only the derived index leaves.
- Do not change `is_excluded`'s existing component semantics.

## 6. Acceptance {#acceptance}

```bash
cargo test --workspace
bash tools/self-check.sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive --campaign campaigns/packages-2026-09
```

- Observed file count falls **344 → 308**: 33 `LICENSE.xml` + 3 derived indexes.
  Report the number you actually get; if it is not 308, say so rather than
  adjusting anything to reach it.
- `--exhaustive` unmarked count falls by **at least the 264** the licence files
  contributed. It will still be red — ~8 700 package paragraphs remain genuinely
  unmarked, and that is Phase B's job, not this task's.
- Unit tests: a basename exclusion drops `packages/x/v0.1.0/LICENSE.md` and
  does **not** drop `packages/x/v0.1.0/spec/LICENSE-NOTES.md`; a config
  `exclude` glob drops the derived index and does **not** drop a sibling card;
  an `exclude` pattern matching nothing warns and names itself.
- **The proof the cards survived:** show that
  `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards/` still
  contributes its authored card files to the corpus, and only `INDEX.md` left.
  An over-broad exclusion is the way this task fails.
- Discipline: `cargo fmt --all`, clippy clean, no AI attribution.

## 7. Analogies {#analogies}

`DEFAULT_EXCLUDES` + `is_excluded` (`scope.rs:11-20`, `:95-100`) is the shape
you are extending. `observed_files` (`:73-93`) is where the order in §4.3 lives.

## 8. Stop rule {#stop}

If adding a config `exclude` turns out to need a `progress.toml` schema bump
(`schema = 1` today) to stay honest about compatibility: **STOP and say so** —
that is a contract decision, not an implementation detail.

If the observed count does not land on 308, STOP and report the delta with the
paths, rather than hunting for a pattern that produces the expected number.

Budget signal: past ~4 files, stop and return.

## 9. Log {#log}

- queued 2026-07-26 (Fable), on the owner's «делай». Both findings came from
  the owner asking whether everything generated had really been kept out. The
  answer was *almost*, and the three that slipped through were not found by
  the phrase sweep — that returned eleven files and all eleven were prose
  *about* generated code.

- executed 2026-07-26. §3's numbers all held; nothing had to be contradicted.
  Files touched: `crates/progress-core/src/scope.rs`,
  `crates/vibe-cli/src/commands/progress.rs`,
  `crates/vibe-cli/src/commands/progress/tests.rs` (one fixture field — the
  `Ground` literal stopped compiling), `progress.toml`, this file.

- **Measured, before → after:** observed files **344 → 308**, the predicted
  33 + 3 and no other movement: 36 records left the campaign cache, 0 entered,
  and the 36 are exactly 33 `LICENSE.xml` + 3 `spec/cards/INDEX.md`.
  `--exhaustive` unmarked **8 992 → 8 463**, a fall of 529 — the 264 the
  licences contributed (§6's floor) plus 265 from the three derived indexes,
  measured separately by scanning copies of them in a scratch tree. Markers
  unchanged at 4 990: nothing that left was carrying markup. Still red, as
  §6 said it would be; the remaining 8 463 are Phase B's.

- **The cards survived.** `rust-ai-native-lang/v0.7.0/spec/cards/` went 10
  files → 9: `INDEX.md` left, and `scaffold-a-generators` … `scaffold-i-codemods`
  all stayed. Across the three stacks 27 card files remain observed. The
  narrowness is also unit-tested rather than only observed: the fixture in
  `scope.rs` keeps a sibling card and a `spec/LICENSE-NOTES.md` while dropping
  `INDEX.md` and `LICENSE.xml`.

- Worth knowing for the next widening: the exclusion `packages/**/spec/cards/INDEX.md`
  is *today* indistinguishable in effect from `**/INDEX.md`, because no other
  `INDEX.md` is in the corpus (`spec/boot/INDEX.md` is already outside the
  include globs). The narrow form is deliberate anyway — the next package that
  ships an authored `INDEX.md` is the case the broad form would silently eat.

- **Schema bump (§8): not needed, and the reasoning is not "it is only
  additive".** `exclude` absent deserialises to an empty list, so every
  existing `progress.toml` behaves exactly as before — that much is ordinary
  additive change. The question §8 actually asks is about the other direction:
  a config that *uses* `exclude` read by an older core, which has no such
  field and (no `deny_unknown_fields`) ignores it silently. That reader
  observes the derived indexes and the licences, so it reports **more** files
  and **more** unmarked facts than the config asked for. The failure is loud
  and in the red direction — a gate that should be green goes red — never a
  silent green. A schema number exists to stop a reader from confidently
  misreading a file; nothing here can be confidently misread. Note also that
  `schema` is deserialised into `ScopeConfig` and **never read** by any code
  path, so bumping it would today be a comment with a number in it.

- **Proposed PROP-043 §4 wording, for the reviewer's sync-from-code pass.**
  One anchor added, after `##excludes-rationale`:

  > - @fact:CONFIG-EXCLUDE An optional `exclude` list of globs, matched against
  >   the `/`-separated repo-relative path and applied **after** the includes
  >   and after both default rules. Enumerated, never a wildcard escape
  >   hatch: it serves `##INCLUDE-STYLE`'s purpose — nothing observed by
  >   accident — because an enumerated exclusion is exactly as explicit and
  >   as reviewable as an enumerated include. A pattern matching nothing is
  >   a warning naming the pattern; the count of files removed is reported
  >   by `scan`; a pattern that is not a valid glob is an error naming it.
  >   Absent ⇒ empty ⇒ unchanged behaviour. @status:impl/done

  And one **body** amendment, which moves no anchor — `##DEFAULT-EXCLUDES`
  today says the always-on set is eight directories, while the code now also
  holds a file name, and an unamended §4 is the drift the next audit files:

  > - @fact:DEFAULT-EXCLUDES **Default excludes** (applied always, even under
  >   explicit includes): by path component — `vibedeps/`, `.vibe/`, `refs/`,
  >   `fixtures/`, `campaigns/`, `target/`, `node_modules/`, `**/vendor/`;
  >   and by file name — `LICENSE.xml`. @status:impl/done

  If the reviewer prefers to keep `##DEFAULT-EXCLUDES` verbatim, the file-name
  rule wants its own anchor (`##DEFAULT-EXCLUDE-FILES`) rather than going
  unrecorded — but that is a second added anchor, which §5 did not budget.
  The `include = [...]` example in §4 could also gain a commented
  `# exclude = []` line; it is illustrative, not normative.

- **The floor caught this task on a budget nobody had noticed was full.**
  `crates/vibe-cli/src/commands/progress.rs` was **589 / 600** lines before
  this change; the first, comment-rich version of the adapter edit took it to
  614 and `conform check` failed on `file-length` — the rule's own remedy
  being "split along the file's responsibility seams into module-grain
  cells". The adapter edit was rewritten to its true minimum (PROP-043 §5
  says this file holds no markup knowledge, so the *why* belongs in
  `scope.rs`, and that is where all of it now is), landing at **599 / 600**.
  That is green, and it is one line of headroom. **The next change to this
  file will not fit**, and the split the rule names — `Ground` / `ground` /
  `refresh_state` and the campaign-path helpers are one cohesive cell, the
  same shape `mod baseline` and `mod rescan` already use — is a structural
  decision this task did not own and did not take.

- Found and deliberately not absorbed: (1) `progress.toml`'s audit block
  gained a fresh accounting during this task (`286` observed under
  `packages/**`, "the 8 992 unmarked facts are authored text only") — both
  numbers are now stale (250 and 8 463) and its penultimate line runs long
  after the insertion, but that block is another author's live text and this
  task did not edit over it. (2) `ScopeConfig::schema` is parsed and never
  consulted — a validation gap, not this task's. (3) `progress scan`'s human
  output now has a conditional line; the JSON gained `"excluded"`
  unconditionally, since a JSON consumer that cannot see an exclusion is the
  invisibility §4.2 forbids.
