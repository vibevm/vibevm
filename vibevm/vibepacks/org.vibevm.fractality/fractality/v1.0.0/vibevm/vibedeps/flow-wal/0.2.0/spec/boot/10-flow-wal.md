# Flow: WAL (Write-Ahead Log) {#root}

This project uses **WAL discipline** for session continuity. Two files
carry it:

- `spec/WAL.md` — the living checkpoint. **Canonical.**
- `CONTINUE.md` (repo root) — the cold-resume snapshot. The WAL
  supersedes it wherever they diverge.

## At the start of every session {#session-start}

1. Read `spec/WAL.md` **before** doing anything else. The `wal-status`
   skill, where installed, is the fast form of this read.
2. Verify the `_Updated:` line is current. If it is older than 24
   hours, ask the user to confirm state before acting on anything the
   WAL claims — and before any destructive work.
3. Honour every constraint listed in the WAL's **Constraints** section
   verbatim. These are the "do not touch" rules: violate them only
   after an explicit, in-session confirmation from the user.

## During the session {#during}

4. If the user makes a decision that affects future sessions, propose
   adding it to the WAL (or the spec, if it's architectural). Do not
   silently file it as "remembered."
5. If you find yourself about to violate a Constraint, stop and surface
   the question explicitly. A violation snuck past in a diff is a
   future bug.

## At the end of every session {#session-end}

6. Rewrite `spec/WAL.md` per the protocol in
   [`spec/flows/wal/session-end-hook.md`](../flows/wal/session-end-hook.md).
   Rewrite, not append — the WAL must reflect the **current** state,
   not the history. History lives in `git log` and in milestone commit
   messages; the WAL is a checkpoint, not a journal.

## Session commands {#commands}

7. Recognise the **wind-down** phrases — `END SESSION`, `WRAP UP`,
   `CHECKPOINT AND CLOSE`, and any project-defined twins (recognise the
   intent, not the exact wording). A wind-down invokes the full
   session-end hook *plus* a wholesale overwrite of `CONTINUE.md`, per
   [`spec/flows/wal/cold-resume.md`](../flows/wal/cold-resume.md).
8. Recognise the **resume** phrases — `RESUME SESSION`, `RESTORE
   CONTEXT`. Restore context, verify the repository state empirically,
   emit a status report — then **stop and wait for direction**. A
   recorded "next step" is a candidate, not authorisation.

## Scope of this flow {#scope}

- This flow owns only the protocol files under `spec/flows/wal/`, the
  `wal-status` skill, and this boot snippet.
- `spec/WAL.md` and `CONTINUE.md` are **project state**, not package
  state — the package never creates, deletes, or overwrites them as
  part of install or uninstall.

Full protocol: [`spec/flows/wal/WAL-PROTOCOL.md`](../flows/wal/WAL-PROTOCOL.md).
