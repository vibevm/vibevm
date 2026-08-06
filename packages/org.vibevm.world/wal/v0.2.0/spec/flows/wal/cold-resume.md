# Cold resume — `CONTINUE.md` and the session commands {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** The second file of the discipline:
`CONTINUE.md`, the cold-resume snapshot at the repository root. What
goes in it, when it is written, the wind-down and resume commands that
bracket a session, and the precedence rule when the snapshot and the
WAL disagree. @status:impl/done

## Why a second file {#why}

@fact:WAL-IS-WRITTEN-FOR-THE-SAME-PROJECT-RHYTHM The WAL is written for the next session on the *same* project rhythm:
terse, current, one page, assuming the reader knows the terrain. @status:impl/done

@fact:A-COLD-READER-DOES-NOT-KNOW-THE-TERRAIN A cold
reader does not know the terrain: you on a different machine, a
teammate cloning the repository, a session resuming after weeks or
after a context compaction. @status:spec/done

@fact:THE-COLD-READER-NEEDS-THE-TOUR The cold reader needs the tour — where
things are, what commands run, what was decided and why — before the
one-page checkpoint means anything. @status:spec/done

@fact:CONTINUE-IS-THAT-TOUR `CONTINUE.md` is that tour. @status:impl/done

@fact:THE-PAIR-PASSES-THE-ACCEPTANCE-TEST The pair together passes the discipline's
acceptance test: **a stranger with only the repository resumes work
without asking.** @status:impl/done

## The CONTINUE.md contract {#contract}

@fact:CONTINUE-LIVES-AT-THE-REPOSITORY-ROOT `CONTINUE.md` lives at the repository root, where a cold reader looks
first. @status:impl/done

@fact:CONTINUE-IS-OVERWRITTEN-WHOLESALE It is **overwritten wholesale** every time it is written — never
appended to, never patched; staleness compounds otherwise. @status:impl/done

@fact:body-contents-lead The body
includes, at minimum: @status:impl/done

1. @fact:BODY-TLDR-SUMMARY A short TL;DR / executive summary at the top. @status:impl/done
2. @fact:BODY-WHERE-WORK-STANDS Where work stands: branch, ahead/behind the remote, working-tree
   status. @status:impl/done
3. @fact:BODY-ACTIVE-BLOCKER The active blocker, if any, and the exact human action that
   unblocks it. @status:impl/done
4. @fact:BODY-NEXT-STEPS-RECIPE The exact next-steps recipe — commands, file paths, line numbers —
   for whoever picks up cold. @status:impl/done
5. @fact:BODY-NON-OBVIOUS-FINDINGS Non-obvious findings of the session: API quirks, config gotchas,
   vendor-specific surprises. @status:impl/done
6. @fact:BODY-REPOSITORY-MAP A repository map: top-level layout and what each directory or
   component holds. @status:impl/done
7. @fact:BODY-DECISIONS-IN-FORCE The architectural and policy decisions still in force, in long
   form. @status:impl/done
8. @fact:BODY-RECENT-COMMIT-CHAIN The recent commit chain (last ~25, oneline format), so the cold
   reader sees velocity and direction. @status:impl/done
9. @fact:BODY-QUICK-START-COMMANDS Quick-start commands for the workspace. @status:impl/done
10. @fact:BODY-PRECEDENCE-POINTER A pointer noting that the WAL is the canonical living state and
    supersedes this snapshot if they diverge. @status:impl/done

@fact:what-the-item-groups-answer Items 1–4 answer "what do I do right now"; items 5–9 answer "what must
I know before I trust myself here"; item 10 keeps the file honest
about its own rank. @status:impl/done

## When to write it {#when}

- @fact:WRITE-AT-EVERY-EXPLICIT-WIND-DOWN **At every explicit wind-down** (the command below). Mandatory. @status:impl/done
- @fact:WRITE-BEFORE-A-MACHINE-SWITCH **Before a machine switch** — the other machine gets the tour. @status:impl/done
- @fact:WRITE-BEFORE-A-LONG-GAP **Before a long gap** — a vacation, weeks on another project; future
  you is a cold reader too. @status:impl/done

@fact:any-session-end-is-a-fine-time Any session end is a fine time; the wind-down makes it non-optional. @status:impl/done

## The wind-down command {#wind-down}

@fact:WIND-DOWN-IS-THE-EXPLICIT-FORM-OF-THE-HOOK The wind-down is the explicit form of the session-end hook. @status:impl/done

@fact:SHIP-TRIGGER-PHRASES Ship
trigger phrases: `END SESSION`, `WRAP UP`, `CHECKPOINT AND CLOSE`. @status:impl/done

@fact:RECOGNISE-THE-INTENT-NOT-THE-EXACT-WORDING Recognise the intent, not the exact wording. @status:impl/done

@fact:A-PROJECT-MAY-ADD-NATIVE-LANGUAGE-TWINS A project may add
native-language twins in its agent instructions — the origin project
of this flow runs a bilingual Russian/English set. @status:impl/done

@fact:required-behaviour-lead Required behaviour, in order: @status:impl/done

1. @fact:STEP-OVERWRITE-CONTINUE **Overwrite `CONTINUE.md`** wholesale, per the contract above. @status:impl/done
2. @fact:STEP-REWRITE-THE-WAL **Rewrite `spec/WAL.md`** per
   [`session-end-hook.md`](session-end-hook.md): fresh date line,
   current phase, constraints, next step. @status:impl/done
3. @fact:STEP-COMMIT-IN-TOPIC-GROUPED-COMMITS **Commit in topic-grouped commits.** The snapshot and the WAL
   update are checkpoint commits; a code or config change landed by
   the same session is a separate commit. @status:impl/done
4. @fact:STEP-PUSH-ONLY-IF-AUTONOMY-SANCTIONS-IT **Push only if the project's autonomy rules sanction it.** No
   standing autonomy — stop at drafts or at local commits, per the
   hook's propose-by-default rule. @status:impl/done
5. @fact:STEP-EMIT-A-CHAT-TLDR **Emit a chat TL;DR** of what the wind-down did: files written,
   commits created, push status, what the next session should pick up
   first. One screen; enough detail to verify without opening files. @status:impl/done

@fact:the-point-of-the-command The point of the command is to make session-boundary loss-of-context
cheap: any session can end at any time and be resumed from
`CONTINUE.md` plus the WAL with no degradation. @status:impl/done

@fact:TREAT-IT-AS-A-HARD-CONTRACT-NOT-A-COURTESY Treat it as a hard
contract, not a courtesy. @status:impl/done

## The resume command {#resume}

@fact:RESUME-TRIGGER-PHRASES Trigger phrases: `RESUME SESSION`, `RESTORE CONTEXT` (and twins —
recognise the intent). @status:impl/done

@fact:THE-JOB-IS-TO-RESTORE-CONTEXT-AND-REPORT The job is to **restore context and report —
nothing else**: @status:impl/done

1. @fact:RESUME-STEP-RUN-THE-BOOT-SEQUENCE Run the project's boot sequence (whatever its agent instructions
   define), then read `CONTINUE.md` and `spec/WAL.md`. @status:impl/done
2. @fact:RESUME-STEP-VERIFY-THE-REPOSITORY-EMPIRICALLY Verify the repository state empirically: current branch, sync with
   the remote, working-tree status, recent commits. Never take the
   snapshot's word for what the tree looks like. @status:impl/done
3. @fact:RESUME-STEP-EMIT-A-STATUS-REPORT Emit a status report: where work stands, active blockers, and the
   candidate next steps. @status:impl/done
4. @fact:RESUME-STEP-STOP-AND-WAIT-FOR-DIRECTION **Stop and wait for direction.** No code edits, no commits, no
   pushes. @status:impl/done

## Restore is not authorisation {#not-authorisation}

@fact:A-RECORDED-POINTER-IS-A-CANDIDATE-NOT-AUTHORISATION Any "resume work at …" pointer in `CONTINUE.md` or the WAL names the
*candidate* next step for the report — it is not authorisation to
start it. @status:impl/done

@fact:why-the-resume-boundary-exists The resume boundary exists so the owner can inspect the
restored state and steer, possibly somewhere other than the recorded
next step. @status:spec/done

@fact:booting-into-execution-takes-the-decision-away A session that boots straight into execution takes that
decision away from the owner. @status:spec/done

@fact:the-misfire-that-produced-the-rule This rule is written down because exactly that misfire happened once:
a resumed session read the recorded pointer as a work order and began
executing, when the owner wanted a report. @status:spec/done

@fact:boundary-explicit-ever-since The boundary has been
explicit ever since. @status:impl/done

## Precedence {#precedence}

@fact:WAL-IS-CANONICAL-CONTINUE-IS-A-SNAPSHOT `spec/WAL.md` is canonical; `CONTINUE.md` is a snapshot of the moment
the last wind-down ran. @status:impl/done

@fact:ON-DISAGREEMENT-TRUST-THE-WAL-AND-FLAG-IT When they disagree — and they will, whenever a
session updates the WAL without a full wind-down — trust the WAL and
flag the divergence in your report. @status:impl/done

@fact:THE-SAME-RULE-COVERS-ANY-OTHER-SNAPSHOT The same rule covers any other
snapshot: a plan document's status line, a README's "current state"
paragraph. @status:impl/done

@fact:THE-LIVING-CHECKPOINT-BEATS-A-FROZEN-SNAPSHOT The living checkpoint beats a frozen snapshot, always. @status:impl/done

## Never {#never}

- @fact:NEVER-APPEND-TO-CONTINUE Never append to `CONTINUE.md` — overwrite it wholesale. @status:impl/done
- @fact:NEVER-WRITE-THE-SNAPSHOT-FROM-MEMORY-ALONE Never write the snapshot from memory alone — verify branch, tree,
  and commits empirically first. @status:impl/done
- @fact:NEVER-START-EXECUTING-AFTER-A-RESUME-COMMAND Never start executing after a resume command — report, then wait. @status:impl/done
- @fact:NEVER-TREAT-CONTINUE-AS-OVERRIDING-THE-WAL Never treat `CONTINUE.md` as overriding the WAL. @status:impl/done
- @fact:NEVER-SKIP-THE-CHAT-TLDR-ON-A-WIND-DOWN Never skip the chat TL;DR on a wind-down: it is how the user
  verifies the checkpoint without opening files. @status:impl/done

## Summary {#summary}

- @fact:SUM-CONTINUE-IS-THE-COLD-READERS-TOUR `CONTINUE.md` at the repo root is the cold reader's tour: TL;DR,
  state, blocker, recipe, findings, map, decisions, commits,
  quick-start — overwritten wholesale, never appended. @status:impl/done
- @fact:SUM-WHEN-TO-WRITE-IT Write it at every wind-down, before machine switches, before gaps. @status:impl/done
- @fact:SUM-WIND-DOWN-SHAPE Wind-down: snapshot + WAL rewrite + topic-grouped commits +
  sanctioned push + chat TL;DR. @status:impl/done
- @fact:SUM-RESUME-SHAPE Resume: boot, read both files, verify empirically, report, stop. @status:impl/done
- @fact:SUM-A-RECORDED-NEXT-STEP-IS-A-CANDIDATE A recorded next step is a candidate, not authorisation. @status:impl/done
- @fact:SUM-THE-WAL-SUPERSEDES-THE-SNAPSHOT The WAL supersedes the snapshot wherever they diverge. @status:impl/done
