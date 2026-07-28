# W2 — two-process-model, wal, wal-specspaces, sync-from-code: the three sources

_Captured 2026-07-28 at the W2 opening. Every number below is the output of the
command printed above it._

W2 is the batch where §3.1's source 2 is at its strongest anywhere in `world`: these
four flows specify the session ritual this repository actually runs, and the
artifacts they name — `spec/WAL.md`, `CONTINUE.md`, `SPECSPACES.md`, the session
commands — are all in the host and all in daily use. A claim here is rarely
unverifiable; it is right or it is drift.

## Source 1 — the package agreeing with itself {#source-1}

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py \
    packages/org.vibevm.world/two-process-model packages/org.vibevm.world/wal \
    packages/org.vibevm.world/wal-specspaces packages/org.vibevm.world/sync-from-code
source-1 join over 23 file(s) under …
  relative .md citations resolved: 23
  broken: 0
```

**Twenty-three relative citations, none broken** — twice W1's eleven, over the same
number of files. The mechanical half of source 1 is clean for this batch too.

## Source 3 — the installed reality {#source-3}

```console
$ python campaigns/packages-2026-09/tasks/source23-boot-join.py
  org.vibevm.world/two-process-model  [INSTALLED SOURCED WORDS-DIFFER]
    package 436 words, host 433 — 3 differ
    only in the package: architecture consequences never
  org.vibevm.world/sync-from-code  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-sync-from-code/0.1.0/boot/20-flow-sync-from-code.md
    source   : <none found>
```

Two of the four are on the join's problem list and two are not — `wal` and
`wal-specspaces` resolve cleanly and carry the package's exact word stream.

- **`two-process-model` is the corpus's only WORDS-DIFFER case in this batch**: the
  host runs a snippet three words shorter than the one the package ships, and the
  three missing words are `architecture`, `consequences`, `never`. That is the drift
  §3.1 source 3 exists to catch, and no amount of reading the package would find it.
- **`sync-from-code` is NO-SOURCE for the same reason four of W1's five were**: the
  installed copy sits at the pre-DRIFT-039 `boot/` path while the package ships at
  `spec/boot/`, so the join cannot pair them and declines to compare words.

**The `wal-status` skill is shipped and not installed.**

```console
$ ls packages/org.vibevm.world/wal/v0.2.0/spec/skills/wal-status/
SKILL.md
$ ls .claude/skills/
rust-ai-native-sweep  rust-ai-native-terraform
typescript-ai-native-sweep  typescript-ai-native-terraform  vibevm
```

The package ships a `wal-status` skill and its boot snippet calls it «the fast form»
of the session-start WAL read. The host's skill directory carries five skills and
none of them is it — the same shape C6 found for the two Go skills.

## Source 2 — the host's observed conformance {#source-2}

The consuming project is this repository, and it runs all four rituals.

```console
$ test -f spec/WAL.md && test -f CONTINUE.md && test -f SPECSPACES.md ; echo present
present
$ grep -m1 '^_Updated' spec/WAL.md
_Updated: 2026-07-28 (**Phase C — the reviewing debt is CLOSED and `world` batch W1
$ grep -n '^default:' SPECSPACES.md
18:default: host
```

All three artifacts the family names exist, the WAL carries the `_Updated:` line its
protocol requires, and `SPECSPACES.md` declares the `default:` its own flow's
target-resolution law depends on — `host`, which is why a bare resume phrase in this
repository restores the host and not the one registered specspace.

**Two rituals worth measuring per fact rather than here**, because they are the
substance of the wal and sync-from-code flows respectively: whether the host's
session-end sequence matches the protocol's step list (`CLAUDE.md`'s
`ЗАВЕРШИ СЕССИЮ` section against `session-end-hook.md` and `cold-resume.md`), and
whether any spec edit in this repository's history followed the sync-from-code
path — propose the diff, do not apply, commit as `docs(spec): sync …`. The scope
histogram gives the second one a starting point: `docs(spec)` runs 82 of the last
400 commits.

**Scope:** §3.1 sources 1, 2 and 3 for the four flows of batch W2.
