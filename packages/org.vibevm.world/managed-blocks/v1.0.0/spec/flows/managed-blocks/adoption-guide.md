# Adoption guide {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** The practical work of shipping managed
blocks: migrating an existing tool that overwrites whole files onto a
block without eating its users' data, a fixture table that pins the
state machine so a test suite can lock it down, and the rule for what
content belongs inside the block versus in a tool-owned file it points
at. @status:impl/done

## Migrating from whole-file overwrite {#migrating}

@fact:THE-HARDEST-CASE-IS-AN-EXISTING-OVERWRITING-TOOL The hardest case is not a new tool — it is an existing one that has
been rewriting a whole file for releases, and now must become a
co-tenant without a destructive flag day. @status:spec/done

@fact:THE-MIGRATION-HAS-THREE-MOVING-PARTS The migration has three
moving parts. @status:impl/done

### Detect the legacy shape {#detect}

@fact:A-LEGACY-FILE-CARRIES-NO-MARKERS A file written by the old whole-file path carries **no markers** — the
tool never wrote any. @status:impl/done

@fact:ABSENT-CANNOT-TELL-LEGACY-FROM-FRESH So a plain *absent* classification cannot tell a
legacy file apart from a genuinely fresh host file. @status:impl/done

@fact:DISTINGUISH-THEM-BY-RECOGNISING-THE-OLD-GENERATED-CONTENT Distinguish them by
recognising the old generated content: @status:impl/done

- @fact:DETECT-KNOWN-OLD-FORM-REPLACE-WITH-A-CLEAN-BLOCK-FILE If the entire file matches the tool's **known old generated form**
  (match the exact header string the old version wrote — precise, so a
  false positive cannot destroy a hand-authored file), the content
  *was* the tool's. Replace it with a clean file containing just the
  block; nothing is lost. @status:impl/done
- @fact:DETECT-OTHERWISE-TAKE-THE-ABSENT-CREATE-PATH Otherwise, take the ordinary **absent → create** path: append a
  block, preserve every other byte. A file the tool did not generate is
  a file with a co-tenant, even if that co-tenant is the user's past
  self. @status:impl/done

@fact:MATCH-THE-OLD-FORM-BY-AN-EXACT-STRING Match the old form by an **exact** string, never a loose heuristic:
here a false positive means wrongly deleting a file a human wrote by
hand, which is the exact disaster the whole practice exists to prevent. @status:impl/done

### One-time, consent-gated conversion {#conversion}

@fact:CONVERT-ONCE-AND-GATE-IT-ON-EXPLICIT-CONSENT Do the legacy-to-block conversion **once**, and gate it on explicit
consent when there is any doubt the file is purely tool-owned. @status:impl/done

@fact:A-SILENT-CONVERSION-IS-THE-SAME-TRUST-VIOLATION A silent
in-place conversion is the same trust violation as the overwrite it
replaces. Concretely: @status:impl/done

- @fact:CONVERSION-ON-FIRST-RUN-CONVERT-AND-PRINT-ONE-LINE On first run of the block-aware version, if the file is the known
  old form, convert and print one line saying so. @status:impl/done
- @fact:CONVERSION-IF-THE-FILE-HAS-DRIFTED-DO-NOT-GUESS If the file has drifted from the known form — a human touched it —
  do **not** guess. Take the append path, or stop and ask, depending on
  how destructive a wrong guess would be. When in doubt, append: a
  spurious extra block is a cosmetic annoyance the user can delete; a
  deleted hand-edit is unrecoverable. @status:impl/done

### The changelog note {#changelog}

@fact:OWNING-A-BLOCK-INSTEAD-OF-A-FILE-CHANGES-THE-CONTRACT A tool that starts owning a block instead of a file has changed its
contract with every host file on every user's machine. @status:impl/done

@fact:SAY-SO-IN-THE-RELEASE-NOTES-IN-ONE-PLAIN-SENTENCE Say so in the
release notes, in one plain sentence: *"toolname now writes into a
delimited `<toolname>` block instead of replacing the whole file; your
own edits outside the block are preserved from this release on."* @status:impl/done

@fact:both-kinds-of-user-need-to-know Users
who hand-edited the file around the tool need to know the hazard is
gone; users who scripted around the old whole-file behaviour need to
know it changed. @status:spec/done

## Testing the state machine {#testing}

@fact:THE-CLASSIFIER-IS-THE-SAFETY-CRITICAL-PART The classifier is the safety-critical part: it decides whether a write
is a splice, an append, or a hard stop. @status:impl/done

@fact:PIN-EVERY-CASE-WITH-A-FIXTURE Pin every case with a fixture. @status:impl/done

@fact:FEED-EACH-FIXTURE-TO-THE-CLASSIFIER-AND-ASSERT-THE-VERDICT Feed each fixture file to the classifier and assert the verdict. @status:impl/done

| Fixture | Marker content | Expected verdict |
|---------|---------------|------------------|
| @fact:ROW-FIXTURE-ABSENT **Absent** @status:impl/done | No markers anywhere @status:impl/done | `absent` → create @status:impl/done |
| @fact:ROW-FIXTURE-PRESENT **Present** @status:impl/done | One opener, then one closer, in order @status:impl/done | `present` → splice @status:impl/done |
| @fact:ROW-FIXTURE-MOVED **Moved** @status:impl/done | One ordered pair, but at the top of the file, preceded by user text @status:impl/done | `present` → splice **in place** (never relocate) @status:impl/done |
| @fact:ROW-FIXTURE-DUPLICATED-OPENER **Duplicated opener** @status:impl/done | Two openers, one closer @status:impl/done | `malformed` → hard stop @status:impl/done |
| @fact:ROW-FIXTURE-DUPLICATED-CLOSER **Duplicated closer** @status:impl/done | One opener, two closers @status:impl/done | `malformed` → hard stop @status:impl/done |
| @fact:ROW-FIXTURE-REVERSED-ORDER **Reversed order** @status:impl/done | A closer appears before its opener @status:impl/done | `malformed` → hard stop @status:impl/done |
| @fact:ROW-FIXTURE-ONE-MARKER-ONLY **One marker only** @status:impl/done | An opener with no closer (or vice versa) @status:impl/done | `malformed` → hard stop @status:impl/done |
| @fact:ROW-FIXTURE-NESTED **Nested** @status:impl/done | An opener, another opener, then two closers @status:impl/done | `malformed` → hard stop @status:impl/done |
| @fact:ROW-FIXTURE-EMPTY-BODY **Empty body** @status:impl/done | One ordered pair with nothing between @status:impl/done | `present` → splice (a valid empty block) @status:impl/done |
| @fact:ROW-FIXTURE-BYTE-IDENTICAL **Byte-identical** @status:impl/done | Present, and new body equals old body @status:impl/done | `present` → **no write** @status:impl/done |

@fact:TWO-ASSERTIONS-DESERVE-THEIR-OWN-TESTS Two assertions deserve their own tests beyond the verdict: @status:impl/done

- @fact:ASSERT-PRESERVATION-OF-EVERY-BYTE-OUTSIDE-THE-MARKERS **Preservation.** After a splice, every byte outside the markers is
  identical to the input — assert on the full file, not just the block. @status:impl/done
- @fact:ASSERT-CLEAN-REMOVAL-IS-THE-IDENTITY-FUNCTION **Clean removal.** After a remove, the file equals what it was before
  the block was ever created, separation blank line included — a
  round-trip create-then-remove is the identity function on the
  surrounding content. @status:impl/done

@fact:THE-MOVED-AND-NESTED-ROWS-ARE-THE-ONES-NAIVE-TOOLS-GET-WRONG The moved and nested rows are the ones naive implementations get
wrong: *moved* tempts a tool to "put the block back where it belongs"
(never — position is the user's), and *nested* tempts a tool to match
the outer pair and proceed (never — anything but one clean ordered
pair is malformed). @status:impl/done

## What belongs inside the block {#what-belongs}

@fact:KEEP-THE-BLOCK-SMALL-STABLE-AND-POINTER-LIKE Keep the block **small, stable, and pointer-like**. @status:impl/done

@fact:the-block-is-a-window-not-storage It is a window
onto the file every reader already opens; it is not storage. @status:spec/done

- @fact:GOOD-BLOCK-CONTENT **Good block content:** a few lines that rarely change — a redirect
  ("read the boot manifest"), a short set of exports, a single source
  line, a pointer to where the real payload lives. @status:impl/done
- @fact:BAD-BLOCK-CONTENT **Bad block content:** a large, frequently regenerated payload — a
  hundred lines of generated config, an embedded database, anything
  that produces a big diff on every run. @status:impl/done

@fact:PUT-A-LARGE-PAYLOAD-IN-A-TOOL-OWNED-FILE When the payload is large, put it in a **tool-owned file** — a file
with a co-tenant of exactly zero, safely under whole-file ownership
(see [`rejected-designs.md`](rejected-designs.md#whole-file)) — and let
the block hold only a pointer to it: @status:impl/done

```
<toolname>
<!-- Generated by toolname — do not edit; pointer only. -->
source /opt/toolname/generated.sh   # the large payload lives here
</toolname>
```

@fact:THE-POINTER-KEEPS-THE-SHARED-FILES-DIFFS-SMALL This keeps the shared file's diffs small and legible — the block
changes only when the pointer changes — while the volatile payload
churns freely in a file no human shares. @status:impl/done

@fact:the-block-is-the-polite-handshake The block is the polite
handshake in the shared space; the payload lives in the tool's own
room. @status:spec/done

## Summary {#summary}

- @fact:SUM-MIGRATE-BY-RECOGNISING-THE-EXACT-OLD-FORM Migrate by recognising the exact old generated form: known form →
  convert once, consent-gated; anything else → append and preserve.
  Match by exact string, never a heuristic. @status:impl/done
- @fact:SUM-ANNOUNCE-THE-CONTRACT-CHANGE Announce the contract change in one changelog sentence. @status:impl/done
- @fact:SUM-PIN-THE-STATE-MACHINE-WITH-FIXTURES Pin the state machine with fixtures: absent, present, moved,
  duplicated / reversed / lone / nested markers, empty body,
  byte-identical. Test preservation and clean removal separately. @status:impl/done
- @fact:SUM-KEEP-THE-BLOCK-SMALL-AND-POINTER-LIKE Keep the block small and pointer-like; large payloads live in a
  tool-owned file the block points at. @status:impl/done
