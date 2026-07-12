# Rejected designs {#root}

**Scope of this document.** Four designs that look reasonable and are
wrong, each with its full reasoning, plus the exact hard-stop drill
the tool runs when it meets a malformed block. These are the shortcuts
every author of a file-writing tool is tempted by; catalogue them once
so the temptation is answered before it costs a user's data.

## A sidecar file instead of a block {#sidecar}

*The design.* Do not touch the host file at all. Write the tool's
content into a companion file next to it — `.tool/redirect`,
`CLAUDE.local.md`, `~/.toolrc` — and point the real consumer at the
sidecar.

*Why it is rejected.* The entire value of a host file like
`CLAUDE.md`, `~/.bashrc`, or `ssh_config` is that its consumer **already
reads it, with zero configuration**. That is the property you are
buying by writing there. A sidecar the consumer does not natively read
forfeits exactly that property: now something must be taught to read
the sidecar, and that teaching step is one more thing to install, get
wrong, and drift. Users read the host file — it is where they look for
what is in effect. Content exiled to a sidecar is content nobody sees
until it misbehaves. The sidecar trades a solved problem (write
politely into the file everyone reads) for an unsolved one (make
everyone read a new file).

A sidecar *is* the right answer for large payloads — but as a
complement to the block, not a replacement. See
[`adoption-guide.md`](adoption-guide.md#what-belongs) on keeping the
block small and pointing it at tool-owned files.

## Model-based or heuristic block detection {#model-detection}

*The design.* Instead of exact markers, find the block by
intelligence: "locate the region that looks like the tool's output,"
possibly by asking an LLM, possibly by a fuzzy match against the last
thing written.

*Why it is rejected.* The region gates a **mutating write** to a file
full of someone else's data. The gate on a destructive operation must
be **deterministic** — the same input yields the same region, every
run, on every machine, forever. A model or a heuristic is
nondeterministic by construction: it returns "probably here," and a
"probably" that is wrong once has overwritten content that does not
belong to the tool. There is no acceptable false-positive rate for
"which bytes may I destroy." A plain line-anchored byte scan for a
unique marker is not a lesser version of smart detection; it is the
only correct design, because it is the only one whose answer you can
prove before you write. Determinism is not a performance choice here —
it is the safety property.

## Auto-repairing a malformed block {#auto-repair}

*The design.* The file has two opening markers, or an opener with no
closer. Be helpful: delete the surplus marker, keep the first block,
and proceed.

*Why it is rejected.* A malformed block is **evidence** — of a failed
previous write, a bad merge, a hand-edit that went wrong, or two tool
versions disagreeing. Auto-repair destroys that evidence before a
human can read it, and the "repair" is a guess about which of two
regions is canonical. Guess wrong and you have deleted the block the
user actually wanted and kept the stale one — silently, in the name of
being helpful. The tool has no basis for the guess: nothing in the
file says which duplicate is intended. Worse, user content may have
drifted *between* the stray markers, so deleting a marker can delete
the user's own lines with it. A malformed block is exactly the case
where the tool must stop and defer to the one party who knows what was
meant. Hard stop, precise report, human decides (§drill).

## Whole-file ownership {#whole-file}

*The design.* Skip blocks entirely: the tool owns the whole file and
regenerates it on every run. Simple, and no marker machinery.

*Why it is mostly rejected — and the narrow case where it is fine.*
Whole-file regeneration is acceptable **only** when the file is 100%
tool-owned and marked as such — a generated `.lock` file, a
`tool-cache.json`, a file whose first line says "generated file — do
not edit, your changes will be lost." In that case there is no
co-tenant, so there is nothing to protect.

The trap is that ownership is not a decision the tool gets to keep.
The moment a human edits the file — and they will, if it is readable
and sits somewhere they look — you have a co-tenant, and whole-file
regeneration silently deletes their edit on the next run. So the test
is not "did I intend to own this file" but "can anyone else plausibly
write here." If yes, you are a co-tenant and you own a block, not the
file. Reserve whole-file ownership for files that are unmistakably,
permanently machine-only, and mark them loudly so no human mistakes
them for editable.

## The malformed-state drill {#drill}

When classification (see
[`MANAGED-BLOCKS-PROTOCOL.md`](MANAGED-BLOCKS-PROTOCOL.md#state-machine))
returns *malformed*, the tool aborts the whole operation and prints a
report with three parts. Nothing is written.

| Part | Content |
|------|---------|
| **What I found** | The file path and the exact defect, with line numbers: "two `<toolname>` opening markers, at lines 12 and 40." |
| **What I expected** | The well-formed shape: "either zero markers, or exactly one `<toolname>` … `</toolname>` pair in order." |
| **How to unblock** | The precise human action: "keep the block you want, delete the other opening marker and its closing marker, then re-run." |

A worked message:

```
error: managed block in CLAUDE.md is malformed — aborting, nothing written.

  found:    two opening markers <toolname> at lines 12 and 40
            (expected exactly one)
  expected: zero markers, or one <toolname> … </toolname> pair in order
  fix:      decide which block is canonical, delete the other opener
            and its matching closer, then re-run `toolname install`

No files were changed.
```

Three properties make this drill correct:

- **It changes nothing.** The file is left exactly as found, so the
  human debugs the real state, not a half-repaired one.
- **It is specific.** Line numbers and the exact defect mean the human
  fixes it in seconds, without opening the tool's source.
- **It names the unblocking action.** The report ends by telling the
  human precisely what to do, so a hard stop is a thirty-second detour,
  not a support ticket.

## Summary {#summary}

- **Sidecar** — forfeits the one property that made the host file
  worth writing to: its consumer already reads it.
- **Model / heuristic detection** — a nondeterministic gate on a
  destructive write; the region you may overwrite must be provable
  before you write.
- **Auto-repair** — destroys evidence and guesses which region is
  canonical; may delete user content that drifted between markers.
- **Whole-file ownership** — fine only for permanently machine-only
  files, marked loudly; the moment a human edits it, you are a
  co-tenant and owe them a block.
- **Malformed → the drill**: change nothing, report what was found vs
  expected, name the exact human action that unblocks.
