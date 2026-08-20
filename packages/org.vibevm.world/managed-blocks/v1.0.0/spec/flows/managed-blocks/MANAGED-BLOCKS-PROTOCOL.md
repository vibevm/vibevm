# Managed Blocks Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines *how* a tool writes
into a file it does not own: the co-tenant law that bounds it to one
delimited region, the marker design that makes that region findable
by a deterministic scan, the well-formedness state machine that
classifies a file before any mutation, the three verbs the tool is
allowed (create / update / remove), and how two tools cohabit one
file without ever colliding. @status:impl/done

## The problem: a tool clobbering a shared file {#problem}

@fact:MANY-TOOLS-WRITE-INTO-FILES-THEY-DO-NOT-SOLELY-OWN Many tools need to write into a file they did not create and do not
solely own. @status:spec/done

@fact:example-a-shell-installer-appends-to-bashrc A shell installer appends to `~/.bashrc`. @status:spec/done

@fact:example-an-agent-framework-wants-a-line-in-claude-md An agent
framework wants a line in `CLAUDE.md`. @status:spec/done

@fact:example-a-deploy-tool-edits-an-ssh-config A deploy tool edits an
`ssh_config`. @status:spec/done

@fact:example-a-linter-drops-config-into-package-json A linter drops config into a shared `package.json`. @status:spec/done

@fact:EACH-OF-THESE-FILES-ALREADY-CARRIES-OTHER-CONTENT Each of these files already carries content from other parties — the
user by hand, and often one or more *other* tools. @status:spec/done

@fact:THE-NAIVE-IMPLEMENTATION-IS-A-WHOLE-FILE-REWRITE The naive implementation is a whole-file rewrite: read a template,
fill it in, `write()` the file. @status:spec/done

@fact:IT-IS-EASY-AND-IT-IS-A-DATA-LOSS-EVENT It is easy, and it is a **data-loss
event**. @status:spec/done

@fact:THE-FIRST-INSTALL-DESTROYS-EVERY-OTHER-TENANTS-BYTES The first install into any project with a non-trivial host
file silently destroys every byte the user and every other tool put
there — and it destroys precisely the file a person is most likely
to have invested in by hand. @status:spec/done

@fact:the-discipline-is-old-and-universal The discipline that avoids this is old and universal: `ssh`,
shell-rc installers, and countless config tools already use it. @status:spec/done

@fact:THE-TOOL-OWNS-ONE-DELIMITED-MACHINE-FINDABLE-REGION The
tool owns one small, clearly delimited, machine-findable region and
never touches a byte outside it. @status:impl/done

@fact:the-host-file-stops-being-the-tools-file The host file stops being "the
tool's file." @status:impl/done

@fact:IT-BECOMES-A-SHARED-FILE-WITH-THE-TOOLS-BLOCK-IN-IT It becomes **a shared file with the tool's block in
it**. @status:impl/done

## The co-tenant law {#co-tenant}

> @fact:THE-CO-TENANT-LAW Own exactly one delimited block. Never touch a byte outside it. @status:impl/done

@fact:A-GOOD-CO-TENANT-WRITES-INTO-ITS-OWN-PEN A good co-tenant writes into its own pen and leaves the rest of the
file to whoever else shares it. Stated operationally: @status:impl/done

- @fact:TOOL-REWRITES-ONLY-BETWEEN-ITS-OWN-MARKERS The tool reads and rewrites only the content **between** its own
  opening and closing markers. @status:impl/done
- @fact:EVERY-BYTE-OUTSIDE-THE-BLOCK-IS-PRESERVED-VERBATIM Every byte outside that block is another tenant's property,
  **preserved verbatim** across every operation the tool performs —
  install, update, uninstall, reconfigure. @status:impl/done
- @fact:THE-TOOL-ASSUMES-IT-IS-NEVER-THE-ONLY-TENANT The tool assumes it is **never the only tenant**. Even in an empty
  project today, a human or a second tool may share the file
  tomorrow. @status:impl/done

@fact:the-same-rule-a-c-include-obeys This is the same rule a C `#include` obeys: pulling in a dependency
must never modify the including file's own authored content. @status:spec/done

## Marker design {#markers}

@fact:THE-BLOCK-IS-BOUNDED-BY-AN-OPENING-AND-A-CLOSING-MARKER The block is bounded by an opening marker and a closing marker, each
alone on its own line. @status:impl/done

@fact:FOUR-PROPERTIES-ARE-NON-NEGOTIABLE Four properties are non-negotiable. @status:impl/done

| Property | Requirement |
|----------|-------------|
| @fact:ROW-MARKER-UNIQUE **Unique** @status:impl/done | The marker string must not plausibly occur in host content. Namespace it with the tool's name — `# >>> toolname >>>`, `<toolname>`, `# BEGIN toolname`. A generic `# BEGIN` will collide. @status:impl/done |
| @fact:ROW-MARKER-GREPPABLE **Greppable** @status:impl/done | Locatable by a plain line-anchored text scan — no format parser, no model. See the next section on why. @status:impl/done |
| @fact:ROW-MARKER-PAIRED **Paired** @status:impl/done | A distinct open and close, so a byte scan yields an unambiguous region, not a single sentinel whose end must be guessed. @status:impl/done |
| @fact:ROW-MARKER-SELF-DOCUMENTING **Self-documenting** @status:impl/done | The first line *inside* the block carries a do-not-edit notice, so a human reading the host file learns the region is managed without consulting docs. @status:impl/done |

@fact:a-worked-block-using-bare-tags A worked block, using bare tags (chosen here because the primary
reader is an LLM, to which a tag reads unambiguously): @status:impl/done

```
<toolname>
<!-- Generated by toolname — do not edit inside this block;
     it is rewritten on `toolname sync`. -->
... the managed body ...
</toolname>
```

@fact:COMMENT-STYLE-MARKERS-SUIT-AN-RC-FILE-OR-SSH-CONFIG Comment-style markers suit an rc file or ssh config equally well: @status:impl/done

```
# >>> toolname managed block >>>
# Do not edit between these markers; regenerated by `toolname apply`.
export PATH="$PATH:/opt/toolname/bin"
# <<< toolname managed block <<<
```

### Version the marker format itself {#marker-version}

@fact:THE-MARKER-SYNTAX-IS-A-WIRE-FORMAT The marker syntax is a wire format between your tool's past and
future selves. @status:impl/done

@fact:old-blocks-in-the-wild-still-carry-the-old-markers If you ever need to change it, old blocks in the wild
still carry the old markers. @status:spec/done

@fact:CARRY-A-VERSION-TOKEN-IN-THE-OPENING-MARKER Carry a small version token in the
opening marker (`<toolname v=1>`) or in the notice line, so a future
release can recognise and migrate a v1 block instead of appending a
duplicate v2 one beside it. @status:impl/done

@fact:DECIDE-THE-VERSION-BEFORE-THE-FIRST-RELEASE Decide this before the first release —
retrofitting a version onto an unversioned marker means one painful
generation of heuristics. @status:impl/done

## Well-formedness: absent, present, malformed {#state-machine}

@fact:THE-TOOL-CLASSIFIES-THE-HOST-FILE-BEFORE-WRITING Before writing, the tool classifies the host file by counting its
own markers. @status:impl/done

@fact:THERE-ARE-EXACTLY-THREE-STATES There are exactly three states. @status:impl/done

| State | Marker count | Allowed action |
|-------|-------------|----------------|
| @fact:ROW-STATE-ABSENT **Absent** @status:impl/done | Zero markers @status:impl/done | Create: append a fresh block (§verbs). @status:impl/done |
| @fact:ROW-STATE-PRESENT **Present** @status:impl/done | Exactly one opener, then exactly one closer, in order @status:impl/done | Update or remove the body between them. @status:impl/done |
| @fact:ROW-STATE-MALFORMED **Malformed** @status:impl/done | Anything else — two+ of either marker, an opener with no closer, a closer with no opener, or a closer before its opener @status:impl/done | **Hard stop.** Change nothing; report. @status:impl/done |

@fact:MALFORMED-IS-THE-LOAD-BEARING-STATE Malformed is the load-bearing state. @status:impl/done

@fact:A-MALFORMED-BLOCK-IS-ALWAYS-A-HUMANS-CALL A malformed managed block is
**always a human's call**: the tool never guesses which of two
blocks is canonical, never deletes a stray marker, never auto-repairs. @status:impl/done

@fact:IT-ABORTS-NAMES-THE-DEFECT-AND-WAITS It aborts the whole operation, names the file and the exact defect,
and waits. @status:impl/done

@fact:auto-repair-rationale-pointer The rationale is in
[`rejected-designs.md`](rejected-designs.md#auto-repair): auto-repair
destroys the evidence a human needs and may destroy user content
that drifted into the wrong side of a marker. @status:spec/done

## The three verbs {#verbs}

@fact:EVERYTHING-THE-TOOL-DOES-IS-ONE-OF-THREE-OPERATIONS Everything the tool does to the block is one of three operations. @status:impl/done

- @fact:VERB-CREATE **Create** — the file is *absent* of a block. Append the block at
  the **end of the file**, preceded by one blank line of separation.
  If the file itself does not exist, create it containing only the
  block. End-of-file is the humble default: the tool does not claim
  the attention-priority top of a shared file. @status:impl/done
- @fact:VERB-UPDATE **Update** — the file is *present*. Replace the content between the
  markers with freshly generated content. The markers themselves,
  and every byte outside them, are untouched. If the new body is
  byte-identical to the old, write nothing (§no-op). @status:impl/done
- @fact:VERB-REMOVE **Remove** — the file is *present* and the tool is uninstalling.
  Delete the block **and its separation** — the blank line the create
  step added — leaving the surrounding content byte-identical to what
  it was before the block ever existed. A remove that leaves a
  dangling blank line is a remove with a bug. @status:impl/done

### Placement belongs to the user {#placement}

@fact:THE-TOOL-DECIDES-THE-POSITION-EXACTLY-ONCE The tool decides the block's position **exactly once** — at create,
appended to the end. @status:impl/done

@fact:FROM-THEN-ON-THE-POSITION-IS-THE-USERS From then on the position is the **user's**. @status:impl/done

@fact:UPDATE-NEVER-RELOCATES-THE-MARKERS Update rewrites the body and never relocates the markers; if the user
moved the block to the top of the file, the tool splices it in place,
wherever they put it. @status:impl/done

@fact:honouring-position-lets-the-user-tune-the-weight Honouring that lets the user tune how strongly
the block weighs: at the top of an instruction file it reads as a
first, system-prompt-like instruction; at the bottom it is a sidecar. @status:spec/done

@fact:THE-TOOL-SUPPLIES-A-DEFAULT-AND-NEVER-OVERRIDES-THE-CHOICE The tool supplies a polite default and never overrides the choice. @status:impl/done

## Classify at plan time, not mid-write {#plan-time}

@fact:CLASSIFY-BEFORE-ANY-MUTATION-BEGINS Run the absent / present / malformed classification **before** any
mutation begins — in a planning pass that touches nothing. @status:impl/done

@fact:a-half-applied-change-has-no-clean-state-to-return-to A tool
that discovers a malformed block halfway through a multi-file write
has already half-applied its change: some files rewritten, one
unwritable, no clean state to return to. @status:spec/done

@fact:CLASSIFYING-UP-FRONT-FAILS-FAST-AND-CLEAN Classifying every target
file up front means a malformed block fails the operation **fast and
clean** — nothing is touched until every file is known to be
writable. @status:impl/done

@fact:THE-MUTATION-PHASE-SHOULD-NEVER-DISCOVER-A-SURPRISE The mutation phase should never discover a surprise. @status:impl/done

## The byte-identical no-op {#no-op}

@fact:DO-NOT-REWRITE-WHEN-THE-BODY-IS-BYTE-IDENTICAL If the freshly generated body equals the existing body, **do not
rewrite the file**. @status:impl/done

@fact:an-unconditional-write-churns-mtime-and-produces-a-no-op-diff An unconditional write churns the file's mtime
and, worse, produces a no-op diff in version control that a reviewer
must read to confirm it changes nothing. @status:spec/done

@fact:an-ignored-diff-is-where-a-real-change-hides A tool that writes on every
invocation trains its users to ignore its diffs — and an ignored diff
is where a real change hides. @status:spec/done

@fact:COMPARE-THEN-WRITE-ONLY-ON-A-DIFFERENCE Compare, then write only on a
difference. @status:impl/done

## Multi-tool cohabitation {#cohabitation}

@fact:THE-LAW-SCALES-BECAUSE-EACH-TOOL-SCANS-FOR-ITS-OWN-MARKERS The law scales to any number of tools because each tool scans only
for **its own** markers. @status:impl/done

@fact:two-tools-two-blocks-one-file Two tools, two blocks, one file: @status:impl/done

```
# Hand-written by the user — untouched by both tools.
alias gs='git status'

<toolA>
<!-- managed by toolA -->
source /opt/toolA/env.sh
</toolA>

# >>> toolB >>>
# managed by toolB
export TOOLB_HOME=/opt/toolB
# <<< toolB <<<
```

@fact:example-tool-a-finds-one-ordered-pair Tool A counts `<toolA>` / `</toolA>` and finds exactly one ordered
pair: *present*, splice its body. @status:impl/done

@fact:example-tool-b-markers-are-ordinary-host-content-to-tool-a Tool B's markers are, to tool A,
ordinary host content — outside its block, therefore preserved
verbatim. @status:impl/done

@fact:example-tool-b-sees-the-mirror-image Tool B sees the mirror image. @status:impl/done

@fact:NEITHER-TOOL-NEEDS-TO-KNOW-THE-OTHER-EXISTS Neither tool needs to know
the other exists; the unique-marker requirement (§markers) is exactly
what makes that independence hold. @status:impl/done

@fact:THE-USERS-HAND-WRITTEN-CONTENT-SURVIVES-BOTH-TOOLS The user's hand-written alias is
outside both blocks and survives every operation of both tools. @status:impl/done

@fact:MARKER-UNIQUENESS-PREVENTS-SILENT-MUTUAL-CORRUPTION The failure this prevents is the reason marker uniqueness is not
optional: if both tools used a generic `# BEGIN` / `# END`, each would
match the other's block, and the second tool to run would splice its
body into the first tool's region — silent mutual corruption. @status:impl/done

## Re-derive for your project {#re-derive}

@fact:this-document-states-the-practice-tool-neutrally This document states the practice tool-neutrally. @status:impl/done

@fact:ADAPT-IT-BY-HANDING-YOUR-AGENT-THE-TASK Adapt it by handing
your agent the task, not a copied template: @status:impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-managed-blocks/<version>/spec/flows/managed-blocks/`, check `vibe.lock`) end to end. Then design the managed
block for THIS tool:
1. Name the host files we write into and who else writes to each
   (the user by hand? other tools?).
2. Choose a unique, greppable, namespaced marker pair per file type,
   and write the exact byte-scan that locates them (no parser, no
   model). Include a version token in the opening marker.
3. Draft the do-not-edit notice that sits as the first line inside
   the block.
4. Restate the absent / present / malformed table in our terms, and
   write the exact hard-stop message for malformed.
5. Specify the three verbs, including that remove deletes the block
   AND its separation, and the byte-identical no-op.
Show me the design before writing any code.
```

## Summary {#summary}

- @fact:SUM-A-WHOLE-FILE-REWRITE-IS-A-DATA-LOSS-EVENT A whole-file rewrite of a shared file is a data-loss event. Own one
  delimited block; never touch a byte outside it. @status:spec/done
- @fact:SUM-MARKER-PROPERTIES-AND-VERSIONING Markers must be unique, greppable, paired, and carry an internal
  do-not-edit notice. Version the marker format from day one. @status:impl/done
- @fact:SUM-THREE-STATES Three states: absent → create at end of file; present → update or
  remove the body; malformed → hard stop, human decides. @status:impl/done
- @fact:SUM-REMOVE-RESTORES-THE-FILE-AND-NO-OP-WRITES-NOTHING Remove deletes the block *and* its separation, restoring the file
  byte-for-byte. Never rewrite when the result is byte-identical. @status:impl/done
- @fact:SUM-CLASSIFY-AT-PLAN-TIME Classify at plan time so mutation never discovers a surprise. @status:impl/done
- @fact:SUM-TWO-TOOLS-COHABIT-BECAUSE-MARKERS-ARE-UNIQUE Two tools cohabit because each scans only for its own markers —
  which is why the markers must be unique. @status:impl/done
