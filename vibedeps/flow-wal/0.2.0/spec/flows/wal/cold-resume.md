# Cold resume — `CONTINUE.md` and the session commands {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The second file of the discipline:
`CONTINUE.md`, the cold-resume snapshot at the repository root. What
goes in it, when it is written, the wind-down and resume commands that
bracket a session, and the precedence rule when the snapshot and the
WAL disagree. @impl/done

## Why a second file {#why}

##WAL-IS-WRITTEN-FOR-THE-SAME-PROJECT-RHYTHM The WAL is written for the next session on the *same* project rhythm:
terse, current, one page, assuming the reader knows the terrain. @impl/done

##A-COLD-READER-DOES-NOT-KNOW-THE-TERRAIN A cold
reader does not know the terrain: you on a different machine, a
teammate cloning the repository, a session resuming after weeks or
after a context compaction. @spec/done

##THE-COLD-READER-NEEDS-THE-TOUR The cold reader needs the tour — where
things are, what commands run, what was decided and why — before the
one-page checkpoint means anything. @spec/done

##CONTINUE-IS-THAT-TOUR `CONTINUE.md` is that tour. @impl/done

##THE-PAIR-PASSES-THE-ACCEPTANCE-TEST The pair together passes the discipline's
acceptance test: **a stranger with only the repository resumes work
without asking.** @impl/done

## The CONTINUE.md contract {#contract}

##CONTINUE-LIVES-AT-THE-REPOSITORY-ROOT `CONTINUE.md` lives at the repository root, where a cold reader looks
first. @impl/done

##CONTINUE-IS-OVERWRITTEN-WHOLESALE It is **overwritten wholesale** every time it is written — never
appended to, never patched; staleness compounds otherwise. @impl/done

##body-contents-lead The body
includes, at minimum: @impl/done

1. ##BODY-TLDR-SUMMARY A short TL;DR / executive summary at the top. @impl/done
2. ##BODY-WHERE-WORK-STANDS Where work stands: branch, ahead/behind the remote, working-tree
   status. @impl/done
3. ##BODY-ACTIVE-BLOCKER The active blocker, if any, and the exact human action that
   unblocks it. @impl/done
4. ##BODY-NEXT-STEPS-RECIPE The exact next-steps recipe — commands, file paths, line numbers —
   for whoever picks up cold. @impl/done
5. ##BODY-NON-OBVIOUS-FINDINGS Non-obvious findings of the session: API quirks, config gotchas,
   vendor-specific surprises. @impl/done
6. ##BODY-REPOSITORY-MAP A repository map: top-level layout and what each directory or
   component holds. @impl/done
7. ##BODY-DECISIONS-IN-FORCE The architectural and policy decisions still in force, in long
   form. @impl/done
8. ##BODY-RECENT-COMMIT-CHAIN The recent commit chain (last ~25, oneline format), so the cold
   reader sees velocity and direction. @impl/done
9. ##BODY-QUICK-START-COMMANDS Quick-start commands for the workspace. @impl/done
10. ##BODY-PRECEDENCE-POINTER A pointer noting that the WAL is the canonical living state and
    supersedes this snapshot if they diverge. @impl/done

##what-the-item-groups-answer Items 1–4 answer "what do I do right now"; items 5–9 answer "what must
I know before I trust myself here"; item 10 keeps the file honest
about its own rank. @impl/done

## When to write it {#when}

- ##WRITE-AT-EVERY-EXPLICIT-WIND-DOWN **At every explicit wind-down** (the command below). Mandatory. @impl/done
- ##WRITE-BEFORE-A-MACHINE-SWITCH **Before a machine switch** — the other machine gets the tour. @impl/done
- ##WRITE-BEFORE-A-LONG-GAP **Before a long gap** — a vacation, weeks on another project; future
  you is a cold reader too. @impl/done

##any-session-end-is-a-fine-time Any session end is a fine time; the wind-down makes it non-optional. @impl/done

## The wind-down command {#wind-down}

##WIND-DOWN-IS-THE-EXPLICIT-FORM-OF-THE-HOOK The wind-down is the explicit form of the session-end hook. @impl/done

##SHIP-TRIGGER-PHRASES Ship
trigger phrases: `END SESSION`, `WRAP UP`, `CHECKPOINT AND CLOSE`. @impl/done

##RECOGNISE-THE-INTENT-NOT-THE-EXACT-WORDING Recognise the intent, not the exact wording. @impl/done

##A-PROJECT-MAY-ADD-NATIVE-LANGUAGE-TWINS A project may add
native-language twins in its agent instructions — the origin project
of this flow runs a bilingual Russian/English set. @impl/done

##required-behaviour-lead Required behaviour, in order: @impl/done

1. ##STEP-OVERWRITE-CONTINUE **Overwrite `CONTINUE.md`** wholesale, per the contract above. @impl/done
2. ##STEP-REWRITE-THE-WAL **Rewrite `spec/WAL.md`** per
   [`session-end-hook.md`](session-end-hook.md): fresh date line,
   current phase, constraints, next step. @impl/done
3. ##STEP-COMMIT-IN-TOPIC-GROUPED-COMMITS **Commit in topic-grouped commits.** The snapshot and the WAL
   update are checkpoint commits; a code or config change landed by
   the same session is a separate commit. @impl/done
4. ##STEP-PUSH-ONLY-IF-AUTONOMY-SANCTIONS-IT **Push only if the project's autonomy rules sanction it.** No
   standing autonomy — stop at drafts or at local commits, per the
   hook's propose-by-default rule. @impl/done
5. ##STEP-EMIT-A-CHAT-TLDR **Emit a chat TL;DR** of what the wind-down did: files written,
   commits created, push status, what the next session should pick up
   first. One screen; enough detail to verify without opening files. @impl/done

##the-point-of-the-command The point of the command is to make session-boundary loss-of-context
cheap: any session can end at any time and be resumed from
`CONTINUE.md` plus the WAL with no degradation. @impl/done

##TREAT-IT-AS-A-HARD-CONTRACT-NOT-A-COURTESY Treat it as a hard
contract, not a courtesy. @impl/done

## The resume command {#resume}

##RESUME-TRIGGER-PHRASES Trigger phrases: `RESUME SESSION`, `RESTORE CONTEXT` (and twins —
recognise the intent). @impl/done

##THE-JOB-IS-TO-RESTORE-CONTEXT-AND-REPORT The job is to **restore context and report —
nothing else**: @impl/done

1. ##RESUME-STEP-RUN-THE-BOOT-SEQUENCE Run the project's boot sequence (whatever its agent instructions
   define), then read `CONTINUE.md` and `spec/WAL.md`. @impl/done
2. ##RESUME-STEP-VERIFY-THE-REPOSITORY-EMPIRICALLY Verify the repository state empirically: current branch, sync with
   the remote, working-tree status, recent commits. Never take the
   snapshot's word for what the tree looks like. @impl/done
3. ##RESUME-STEP-EMIT-A-STATUS-REPORT Emit a status report: where work stands, active blockers, and the
   candidate next steps. @impl/done
4. ##RESUME-STEP-STOP-AND-WAIT-FOR-DIRECTION **Stop and wait for direction.** No code edits, no commits, no
   pushes. @impl/done

## Restore is not authorisation {#not-authorisation}

##A-RECORDED-POINTER-IS-A-CANDIDATE-NOT-AUTHORISATION Any "resume work at …" pointer in `CONTINUE.md` or the WAL names the
*candidate* next step for the report — it is not authorisation to
start it. @impl/done

##why-the-resume-boundary-exists The resume boundary exists so the owner can inspect the
restored state and steer, possibly somewhere other than the recorded
next step. @spec/done

##booting-into-execution-takes-the-decision-away A session that boots straight into execution takes that
decision away from the owner. @spec/done

##the-misfire-that-produced-the-rule This rule is written down because exactly that misfire happened once:
a resumed session read the recorded pointer as a work order and began
executing, when the owner wanted a report. @spec/done

##boundary-explicit-ever-since The boundary has been
explicit ever since. @impl/done

## Precedence {#precedence}

##WAL-IS-CANONICAL-CONTINUE-IS-A-SNAPSHOT `spec/WAL.md` is canonical; `CONTINUE.md` is a snapshot of the moment
the last wind-down ran. @impl/done

##ON-DISAGREEMENT-TRUST-THE-WAL-AND-FLAG-IT When they disagree — and they will, whenever a
session updates the WAL without a full wind-down — trust the WAL and
flag the divergence in your report. @impl/done

##THE-SAME-RULE-COVERS-ANY-OTHER-SNAPSHOT The same rule covers any other
snapshot: a plan document's status line, a README's "current state"
paragraph. @impl/done

##THE-LIVING-CHECKPOINT-BEATS-A-FROZEN-SNAPSHOT The living checkpoint beats a frozen snapshot, always. @impl/done

## Never {#never}

- ##NEVER-APPEND-TO-CONTINUE Never append to `CONTINUE.md` — overwrite it wholesale. @impl/done
- ##NEVER-WRITE-THE-SNAPSHOT-FROM-MEMORY-ALONE Never write the snapshot from memory alone — verify branch, tree,
  and commits empirically first. @impl/done
- ##NEVER-START-EXECUTING-AFTER-A-RESUME-COMMAND Never start executing after a resume command — report, then wait. @impl/done
- ##NEVER-TREAT-CONTINUE-AS-OVERRIDING-THE-WAL Never treat `CONTINUE.md` as overriding the WAL. @impl/done
- ##NEVER-SKIP-THE-CHAT-TLDR-ON-A-WIND-DOWN Never skip the chat TL;DR on a wind-down: it is how the user
  verifies the checkpoint without opening files. @impl/done

## Summary {#summary}

- ##SUM-CONTINUE-IS-THE-COLD-READERS-TOUR `CONTINUE.md` at the repo root is the cold reader's tour: TL;DR,
  state, blocker, recipe, findings, map, decisions, commits,
  quick-start — overwritten wholesale, never appended. @impl/done
- ##SUM-WHEN-TO-WRITE-IT Write it at every wind-down, before machine switches, before gaps. @impl/done
- ##SUM-WIND-DOWN-SHAPE Wind-down: snapshot + WAL rewrite + topic-grouped commits +
  sanctioned push + chat TL;DR. @impl/done
- ##SUM-RESUME-SHAPE Resume: boot, read both files, verify empirically, report, stop. @impl/done
- ##SUM-A-RECORDED-NEXT-STEP-IS-A-CANDIDATE A recorded next step is a candidate, not authorisation. @impl/done
- ##SUM-THE-WAL-SUPERSEDES-THE-SNAPSHOT The WAL supersedes the snapshot wherever they diverge. @impl/done
