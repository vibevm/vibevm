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

### A.1 The group is larger than three obligations, and the measurement says why {#addresses-scope}

_Measured 2026-07-29, wave 6. Reproduce the lane counts with
[`tasks/address-repair.py`](tasks/address-repair.py)._

**The package is not the broken side.** The same link resolves or dangles
depending only on which lane you read it in:

| lane | `../flows/…` links | dangling |
|---|---:|---:|
| `packages/**` — where the text is authored | 70 | **0** |
| `vibedeps/**` — the installed slots | 142 | 21 |
| `spec/boot/STATIC.md` — where a session reads it | 75 | **75** |

`spec/flows/` does not exist in this host (`ls spec/` → `WAL.md boot common
design manual-tests modules terraforms`). The boot compiler concatenates snippet
bodies verbatim (PROP-035's linker stage), so a relative path that meant
`<pkg>/spec/flows/…` in the package means the host's `spec/flows/…` once
compiled. The defect is the **form**: a relative path cannot survive being
moved, and an `@spec://` address can. That is why the owner's ruling puts the
repair in the packages and not in the compiler — and it is also why the repair
**cannot be verified by editing a package**.

- ##A1-EVERY-ROUTE-NEEDS-PUBLICATION **The consequence for the queue.** `spec/boot/STATIC.md` is
  generated from `vibedeps/` — its own provenance comments say so
  (`<!-- vibe:static org.vibevm.world/addressable-specs — vibedeps/flow-addressable-specs/0.1.0/… -->`).
  So a package edit reaches the lane only through a version bump and
  `cargo xtask sync-engines`. **No address obligation closes without
  publication, whatever route the registry assigns it.** Joining the repaired
  links to the registry by their governing anchor gives **24 obligations · 54
  drift verdicts · 22 packages**, and only **two** of those sit on the `release`
  route:

  | route | obligations |
  |---|---:|
  | `prose-edit` | 19 |
  | `release` | 2 (`F-136`, `F-145`) |
  | `build-or-demote` | 2 (`F-316`, `F-332`) |
  | `sync-from-code` | 1 (`F-087`) |

  The nineteen `prose-edit` rows read as ordinary boss work and are not: their
  verdicts name the compiled lane explicitly — «the compiled lane keeps
  `../flows/addressable-specs/…`» (F-193), «`STATIC.md:1135`» (F-334),
  «STATIC.md:1365 keeps the relative link» (F-348), «a booting session that
  follows the link from the compiled lane lands on nothing» (F-145). **One
  approval covers all 24.**

  **Twenty are wholly in the family; four are partial, and the difference is
  worth stating rather than rounding.** Of the 54 verdicts these 24 carry, **47
  sit on a repaired link**. The other 7 belong to four obligations that the join
  catches by one anchor — `F-136` 10 of 11, `F-245` 1 of 2, `F-087` 1 of 3,
  `F-173` 1 of 4 — and those off-link anchors are a different defect that closes
  independently. `F-173` is the clearest case: its opening verdict is about
  missing access dates, and it enters this family only because
  `##LAW-DELTAS-NOT-DECREES`' sentence happens to end in a dangling pointer.
  **So: 47 verdicts blocked on publication, 7 not.**
- ##A1-THE-EDIT-IS-A-COMMAND **The edit is prepared as a transformation, not as 62 hand edits.**
  `tasks/address-repair.py` computes every replacement, refuses to apply if any
  emitted address does not resolve, and is line-indexed rather than text-wide
  (a whole-text replace was caught being wrong — `two-process-model` carries the
  identical link on two lines). Dry-run, verified: **62 link constructs · 25
  files · 25 packages · 62/62 addresses resolve · 0 malformed against the
  PROP-035 §6 grammar · 0 residual `../flows/` after the rewrite.** The 62
  constructs cover all 69 raw occurrences because 7 carry the path twice, once
  as visible link text.
- ##A1-ALL-POINTERS-NO-EMBEDS **The `#embed` half of the ruling has no member here.** The owner
  ruled `@spec://` for pointers and `#embed` where the target belongs in the
  lane. Read line by line, **all 69 are pointers** — «Full protocol:», «Full
  model:», «Full rationale:», «Grammar and forms:», «Responsibility table:»,
  «read …». Every one deliberately withholds the target's content. The emitted
  form copies the house form already live in the host's own spec
  (`spec/common/PROP-000.md:161-164`, `PROP-016:8`):
  `spec://<group>/<name>/<doc-path>#<anchor>`, no `.md`, always an anchor.
- ##A1-F240-IS-SCOPED-AT-TWO-AND-THE-DEFECT-IS-IN-SEVENTEEN **`F-240`'s scope is wrong, and this is the one thing here that
  changes what the owner should approve.** The root-relative variant — a
  re-derive prompt whose first instruction is `Read spec/flows/<name>/ …` —
  is recorded in two packages and **present in seventeen**: addressable-specs,
  comparative-research, conflict-protocol, decision-records, discovery-prompt,
  git-attribution-policy, health-audit, licensing, managed-blocks, manual-tests,
  operating-modes, qualified-naming, secrets-hygiene, source-mirrors,
  spec-genres, two-process-model, wal. The fifteen unrecorded ones are not
  mis-judged verdicts — the instruction lives **inside a fenced block**, which
  carries no anchor, so which of the prompt's claims got tested varied by
  worker. Filed as [`BACKLOG.md` B-004](../../BACKLOG.md#b-004). **Publishing
  the two-package fix alone is what §4.5 calls not a closure**; the ask should
  be scoped at seventeen or the remainder recorded as a deferral.

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

1. **Group A — publish, and the approval covers more than the three rows say.**
   Measured in §A.1: the `../flows/…` defect exists only in the **compiled
   lane**, which is generated from `vibedeps/`, so every address obligation —
   **24 of them across four routes, not the 3 listed here** — closes through
   publication and none is boss-closable before it. The edit itself is decided
   and is one verified command
   ([`tasks/address-repair.py`](tasks/address-repair.py): 62 links, 25 packages,
   62/62 resolve, 0 residual). **One further decision is owed and it is new:**
   `F-240`'s root-relative variant is recorded in 2 packages and present in
   **17** (`BACKLOG.md` B-004) — publish the narrow fix and fifteen packages ship
   the same broken first instruction.
2. **Group B — two product decisions first** (`F-189` go dispatch, `F-187` the
   Go skills), then publish the other seven.
3. **Group C — one which-side ruling** on `F-220` and `F-233`; `F-219` needs
   only publication.
4. **Group D — publish.**

Nothing here is edited before its ruling. Diffs are prepared, shown, and
approved per §1.2, which is the order the first wave got wrong and paid for.
