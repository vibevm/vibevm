# `flow:autonomy` — routine proceeds, red lines stop and ask

A vibevm `flow` package that installs the **commit/push autonomy** posture: routine, authorised
large changes proceed and are committed/pushed without a confirmation handshake, while a fixed
set of non-routine, hard-to-reverse operations always stops and asks a human first.

The red-line set — rewriting published history, force-push, large binary blobs, CI / signing /
secrets configuration, and the catch-all *anything whose reversal would cost work* — is never
suspended, not even by a heads-down "move fast" posture: a mode may remove the "may I proceed
with routine work?" handshake, never the "may I cross an irreversible threshold?" one.

This package ships:

- `spec/flows/autonomy/AUTONOMY-PROTOCOL.md` — the routine-vs-red-line line, why the red lines
  survive every mode, and how to re-derive your own red-line set.
- `spec/boot/32-flow-autonomy.md` — the boot snippet loaded at session start.

## Install

```bash
vibe install flow:autonomy
```

## Composition

- A member of the `flow:git-practices` family (the commit-and-push discipline).

## License

UPL-1.0 — see `LICENSE`.
