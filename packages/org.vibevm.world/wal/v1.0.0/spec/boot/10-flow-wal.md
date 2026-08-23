# Flow: WAL (Write-Ahead Log) {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-USES-WAL-DISCIPLINE-FOR-SESSION-CONTINUITY This project uses **WAL discipline** for session continuity. @status:impl/done

@fact:two-files-carry-it-lead Two files
carry it: @status:impl/done

- @fact:FILE-WAL-IS-THE-CANONICAL-LIVING-CHECKPOINT `spec/WAL.md` — the living checkpoint. **Canonical.** @status:impl/done
- @fact:FILE-CONTINUE-IS-THE-COLD-RESUME-SNAPSHOT `CONTINUE.md` (repo root) — the cold-resume snapshot. The WAL
  supersedes it wherever they diverge. @status:impl/done

## At the start of every session {#session-start}

1. @fact:READ-THE-WAL-BEFORE-DOING-ANYTHING-ELSE Read `spec/WAL.md` **before** doing anything else. The `wal-status`
   skill, where installed, is the fast form of this read. @status:impl/done
2. @fact:VERIFY-THE-UPDATED-LINE-IS-CURRENT Verify the `_Updated:` line is current. If it is older than 24
   hours, ask the user to confirm state before acting on anything the
   WAL claims — and before any destructive work. @status:impl/done
3. @fact:HONOUR-EVERY-CONSTRAINT-VERBATIM Honour every constraint listed in the WAL's **Constraints** section
   verbatim. These are the "do not touch" rules: violate them only
   after an explicit, in-session confirmation from the user. @status:impl/done

## During the session {#during}

4. @fact:PROPOSE-ADDING-A-DURABLE-DECISION-TO-THE-WAL If the user makes a decision that affects future sessions, propose
   adding it to the WAL (or the spec, if it's architectural). Do not
   silently file it as "remembered." @status:impl/done
5. @fact:STOP-AND-SURFACE-BEFORE-VIOLATING-A-CONSTRAINT If you find yourself about to violate a Constraint, stop and surface
   the question explicitly. A violation snuck past in a diff is a
   future bug. @status:impl/done

## At the end of every session {#session-end}

6. @fact:REWRITE-THE-WAL-NEVER-APPEND-TO-IT Rewrite `spec/WAL.md` per the protocol in
   @spec://org.vibevm.world/wal/flows/wal/session-end-hook#root.
   Rewrite, not append — the WAL must reflect the **current** state,
   not the history. History lives in `git log` and in milestone commit
   messages; the WAL is a checkpoint, not a journal. @status:impl/done

## Session commands {#commands}

7. @fact:RECOGNISE-THE-WIND-DOWN-PHRASES Recognise the **wind-down** phrases — `END SESSION`, `WRAP UP`,
   `CHECKPOINT AND CLOSE`, and any project-defined twins (recognise the
   intent, not the exact wording). A wind-down invokes the full
   session-end hook *plus* a wholesale overwrite of `CONTINUE.md`, per
   @spec://org.vibevm.world/wal/flows/wal/cold-resume#root. @status:impl/done
8. @fact:RECOGNISE-THE-RESUME-PHRASES Recognise the **resume** phrases — `RESUME SESSION`, `RESTORE
   CONTEXT`. Restore context, verify the repository state empirically,
   emit a status report — then **stop and wait for direction**. A
   recorded "next step" is a candidate, not authorisation. @status:impl/done

## Scope of this flow {#scope}

- @fact:FLOW-OWNS-ONLY-ITS-OWN-FILES This flow owns only the protocol files under `spec/flows/wal/`, the
  `wal-status` skill, and this boot snippet. @status:impl/done
- @fact:WAL-AND-CONTINUE-ARE-PROJECT-STATE-NOT-PACKAGE-STATE `spec/WAL.md` and `CONTINUE.md` are **project state**, not package
  state — the package never creates, deletes, or overwrites them as
  part of install or uninstall. @status:impl/done

@fact:WAL-IS-THE-SINGLE-DEVELOPER-CONVENTION This flow is the
**single-developer, central-WAL convention**. A multi-developer project
chooses its own session-durability scheme — many WALs (the registered-
subprojects form is `flow:wal-specspaces`), or none — by not installing
this flow or by superseding it; nothing in VibeVM requires a WAL
(PROP-049). @status:impl/done

@fact:full-protocol-pointer Full protocol: @spec://org.vibevm.world/wal/flows/wal/WAL-PROTOCOL#root. @status:impl/done
