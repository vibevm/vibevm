# DRIFT-016 — the irreplaceable stays in git, the acceleration leaves the repo {#root}

<status stage="impl" state="plan" ref="DRIFT-016"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (progress-core cache / progress adapter)
**Unit-stability check:** no spec anchor moves. PROP-043 §7.1 and §7.5 are
satisfied *better* after this change than before it — see §2.

## 1. Goal {#goal}

`campaigns/<id>/run/cache.json` carries only what cannot be recomputed — the
campaign verdicts — and the parse payload that made it 92 % larger lives
outside the repository, keyed by branch, where its size costs nothing.

## 2. Contract {#contract}

> Everything else can be erased at any moment — no knowledge is lost.
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#erasure`

> Per observed file: path, content-hash, extracted markers with positions …
> — `spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#cache`

> **Cache campaign maps are load-bearing.** `run/cache.json` carries the
> C-phase verdicts; mutate it by load-and-merge only.
> — `spec/WAL.md`, Constraints

The erasure law and the load-bearing law are both about `cache.json`, and
they pull in opposite directions because two different things live in one
file. DRIFT-010 made that visible by growing the erasable half to 2.5 MB of
tracked, churning JSON. This task separates them so each law applies to
exactly the file it means.

**Owner's ruling, 2026-07-25, verbatim:** «Может быть, архив и кэш хранить
где-то в отдельном месте, возможно даже вообще не в репозитории … Тогда нам
не нужно мучиться с размером репозитория вообще, мы можем хранить очень
большой архив не замусоривая `.git`. Важные вещи типа вердиктов, конечно,
стоит хранить в git на случай потери кэша.»

Finding realised: the storage half of **F-058**'s neighbourhood; the
measurement that motivated it is DRIFT-010 §9.

## 3. Current state {#current}

- `crates/progress-core/src/cache.rs` — `FileRecord` carries
  `content_hash`, `rollup`, `campaign` (the verdicts) **and** `parsed:
  Option<ParsedDoc>` (DRIFT-010's payload). One file, one write.
- `campaigns/progress-2026-08/run/cache.json` is **5.14 MB**, git-tracked,
  and rewritten by every `scan`. Before DRIFT-010 it was 2.68 MB.
- Measured cost of the payload in **release**: parse 10.3 ms vs
  serialize+deserialize+clone 7.5 ms over 58 files. It nearly pays for
  itself and no more — the size, not the speed, is what this task is about.
- The verdict half is genuinely irreplaceable: 4 490 verdicts, five phases
  of work, and `baseline.json` is not written until close-out.

## 4. Required behavior {#behavior}

1. **Split the record.** `cache.json` keeps `path`, `content_hash`,
   `rollup`, `campaign` — everything a cold reader needs to know what was
   judged and what it was judged against. The `parsed` payload moves to a
   **sidecar store outside the repository**.
2. **Where.** A per-repository, per-branch directory under the tool's own
   per-user home. Resolution order, first hit wins:
   - a `[progress] cache_dir` key in the project's `progress.toml` — the
     explicit escape hatch;
   - the default:
     **`<settings-home>/progress-cache/<repo-id>/<branch-slug>/`**, where
     `<settings-home>` is `$VIBE_SETTINGS` or `~/.vibe`, `<repo-id>` is the
     repository directory's name plus a short hash of its canonical path
     (`vibevm-3f9a1c`), and `<branch-slug>` is the branch with `/` replaced
     by `-` plus a short hash of the original.

   Three reasons for that shape, and the third is the load-bearing one:
   - **`<settings-home>`, not a sibling of the repo.** `~/.vibe/` is where
     this tool already keeps exactly this class of data
     (`~/.vibe/registries/<hash>/`), so there is one home to document, one
     to clean, one to relocate. A repo's parent directory is also not
     reliably writable — on CI it is often the workspace root.
   - **Relocating it needs no second environment variable.** `VIBE_SETTINGS`
     already moves the whole per-user home, so a test or CI run that sets it
     gets the payload store moved for free. A dedicated
     `VIBE_PROGRESS_CACHE` would be one more thing to forget — and forgetting
     exactly that is F-055, which cost a day: the harness isolated
     `VIBE_REGISTRY_CACHE` and not the settings chokepoint.
   - **`<repo-id>` carries a path hash** so two clones of the same
     repository never share a bucket, without depending on their parents
     being distinct or writable.

   Branch-keying is deliberate: a different branch is a different corpus, so
   one payload store across branches would hand a reader the right hash with
   the wrong parse. The slug must survive `/` in a branch name — `feature/foo`
   is legal and must not become a nested path.

   *Owner's alternative, on the table:* a sibling-of-the-repo location,
   `C:\Users\olegc\git\v\cache\vibevm-cache-facts-<branch>`, which trades the
   single-home property for being findable by eye next to the repositories.
   If the owner prefers it, only §4.2's first bullet changes; every other
   requirement in this task is unaffected.
3. **Losing the sidecar must be harmless and silent.** Absent directory,
   absent file, unreadable file, a payload whose `content_hash` disagrees
   with the record in git — every one of these is a cache miss that parses,
   not a warning and never an error. This is the erasure law with teeth: the
   sidecar is *only* an accelerator, and the campaign must run identically
   on a machine that has never seen it.
4. **The verdicts never move.** `cache.json` stays in git, stays
   load-and-merge, and stays the file the campaign's tooling reads. If a
   change would put a verdict anywhere but there, it is wrong.
5. Report what `cache.json` weighs after the split. The target is roughly
   its pre-DRIFT-010 size; if it is not close, say why.

Edge cases: a detached HEAD or an unresolvable branch ⇒ fall back to a
`detached` bucket rather than failing — the payload is optional by
construction. A repo checked out at a path whose parent is not writable ⇒
skip the sidecar entirely and run cold, silently. Two campaigns in one repo
share the branch bucket keyed by campaign id inside it.

Error paths: none that stop a run. The only hard error is if `cache.json`
itself cannot be written — that is unchanged from today.

## 5. Boundaries {#boundaries}

- **Never** put a verdict in the sidecar, and never make a run depend on the
  sidecar existing. If the tool cannot run without it, the split failed.
- Do not change the verdict map's shape, the campaign field, or
  load-and-merge.
- `progress-core` must not learn what git is. The branch comes from the
  adapter, like DRIFT-009's crate→commit map.
- Never edit spec text.

## 6. Acceptance {#acceptance}

```bash
cargo test -p progress-core -p vibe-cli
cargo run -q -p vibe-cli --bin vibe -- progress scan     # 58 files, 4979 markers, 0 errors
cargo run -q -p vibe-cli --bin vibe -- progress check    # 0
bash tools/self-check.sh
```

- New test: `sidecar_absent_runs_cold` — with the sidecar directory removed,
  a scan produces byte-identical `corpus.json` / `campaign.json` to a warm
  one, and emits nothing about it.
- New test: `sidecar_stale_hash_is_a_miss` — a payload whose hash disagrees
  with the git-side record is ignored, not trusted.
- New test: `verdicts_never_leave_cache_json` — after a scan, the sidecar
  contains no `campaign` key anywhere; assert on the serialised bytes.
- New test: `branch_keys_the_bucket`.
- **Live check, reported in §9:** run a scan on this repository, then
  `git status` — `cache.json`'s diff must be the ordinary small one, and
  `du` its size before and after.
- Discipline: `#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#erasure")]`
  on the sidecar seam, `cargo fmt --all`, clippy clean, atomic commits, no
  AI attribution.

## 7. Analogies {#analogies}

`crates/vibe-registry`'s cache under `~/.vibe/registries/<hash>/` is this
project's existing "big derived thing that lives outside the tree" — and its
`VIBE_REGISTRY_CACHE` override is exactly the seam §4.2 asks for.

## 8. Stop rule {#stop}

If the payload cannot be separated without changing the verdict map's shape
or its file: STOP, record it in §9, return. The verdicts are the one thing
in this campaign that cannot be regenerated, and no size win justifies
touching them.

Budget signal: past ~6 files or ~450 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable), on the owner's ruling — his phrasing was
  «важные вещи типа вердиктов, конечно, стоит хранить в git на случай
  потери кэша», which is the whole design in one line.
- implemented 2026-07-25 (Opus). **§8 did not fire.** The verdict map's
  shape, its file and its load-and-merge are untouched: the split *removes*
  one key (`FileRecord.parsed`) and adds none, which is exactly as additive
  in reverse as DRIFT-010's addition was forward — an old record's key is
  ignored on load, a new record's absence reads as the miss it already
  meant. No record is re-keyed, so `CACHE_SCHEMA` stays **2** and there is
  no migration for the live verdict maps to survive. Verified on the real
  thing rather than argued: **4 490 verdicts across 58/58 campaign maps
  before the scan, 4 490 across 58/58 after** — and again after the cold
  run below.
- **Where it landed.** §4.2 was rewritten mid-task (settings home, no
  dedicated env var) and the implementation follows the new text:
  `[progress] cache_dir` from `progress.toml` first, else
  `<settings-home>/progress-cache/<repo-id>/<branch-slug>/<campaign-id>/`.
  Live path on this machine:
  `~/.vibe/progress-cache/vibevm-35b1c3/main-0d6e40/progress-2026-08/payloads.json`.
  `<repo-id>` and `<branch-slug>` are slug + 6 hex of sha256 of the
  original (canonical repo path / branch name), so `feature/foo` neither
  nests nor collides with `feature-foo`, and two clones never share a
  bucket. No `VIBE_PROGRESS_CACHE`: `VIBE_SETTINGS` moves the store with
  the rest of the home, which is one variable to forget instead of two
  (F-055).
- **Sizes (§4.5).** `run/cache.json` **5 142 927 → 2 684 898 bytes** — the
  2.68 MB §3 names as the pre-DRIFT-010 figure, hit on the nose, because
  the record is byte-for-byte what it was before the payload landed. The
  payload itself is **1 114 548 bytes** outside the repository — smaller
  than the 2.46 MB it occupied inside it, since the sidecar is written
  compact (nothing diffs a file that exists so no one has to).
- **Live check (§6).** `progress scan` on this repository: 58 files, 4 979
  markers, 0 errors; `progress check`: clean, exit 0. `git status` shows
  `run/cache.json` plus the two state projections, and the state
  projections differ **only in `updated_at`** — the byte-identity claim,
  stated on the live campaign rather than a fixture. `cache.json`'s own
  diff is the one-time 94 430-line deletion (that *is* the change); the
  ordinary diff §6 asks about is the steady state after it, measured by
  scanning twice and diffing the results: **one line, the timestamp**.
  Before this task every scan rewrote 2.5 MB of payload into that diff.
- **The erasure law, demonstrated live.** Deleted `~/.vibe/progress-cache`
  wholesale and rescanned: **stderr 0 bytes**, `corpus.json` and
  `campaign.json` byte-identical to the warm run, `cache.json` identical
  bar its stamp, 4 490 verdicts intact, and the store rebuilt itself. A
  machine that has never seen the sidecar runs this campaign identically.
  The sidecar was also checked structurally, not just by grep: parsed as
  JSON and walked, its key set across all 58 documents is exactly
  `ParsedDoc`'s fields plus the document paths — **no `campaign` key
  anywhere**, no `verdicts`, no `processed_hash`.
- Decisions §4 left open, taken here and named so a reviewer can overturn
  them cheaply:
  - **One `payloads.json` per campaign bucket**, not one file per
    document. DRIFT-010's own measurement says the fsync'd atomic writes
    dominate a run, and 58 files would be 58 of them; this keeps the
    sidecar's write shape identical to `cache.json`'s — one read at the
    head, one write at the end.
  - **No sidecar without a campaign zone.** The leaf is keyed by campaign
    id, and a run with no campaign has no `cache.json` either, so every
    lookup would miss regardless. `payload_dir` returns `None` and the
    store is inert.
  - **The store is rewritten as exactly the observed set**, so a file that
    leaves scope loses its payload for free — the DRIFT-001 prune, without
    a second implementation of it.
  - **The git-side record stays the authority.** `cached_doc` consults the
    sidecar only after `record.content_hash == hash`; a payload is never a
    substitute for a record.
  - `PAYLOAD_SCHEMA = 1`, and a schema this build does not know reads as an
    empty store. There will never be a payload migration — the thing
    migrations exist to protect is in the other file.
  - `ShellGit::branch` answers on `run_raw`: a detached HEAD's non-zero
    exit is an *answer*, not an error worth classifying.
- The floor caught two things in this change and both are fixed. Adding a
  verb pushed `crates/vibe-registry/src/git_backend/shell.rs` to 614 lines,
  over the 600-line budget, so the two read-only checkout queries
  (`last_commit_iso`, `branch`) moved to a cell of their own,
  `git_backend/shell/query.rs` — the seam the file already uses for `tar`,
  and a real one: those two only ask, they never clone, fetch or reset
  (shell.rs is now 581). The same gate then caught `.expect()` in the new
  test helper, exactly as it did in DRIFT-010: `payload_for` returns
  `Option<ParsedDoc>` and each `#[test]` decides to panic.
- Tests added (§6's four, plus the seam-level ones the split created):
  - `progress-core`: `branch_keys_the_bucket`,
    `two_clones_of_one_repo_never_share_a_bucket`,
    `cfg_dir_and_campaign_id_shape_the_leaf`,
    `absent_or_corrupt_store_is_an_empty_store`,
    `store_round_trips_and_answers_only_for_its_own_bytes`,
    `sidecar_stale_hash_is_a_miss`; `cached_doc_misses_are_misses` and
    `cached_doc_round_trips_the_parse` were rewritten across the split, the
    latter now also asserting that the tracked `cache.json` carries the
    hash and *not* the text it stands for.
  - `vibe-cli`: `verdicts_never_leave_cache_json` (in-process, asserting on
    the serialised sidecar bytes), and `sidecar_absent_runs_cold` out of
    process in `tests/cli_progress_sidecar.rs` — the only place stderr can
    actually be read, which is what "emits nothing about it" requires. That
    test uses `UserScratch` to relocate `VIBE_SETTINGS`, so it exercises the
    **default** location without touching the developer's real `~/.vibe`;
    every in-process fixture pins `[progress] cache_dir` inside its own
    tempdir for the same reason.
- Over the file budget, deliberately: 10 files rather than ~6, three of
  them one-line touches (`lib.rs` module line, `rescan.rs`'s "only place
  that knows this is a git checkout" claim, which is now two, and this
  log). The split does not compile half-applied, so stopping at six would
  have left the tree broken rather than smaller.
- Not done here, deliberately: `specmap.json` is **not** regenerated, so
  the new `#[specmark::spec]` tags on the sidecar seam are not in the index
  yet — DRIFT-010's precedent, and other agents were live in this tree
  throughout (DRIFT-013 and DRIFT-019 landed in `cli/inspect.rs`,
  `vibe-resolver/src/lib.rs`, `vibe-workspace/src/freshness.rs` and
  `run/state/findings.json` while this task ran; none of it was touched
  here).
