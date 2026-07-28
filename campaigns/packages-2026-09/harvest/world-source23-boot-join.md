# world â€” Â§3.1 sources 2 and 3, the boot-lane join

_Captured 2026-07-28 against `spec/boot/STATIC.md` and `vibedeps/`._

Source 3 as `files_written` is unusable â€” the field is `[]` for all 36 packages
(batch plan Â§2.3). This is the substitute: the host's boot lane is COMPILED from the
installed packages and carries a provenance marker per contribution, so the marker
joins package â†’ installed copy â†’ what the host actually reads.

```console
$ python campaigns/packages-2026-09/tasks/source23-boot-join.py
boot-lane join over 31 contribution(s) in spec/boot/STATIC.md
  installed, sourced, same word stream: 17
  problems: 14

  org.vibevm.world/campaign-plans  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-campaign-plans/0.1.0/spec/boot/40-flow-campaign-plans.md
    source   : packages/org.vibevm.world/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.md
    package 441 words, host 435 — 6 differ
    only in the package: cold facts verified at writing time

  org.vibevm.world/comparative-research  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-comparative-research/0.1.0/spec/boot/52-flow-comparative-research.md
    source   : packages/org.vibevm.world/comparative-research/v0.1.0/spec/boot/52-flow-comparative-research.md
    package 314 words, host 311 — 3 differ
    only in the package: sibling document pointers

  org.vibevm.world/dev-runtime-docs  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-dev-runtime-docs/0.1.0/boot/58-flow-dev-runtime-docs.md
    source   : <none found>

  org.vibevm.world/git-atomic-commits  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-atomic-commits/0.1.0/boot/30-flow-atomic-commits.md
    source   : <none found>

  org.vibevm.world/git-autonomy  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-autonomy/0.1.0/boot/32-flow-autonomy.md
    source   : <none found>

  org.vibevm.world/git-conventional-commits  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-conventional-commits/0.1.0/boot/31-flow-conventional-commits.md
    source   : <none found>

  org.vibevm.world/git-practices  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-practices/0.1.0/spec/boot/STATIC.md
    source   : <none found>

  org.vibevm.world/git-atomic-commits  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-atomic-commits/0.1.0/boot/30-flow-atomic-commits.md
    source   : <none found>

  org.vibevm.world/git-autonomy  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-autonomy/0.1.0/boot/32-flow-autonomy.md
    source   : <none found>

  org.vibevm.world/git-conventional-commits  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-git-conventional-commits/0.1.0/boot/31-flow-conventional-commits.md
    source   : <none found>

  org.vibevm.world/operating-modes  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-operating-modes/0.1.0/spec/boot/45-flow-operating-modes.md
    source   : packages/org.vibevm.world/operating-modes/v0.1.0/spec/boot/45-flow-operating-modes.md
    package 374 words, host 366 — 8 differ
    only in the package: recognise a codeword by intent not exact wording

  org.vibevm.world/sync-from-code  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-sync-from-code/0.1.0/boot/20-flow-sync-from-code.md
    source   : <none found>

  org.vibevm.world/two-process-model  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-two-process-model/0.1.0/spec/boot/05-flow-two-process-model.md
    source   : packages/org.vibevm.world/two-process-model/v0.1.0/spec/boot/05-flow-two-process-model.md
    package 436 words, host 433 — 3 differ
    only in the package: architecture consequences never

  org.vibevm.world/redbook  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-redbook/0.2.0/spec/boot/03-flow-redbook.md
    source   : packages/org.vibevm.world/redbook/v0.2.0/spec/boot/03-flow-redbook.md
    package 519 words, host 513 — 6 differ
    only in the package: spirit source member list git git
EXIT=1
```

```console
$ grep -oE "vibe:static [^ ]+" spec/boot/STATIC.md | sort | uniq -c | sort -rn | head -8
      2 vibe:static org.vibevm.world/git-conventional-commits
      2 vibe:static org.vibevm.world/git-autonomy
      2 vibe:static org.vibevm.world/git-attribution-policy
      2 vibe:static org.vibevm.world/git-atomic-commits
      1 vibe:static org.vibevm.world/wal-specspaces
      1 vibe:static org.vibevm.world/wal
      1 vibe:static org.vibevm.world/two-process-model
      1 vibe:static org.vibevm.world/tool-design-lessons
```

**Scope:** the Â§3.1 source-2 and source-3 check for every `world` flow that contributes a boot snippet. The anchor list is not maintained here â€” a verdict cites this file in its `ev[]`, and the reverse index is derived at the phase close (PHASE-C-BATCH-PLAN.md Â§5).
