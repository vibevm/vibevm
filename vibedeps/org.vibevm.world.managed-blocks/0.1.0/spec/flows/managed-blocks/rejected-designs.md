# Rejected designs {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** Four designs that look reasonable and are
wrong, each with its full reasoning, plus the exact hard-stop drill
the tool runs when it meets a malformed block. @status:impl/done

@fact:catalogue-the-temptations-once These are the shortcuts
every author of a file-writing tool is tempted by; catalogue them once
so the temptation is answered before it costs a user's data. @status:spec/done

## A sidecar file instead of a block {#sidecar}

@fact:SIDECAR-DESIGN-DO-NOT-TOUCH-THE-HOST-FILE *The design.* Do not touch the host file at all. @status:spec/done

@fact:SIDECAR-DESIGN-WRITE-INTO-A-COMPANION-FILE Write the tool's
content into a companion file next to it — `.tool/redirect`,
`CLAUDE.local.md`, `~/.toolrc` — and point the real consumer at the
sidecar. @status:spec/done

@fact:sidecar-why-the-host-files-value-is-zero-config-reading *Why it is rejected.* The entire value of a host file like
`CLAUDE.md`, `~/.bashrc`, or `ssh_config` is that its consumer **already
reads it, with zero configuration**. @status:spec/done

@fact:sidecar-that-is-the-property-you-are-buying That is the property you are
buying by writing there. @status:spec/done

@fact:SIDECAR-FORFEITS-THE-ZERO-CONFIGURATION-PROPERTY A sidecar the consumer does not natively read
forfeits exactly that property: now something must be taught to read
the sidecar, and that teaching step is one more thing to install, get
wrong, and drift. @status:spec/done

@fact:sidecar-users-read-the-host-file Users read the host file — it is where they look for
what is in effect. @status:spec/done

@fact:sidecar-content-exiled-is-content-nobody-sees Content exiled to a sidecar is content nobody sees
until it misbehaves. @status:spec/done

@fact:SIDECAR-TRADES-A-SOLVED-PROBLEM-FOR-AN-UNSOLVED-ONE The sidecar trades a solved problem (write
politely into the file everyone reads) for an unsolved one (make
everyone read a new file). @status:spec/done

@fact:A-SIDECAR-IS-RIGHT-FOR-LARGE-PAYLOADS-AS-A-COMPLEMENT A sidecar *is* the right answer for large payloads — but as a
complement to the block, not a replacement. @status:impl/done

@fact:adoption-guide-pointer See
[`adoption-guide.md`](adoption-guide.md#what-belongs) on keeping the
block small and pointing it at tool-owned files. @status:impl/done

## Model-based or heuristic block detection {#model-detection}

@fact:MODEL-DETECTION-DESIGN-FIND-THE-BLOCK-BY-INTELLIGENCE *The design.* Instead of exact markers, find the block by
intelligence: "locate the region that looks like the tool's output,"
possibly by asking an LLM, possibly by a fuzzy match against the last
thing written. @status:spec/done

@fact:model-detection-why-the-region-gates-a-mutating-write *Why it is rejected.* The region gates a **mutating write** to a file
full of someone else's data. @status:spec/done

@fact:THE-GATE-ON-A-DESTRUCTIVE-OPERATION-MUST-BE-DETERMINISTIC The gate on a destructive operation must
be **deterministic** — the same input yields the same region, every
run, on every machine, forever. @status:spec/done

@fact:a-model-is-nondeterministic-by-construction A model or a heuristic is
nondeterministic by construction: it returns "probably here," and a
"probably" that is wrong once has overwritten content that does not
belong to the tool. @status:spec/done

@fact:no-acceptable-false-positive-rate-for-which-bytes-to-destroy There is no acceptable false-positive rate for
"which bytes may I destroy." @status:spec/done

@fact:A-BYTE-SCAN-IS-THE-ONLY-CORRECT-DESIGN A plain line-anchored byte scan for a
unique marker is not a lesser version of smart detection; it is the
only correct design, because it is the only one whose answer you can
prove before you write. @status:spec/done

@fact:DETERMINISM-IS-THE-SAFETY-PROPERTY Determinism is not a performance choice here —
it is the safety property. @status:spec/done

## Auto-repairing a malformed block {#auto-repair}

@fact:AUTO-REPAIR-DESIGN-THE-FILE-IS-MALFORMED *The design.* The file has two opening markers, or an opener with no
closer. @status:spec/done

@fact:AUTO-REPAIR-DESIGN-BE-HELPFUL-AND-PROCEED Be helpful: delete the surplus marker, keep the first block,
and proceed. @status:spec/done

@fact:A-MALFORMED-BLOCK-IS-EVIDENCE *Why it is rejected.* A malformed block is **evidence** — of a failed
previous write, a bad merge, a hand-edit that went wrong, or two tool
versions disagreeing. @status:spec/done

@fact:auto-repair-destroys-evidence-and-guesses Auto-repair destroys that evidence before a
human can read it, and the "repair" is a guess about which of two
regions is canonical. @status:spec/done

@fact:auto-repair-guess-wrong-and-you-delete-the-wanted-block Guess wrong and you have deleted the block the
user actually wanted and kept the stale one — silently, in the name of
being helpful. @status:spec/done

@fact:the-tool-has-no-basis-for-the-guess The tool has no basis for the guess: nothing in the
file says which duplicate is intended. @status:spec/done

@fact:user-content-may-have-drifted-between-the-stray-markers Worse, user content may have
drifted *between* the stray markers, so deleting a marker can delete
the user's own lines with it. @status:spec/done

@fact:A-MALFORMED-BLOCK-IS-WHERE-THE-TOOL-MUST-STOP-AND-DEFER A malformed block is exactly the case
where the tool must stop and defer to the one party who knows what was
meant. @status:spec/done

@fact:HARD-STOP-PRECISE-REPORT-HUMAN-DECIDES Hard stop, precise report, human decides (§drill). @status:impl/done

## Whole-file ownership {#whole-file}

@fact:WHOLE-FILE-DESIGN-THE-TOOL-OWNS-THE-WHOLE-FILE *The design.* Skip blocks entirely: the tool owns the whole file and
regenerates it on every run. @status:spec/done

@fact:whole-file-design-simple-and-no-marker-machinery Simple, and no marker machinery. @status:spec/done

@fact:WHOLE-FILE-REGENERATION-IS-ACCEPTABLE-ONLY-WHEN-TOOL-OWNED *Why it is mostly rejected — and the narrow case where it is fine.*
Whole-file regeneration is acceptable **only** when the file is 100%
tool-owned and marked as such — a generated `.lock` file, a
`tool-cache.json`, a file whose first line says "generated file — do
not edit, your changes will be lost." @status:impl/done

@fact:in-that-case-there-is-no-co-tenant-to-protect In that case there is no
co-tenant, so there is nothing to protect. @status:spec/done

@fact:OWNERSHIP-IS-NOT-A-DECISION-THE-TOOL-GETS-TO-KEEP The trap is that ownership is not a decision the tool gets to keep. @status:spec/done

@fact:a-human-edit-makes-you-a-co-tenant The moment a human edits the file — and they will, if it is readable
and sits somewhere they look — you have a co-tenant, and whole-file
regeneration silently deletes their edit on the next run. @status:spec/done

@fact:THE-TEST-IS-CAN-ANYONE-ELSE-PLAUSIBLY-WRITE-HERE So the test
is not "did I intend to own this file" but "can anyone else plausibly
write here." @status:impl/done

@fact:IF-YES-YOU-OWN-A-BLOCK-NOT-THE-FILE If yes, you are a co-tenant and you own a block, not the
file. @status:impl/done

@fact:RESERVE-WHOLE-FILE-OWNERSHIP-AND-MARK-IT-LOUDLY Reserve whole-file ownership for files that are unmistakably,
permanently machine-only, and mark them loudly so no human mistakes
them for editable. @status:impl/done

## The malformed-state drill {#drill}

@fact:ON-MALFORMED-THE-TOOL-ABORTS-AND-PRINTS-A-THREE-PART-REPORT When classification (see
[`MANAGED-BLOCKS-PROTOCOL.md`](MANAGED-BLOCKS-PROTOCOL.md#state-machine))
returns *malformed*, the tool aborts the whole operation and prints a
report with three parts. @status:impl/done

@fact:NOTHING-IS-WRITTEN Nothing is written. @status:impl/done

| Part | Content |
|------|---------|
| @fact:ROW-REPORT-WHAT-I-FOUND **What I found** @status:impl/done | The file path and the exact defect, with line numbers: "two `<toolname>` opening markers, at lines 12 and 40." @status:impl/done |
| @fact:ROW-REPORT-WHAT-I-EXPECTED **What I expected** @status:impl/done | The well-formed shape: "either zero markers, or exactly one `<toolname>` … `</toolname>` pair in order." @status:impl/done |
| @fact:ROW-REPORT-HOW-TO-UNBLOCK **How to unblock** @status:impl/done | The precise human action: "keep the block you want, delete the other opening marker and its closing marker, then re-run." @status:impl/done |

@fact:a-worked-message A worked message: @status:impl/done

```
error: managed block in CLAUDE.md is malformed — aborting, nothing written.

  found:    two opening markers <toolname> at lines 12 and 40
            (expected exactly one)
  expected: zero markers, or one <toolname> … </toolname> pair in order
  fix:      decide which block is canonical, delete the other opener
            and its matching closer, then re-run `toolname install`

No files were changed.
```

@fact:THREE-PROPERTIES-MAKE-THIS-DRILL-CORRECT Three properties make this drill correct: @status:impl/done

- @fact:DRILL-IT-CHANGES-NOTHING **It changes nothing.** The file is left exactly as found, so the
  human debugs the real state, not a half-repaired one. @status:impl/done
- @fact:DRILL-IT-IS-SPECIFIC **It is specific.** Line numbers and the exact defect mean the human
  fixes it in seconds, without opening the tool's source. @status:impl/done
- @fact:DRILL-IT-NAMES-THE-UNBLOCKING-ACTION **It names the unblocking action.** The report ends by telling the
  human precisely what to do, so a hard stop is a thirty-second detour,
  not a support ticket. @status:impl/done

## Summary {#summary}

- @fact:SUM-SIDECAR-FORFEITS-THE-HOST-FILES-ONE-PROPERTY **Sidecar** — forfeits the one property that made the host file
  worth writing to: its consumer already reads it. @status:spec/done
- @fact:SUM-MODEL-DETECTION-IS-A-NONDETERMINISTIC-GATE **Model / heuristic detection** — a nondeterministic gate on a
  destructive write; the region you may overwrite must be provable
  before you write. @status:spec/done
- @fact:SUM-AUTO-REPAIR-DESTROYS-EVIDENCE-AND-GUESSES **Auto-repair** — destroys evidence and guesses which region is
  canonical; may delete user content that drifted between markers. @status:spec/done
- @fact:SUM-WHOLE-FILE-OWNERSHIP-IS-FOR-MACHINE-ONLY-FILES **Whole-file ownership** — fine only for permanently machine-only
  files, marked loudly; the moment a human edits it, you are a
  co-tenant and owe them a block. @status:impl/done
- @fact:SUM-MALFORMED-GOES-TO-THE-DRILL **Malformed → the drill**: change nothing, report what was found vs
  expected, name the exact human action that unblocks. @status:impl/done
