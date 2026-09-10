# Flow: WAL (Write-Ahead Log) {#root}

This project uses **WAL discipline** for session continuity.

## At the start of every session

1. Read `spec/WAL.md` **before** doing anything else.
2. Verify the `_Updated:` line is current. If it is older than 24 hours, ask
   the user to confirm state before acting on anything the WAL claims.
3. Honour every constraint listed in the WAL's **Constraints** section
   verbatim. These are the "do not touch" rules: violate them only after an
   explicit, in-session confirmation from the user.

## During the session

4. If the user makes a decision that affects future sessions, propose adding
   it to the WAL (or the spec, if it's architectural). Do not silently file
   it as "remembered."
5. If you find yourself about to violate a Constraint, stop and surface the
   question explicitly. A violation snuck past in a diff is a future bug.

## At the end of every session

6. Rewrite `spec/WAL.md` per the protocol in
   [`spec/flows/wal/session-end-hook.md`](../flows/wal/session-end-hook.md).
7. The WAL must reflect the **current** state, not the history. History
   lives in `git log` and in milestone commit messages — the WAL is a
   checkpoint, not a journal.

## Scope of this flow

- This flow owns only the WAL protocol files under `spec/flows/wal/` and
  this boot snippet.
- `spec/WAL.md` itself is project state, not package state — vibevm never
  creates, deletes, or overwrites it as part of install/uninstall.

Full protocol: [`spec/flows/wal/WAL-PROTOCOL.md`](../flows/wal/WAL-PROTOCOL.md).
