# Phase D — the release queue {#root}

_Written 2026-07-29. The seventeen obligations §5-D calls **release events**:
each spans a package boundary, so none of them is closed by an edit — it is
closed by a published version and a re-vendor through `cargo xtask sync-engines`.
A fix landing in one family member and not its siblings is a new `duplication`
obligation, not a closure._

Regenerate the list at any time:

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
```

**17 obligations · 59 drift verdicts · 25 distinct packages.** Grouped below by
what the owner actually has to decide, which is not the same as by package.

---

## A. The address family — one defect, 19 packages {#addresses}

| id | n | packages | what fails |
|---|---:|---|---|
| `F-136` | 11 | conflict-protocol, decision-records, git-atomic-commits, health-audit, source-mirrors, sync-from-code, wal | `../flows/<name>/…` in a boot snippet resolves inside the package and to `spec/flows/…` in the compiled lane, which no consumer has |
| `F-145` | 8 | campaign-plans, comparative-research, dev-runtime-docs, git-attribution-policy, git-autonomy, git-conventional-commits, licensing, sync-from-code | same, on the `sibling-document-pointers` anchor |
| `F-240` | 2 | licensing, spec-genres | the root-relative variant: a re-derive prompt whose FIRST instruction is «Read `spec/flows/licensing/` end to end» |

**Decided already** (owner, 2026-07-29): the links take `@spec://` where they are
pointers and `#embed` where the target belongs in the lane; a generated boot
artifact carries no token budget
([PROP-009 `##ARTIFACTS-CARRY-NO-TOKEN-BUDGET`](../../spec/modules/vibe-workspace/PROP-009-loading-model.md#artifacts)),
so `#embed` is not constrained by lane size. PROP-035 §10's link tables are
**not** a precondition — `BACKLOG.md` B-001.

**Still owed by the owner:** approval to publish. Nineteen packages take a
version bump and a re-vendor. That is the whole ask for this group; the edit
itself needs no further ruling.

**Not in this group though it looks like it:** `F-153` (below) is a bare
`rust/…` / `go/…` / `cards/INDEX.md` path that is wrong *inside its own
package* — the targets live under `spec/`. It needs no tag and no decision, only
the correct intra-package path.

---

## B. The three-stack parallel corpus — 9 obligations {#stacks}

One fact projected per language, drifting in two or three stacks at once. The
recurring shape is **a Go-specific truth stated family-wide**: the Rust and
TypeScript sentences are often correct and the Go one is not, so a single
family-wide edit would break two working sentences to fix one.

| id | n | stacks | what fails |
|---|---:|---|---|
| `F-153` | 6 | go, rust, typescript `-lang` | boot snippet cites `rust/…`, `go/…`, `cards/INDEX.md`; all live under `spec/` |
| `F-115` | 3 | the three umbrella packages | the front door points at the `-lang` README and `typescript-ai-native-lang` ships **no README.md** |
| `F-186` | 3 | go, rust `-lang` | the fact cites three evidence ids; `H4` is in **no** register in this repository |
| `F-187` | 3 | go, rust, typescript `-lang` | the two **Go** skills are not installed — `.claude/skills/` carries four of six |
| `F-188` | 3 | go, rust, typescript `-lang` | the printed CLI signature takes five parameters; the shipped verb takes two |
| `F-189` | 3 | go, rust, typescript `-lang` | the host does not dispatch `go` — PROP-026 accepts `typescript` and `rust` |
| `F-190` | 3 | go, rust, typescript `-lang` | **the verdict is half false**: `DISABLED by policy` IS shipped; only `Defaulted` is wrong, and the three sentences are not word-identical |
| `F-211` | 2 | go, rust `-lang` | `init` prints one parameter and five keys; the shipped op takes none and returns four |
| `F-212` | 2 | go, rust `-lang` | `gated_packages` → `gated_crates`, and three kind strings are wrong |
| `F-213` | 2 | go, rust `-lang` | `capture.sh` exists only at the **host's** `discipline/golden/`; no ai-native package carries a `discipline/` at all |

**Prepared and reverted once.** Diffs for F-153, F-190, F-211, F-212 were
written by workers on 2026-07-29, reviewed, and reverted wholesale with the rest
of the mis-routed batch. The reasoning survives in
`harvest/d1-go-ai-native-lang-repairs.md` and `harvest/d1-rust-ts-lang-repairs.md`
and does not need re-deriving.

**Two need a ruling before any edit, not just before publication:**

- **`F-189` — the host does not dispatch `go`.** Three shipped packages claim a
  capability the consumer names as *not accepted*. Either the host grows `go`
  dispatch (Phase E work on PROP-026) or the three packages stop claiming it.
  That is a product decision, not an editorial one.
- **`F-187` — the two Go skills are not installed.** Same shape one layer down:
  install them, or the fact stops saying they are there.

The other seven are factual corrections whose only owner gate is publication.

---

## C. Composition claims across flows — 3 obligations {#composes}

| id | n | packages | what fails |
|---|---:|---|---|
| `F-219` | 2 | addressable-specs, campaign-plans | the behaviour is real, the **attribution** is wrong: 515 commits cite a `spec://` URI, but the rule requiring it lives in `flow:git-conventional-commits`, not `-atomic-commits` |
| `F-220` | 2 | addressable-specs, source-mirrors | the composition is specified on both sides and does not happen in the one consumer |
| `F-233` | 2 | git-attribution-policy, source-mirrors | a composition whose point is that the choice is recorded as a decision — and in the one consumer that installs both, it is not |

`F-219` is a pure attribution fix. `F-220` and `F-233` are **§3.6 route (b)
candidates**: the composition is sound and the consumer does not perform it, so
the package may not be the side that moves. Both need the which-side ruling
before an edit exists to approve.

---

## D. Arithmetic — 1 obligation {#arithmetic}

`F-251` (2 verdicts, spec-genres + tool-design-lessons): «four pieces of content
plus a boot snippet» is five things, four bullets follow, and the fourth bullet
IS the boot snippet. 14 of 16 sibling packages say «three». A count, checkable
against the package's own contents.

---

## What is being asked, in one screen {#ask}

1. **Group A — publish.** One approval covering 19 packages; the repair is
   decided and needs no further ruling.
2. **Group B — two product decisions first** (`F-189` go dispatch, `F-187` the
   Go skills), then publish the other seven.
3. **Group C — one which-side ruling** on `F-220` and `F-233`; `F-219` needs
   only publication.
4. **Group D — publish.**

Nothing here is edited before its ruling. Diffs are prepared, shown, and
approved per §1.2, which is the order the first wave got wrong and paid for.
