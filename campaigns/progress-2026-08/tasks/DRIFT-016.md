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
> — `spec://vibevm/modules/vibe-progress/PROP-043#erasure`

> Per observed file: path, content-hash, extracted markers with positions …
> — `spec://vibevm/modules/vibe-progress/PROP-043#cache`

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
2. **Where.** A per-repository, per-branch directory. Resolution order, first
   hit wins:
   - `VIBE_PROGRESS_CACHE` (env), for CI and for tests — a test that writes
     a real user location is the defect DRIFT-012 just spent a day on;
   - a `[progress] cache_dir` key in the project's `progress.toml`;
   - the default: `<repo-parent>/cache/vibevm-cache-facts-<branch>/`,
     where `<branch>` is the current git branch. The owner's example is
     `C:\Users\olegc\git\v\cache\vibevm-cache-facts-main`.
   Branch-keying is deliberate: a different branch is a different corpus, so
   sharing one payload store across branches would hand a reader the wrong
   parse for the right hash.
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
- Discipline: `#[spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#erasure")]`
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
