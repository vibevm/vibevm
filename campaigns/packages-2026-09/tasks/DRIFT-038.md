# DRIFT-038 — the four dead package names retired {#root}

**Finding:** F-097 (campaign §7 LOG, filed 2026-07-27, widened at B10 and B15).
**Executor:** the reviewer, directly. **Gate:** floor + `progress check
--exhaustive` + `vibe check`. **Landed:** 2026-07-28.

## The transform {#transform}

`520e7478` renamed four `git-practices` members and left the prose that cites
them behind. Every live reference to the dead name becomes the `git-` name:

| dead | live |
|---|---|
| `atomic-commits` | `git-atomic-commits` |
| `attribution-policy` | `git-attribution-policy` |
| `conventional-commits` | `git-conventional-commits` |
| `autonomy` | `git-autonomy` |

## A path is not a name — the trap that shapes the whole task {#trap}

**Only the PACKAGE was renamed.** Three things kept their short names and are
correct as they stand:

- the **flow directory** inside each package — `spec/flows/atomic-commits/…`;
- the **document** — `spec/flows/conventional-commits/conventional-commits.md`;
- a **`spec://` URI's** flow and document segments —
  `spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`
  is *entirely correct* and contains the dead string twice.

A blanket replace would have corrupted paths three ways. Every site was
classified before any edit, and 9 occurrences were **deliberately left**.

## What was done, by form {#sites}

**50 edits across 29 files.**

| form | sites | note |
|---|---|---|
| `flow:<dead>` | **38** | 33 in `packages/`, **5 in the host's `PROP-003`** |
| `` `<dead>` `` backticked | 8 | `redbook`'s roster, `git-practices`' members, two prose refs |
| `**<dead>**` bold | 1 | inside an installed boot snippet |
| bare, undelimited | 3 | **a delimiter-anchored grep never sees these** |

**F-097's recorded count — 21 files, 33 references — was exactly right for the
form it measured** (`flow:`-prefixed, in `packages/`). Beyond it lay 12 live
sites in other forms and 5 in the host tree the sweep had never scanned. The
warning recorded at B15 — *do not build the site list from a delimiter-anchored
grep* — is what produced the extra twelve.

## Deliberately not changed {#omitted}

- **`redbook/v0.1.0/**` — 4 sites.** A superseded version slot is frozen
  history (§3.3); it left the corpus rather than being marked, and it is not
  edited either.
- **5 sites that are the directory or document name**, per §trap.
- **`vibevm/vibespecs/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml` — 7 sites.** That
  is the finding's own text; the dead names there are the record of what was
  wrong.

## The one judgement call {#judgement}

`conventional-commits.xml`'s heading `## Interaction with the atomic-commits
rule` names a **rule**, not obviously an installable. Changed anyway: the rule
has no name other than the package's, and the sibling line two files over says
«the separate `git-atomic-commits` flow». **Surfaced to the owner before the
commit and approved.** The `{#atomicity}` anchor is untouched, so nothing that
cites it moves.

## Acceptance {#acceptance}

- ✅ **Zero live dead package references remain.** Re-scan finds 9, all of them
  the frozen slot or the directory/document name.
- ✅ **Six unusable command lines now resolve** — three `vibe install`, one
  `vibe uninstall`, plus two more found in this pass.
- ✅ `vibe check` 0 errors, 0 warnings, 0 info.
- ✅ `progress check --exhaustive` clean over 259 files, 0 unmarked — the edits
  sit inside marked units and carry their markers.
- ✅ `tools/self-check.sh` exit 0.

**Every replacement asserted its own match count before writing.** A
`str.replace` that finds nothing reports success and changes nothing, which this
campaign has already been bitten by once.
