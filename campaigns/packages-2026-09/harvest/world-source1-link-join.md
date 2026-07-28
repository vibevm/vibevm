# world — §3.1 source 1, the link join

_Captured 2026-07-28 against `packages/org.vibevm.world/`._

§3.1's first source is the package's own shipped artifacts: «a protocol document
a snippet cites must exist and say what the snippet says it says». This run settles
the mechanical half — existence and anchor presence. The «says what it says» half is
judgement and is not delegated to a script.

**The observed corpus — the 121 files this campaign judges:**

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py --corpus
source-1 join over 121 file(s) under packages/org.vibevm.world
  relative .md citations resolved: 185
  broken: 0
EXIT=0
```

**The whole tree, including the 33 files the campaign's `exclude` globs drop:**

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py
source-1 join over 154 file(s) under packages/org.vibevm.world
  relative .md citations resolved: 187
  broken: 2

  MISSING FILE: 2

  MISSING FILE    packages/org.vibevm.world/redbook/v0.1.0/spec/book/ru/chapter-1-two-process-model.md
                    -> safeharbor.md
  MISSING FILE    packages/org.vibevm.world/redbook/v0.2.0/spec/book/ru/chapter-1-two-process-model.md
                    -> safeharbor.md
EXIT=1
```

**Read the two runs together.** 185 citations in corpus, 187 over the tree: the
difference is exactly the two broken ones, so every citation in a dropped file that
resolves is already counted in the corpus run, and the corpus itself is clean. Both
failures are the same one — `safeharbor.md`, cited by the book's chapter 1 in both
`redbook/v0.1.0` and `v0.2.0`, and present nowhere in the repository.

`spec://` URIs are deliberately not resolved. Of 55 occurrences under this tree,
all but two are illustrative (`spec://com.example.shop/PROP-001#…`,
`spec://oproto/PROP-002#…`, a bare `spec://…`) — they teach the grammar rather than
cite a document, and a resolver over them would bury two real references under
fifty-three correct examples.

**Scope:** the §3.1 source-1 existence check for every fact under `packages/org.vibevm.world/` that cites a sibling document. The anchor list is not maintained here — a verdict cites this file in its `ev[]`, and the reverse index is derived from the verdict maps at the phase close (PHASE-C-BATCH-PLAN.md §5).
