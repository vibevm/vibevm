# Adoption guide {#root}

**Scope of this document.** The practical work of shipping managed
blocks: migrating an existing tool that overwrites whole files onto a
block without eating its users' data, a fixture table that pins the
state machine so a test suite can lock it down, and the rule for what
content belongs inside the block versus in a tool-owned file it points
at.

## Migrating from whole-file overwrite {#migrating}

The hardest case is not a new tool — it is an existing one that has
been rewriting a whole file for releases, and now must become a
co-tenant without a destructive flag day. The migration has three
moving parts.

### Detect the legacy shape {#detect}

A file written by the old whole-file path carries **no markers** — the
tool never wrote any. So a plain *absent* classification cannot tell a
legacy file apart from a genuinely fresh host file. Distinguish them by
recognising the old generated content:

- If the entire file matches the tool's **known old generated form**
  (match the exact header string the old version wrote — precise, so a
  false positive cannot destroy a hand-authored file), the content
  *was* the tool's. Replace it with a clean file containing just the
  block; nothing is lost.
- Otherwise, take the ordinary **absent → create** path: append a
  block, preserve every other byte. A file the tool did not generate is
  a file with a co-tenant, even if that co-tenant is the user's past
  self.

Match the old form by an **exact** string, never a loose heuristic:
here a false positive means wrongly deleting a file a human wrote by
hand, which is the exact disaster the whole practice exists to prevent.

### One-time, consent-gated conversion {#conversion}

Do the legacy-to-block conversion **once**, and gate it on explicit
consent when there is any doubt the file is purely tool-owned. A silent
in-place conversion is the same trust violation as the overwrite it
replaces. Concretely:

- On first run of the block-aware version, if the file is the known
  old form, convert and print one line saying so.
- If the file has drifted from the known form — a human touched it —
  do **not** guess. Take the append path, or stop and ask, depending on
  how destructive a wrong guess would be. When in doubt, append: a
  spurious extra block is a cosmetic annoyance the user can delete; a
  deleted hand-edit is unrecoverable.

### The changelog note {#changelog}

A tool that starts owning a block instead of a file has changed its
contract with every host file on every user's machine. Say so in the
release notes, in one plain sentence: *"toolname now writes into a
delimited `<toolname>` block instead of replacing the whole file; your
own edits outside the block are preserved from this release on."* Users
who hand-edited the file around the tool need to know the hazard is
gone; users who scripted around the old whole-file behaviour need to
know it changed.

## Testing the state machine {#testing}

The classifier is the safety-critical part: it decides whether a write
is a splice, an append, or a hard stop. Pin every case with a fixture.
Feed each fixture file to the classifier and assert the verdict.

| Fixture | Marker content | Expected verdict |
|---------|---------------|------------------|
| **Absent** | No markers anywhere | `absent` → create |
| **Present** | One opener, then one closer, in order | `present` → splice |
| **Moved** | One ordered pair, but at the top of the file, preceded by user text | `present` → splice **in place** (never relocate) |
| **Duplicated opener** | Two openers, one closer | `malformed` → hard stop |
| **Duplicated closer** | One opener, two closers | `malformed` → hard stop |
| **Reversed order** | A closer appears before its opener | `malformed` → hard stop |
| **One marker only** | An opener with no closer (or vice versa) | `malformed` → hard stop |
| **Nested** | An opener, another opener, then two closers | `malformed` → hard stop |
| **Empty body** | One ordered pair with nothing between | `present` → splice (a valid empty block) |
| **Byte-identical** | Present, and new body equals old body | `present` → **no write** |

Two assertions deserve their own tests beyond the verdict:

- **Preservation.** After a splice, every byte outside the markers is
  identical to the input — assert on the full file, not just the block.
- **Clean removal.** After a remove, the file equals what it was before
  the block was ever created, separation blank line included — a
  round-trip create-then-remove is the identity function on the
  surrounding content.

The moved and nested rows are the ones naive implementations get
wrong: *moved* tempts a tool to "put the block back where it belongs"
(never — position is the user's), and *nested* tempts a tool to match
the outer pair and proceed (never — anything but one clean ordered
pair is malformed).

## What belongs inside the block {#what-belongs}

Keep the block **small, stable, and pointer-like**. It is a window
onto the file every reader already opens; it is not storage.

- **Good block content:** a few lines that rarely change — a redirect
  ("read the boot manifest"), a short set of exports, a single source
  line, a pointer to where the real payload lives.
- **Bad block content:** a large, frequently regenerated payload — a
  hundred lines of generated config, an embedded database, anything
  that produces a big diff on every run.

When the payload is large, put it in a **tool-owned file** — a file
with a co-tenant of exactly zero, safely under whole-file ownership
(see [`rejected-designs.md`](rejected-designs.md#whole-file)) — and let
the block hold only a pointer to it:

```
<toolname>
<!-- Generated by toolname — do not edit; pointer only. -->
source /opt/toolname/generated.sh   # the large payload lives here
</toolname>
```

This keeps the shared file's diffs small and legible — the block
changes only when the pointer changes — while the volatile payload
churns freely in a file no human shares. The block is the polite
handshake in the shared space; the payload lives in the tool's own
room.

## Summary {#summary}

- Migrate by recognising the exact old generated form: known form →
  convert once, consent-gated; anything else → append and preserve.
  Match by exact string, never a heuristic.
- Announce the contract change in one changelog sentence.
- Pin the state machine with fixtures: absent, present, moved,
  duplicated / reversed / lone / nested markers, empty body,
  byte-identical. Test preservation and clean removal separately.
- Keep the block small and pointer-like; large payloads live in a
  tool-owned file the block points at.
