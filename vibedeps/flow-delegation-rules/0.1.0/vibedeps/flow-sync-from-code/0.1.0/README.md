# `flow:sync-from-code` — reconcile specs with code when code changes first

A vibevm `flow` package that installs the **Sync-from-Code** protocol into
a project. The normal information flow in a spec-driven project is
top-down (head → WAL → spec → code); Sync-from-Code is the *exceptional*
path for closing spec drift when code moves before the spec.

Two legitimate situations break top-down flow:

- The user edits code directly in the editor because it's faster than
  articulating the intent to the agent first.
- The user gives an imperative command in chat ("change the timeout to
  600 s") and the agent edits code without touching the spec.

In both cases the spec is now wrong; left unreconciled, the next session
reads the stale spec, concludes the code is in error, and "fixes" the
code back — correctly by the spec-wins rule, wrong in outcome. This
flow is the sanctioned way to close that gap.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/sync-from-code/SYNC-PROTOCOL.md` — full protocol: what
  Sync-from-Code is, when to run it, what the draft spec diff must
  contain (value + reason + revisit trigger), and what it explicitly
  does not do.
- `spec/flows/sync-from-code/when-to-apply.md` — decision table:
  *should I run it right now?*, including the cases where you should
  **not** (temporary hacks, mechanical changes, unnamed reasons).
- `spec/flows/sync-from-code/review-workflow.md` — human-side checklist
  for the approval step: six checks that catch bad syncs before they
  land.
- `spec/boot/20-flow-sync-from-code.md` — boot snippet loaded at
  session start, pointing the agent at the protocol.

## Install

```bash
vibe install flow:sync-from-code
```

## Uninstall

```bash
vibe uninstall flow:sync-from-code
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files (`00-core.md`, `90-user.md`, `WAL.md`) are
never touched.

## Composition

- Works with `flow:wal` (`10-…`) and `flow:atomic-commits` (`30-…`):
  numeric boot-snippet prefixes are distinct by design.
- A successful sync *may* trigger a WAL update; that update goes
  through `flow:wal`'s session-end hook, not this flow.
- A sync ends in a `docs(spec)` commit; message formatting is pinned
  by `flow:atomic-commits`.

## Philosophical background

The protocol is extracted from *AI-native development*, chapter 3
(*"Архитектура памяти"*, subsection "Протокол Sync-from-Code"). Short
version: spec-driven projects need a named, rare, human-approved path
for the inverse flow; without one, drift accumulates silently and the
spec stops being authoritative.

## License

UPL-1.0 — The Universal Permissive License, Version 1.0. See `LICENSE` and the surrounding registry for distribution terms.
