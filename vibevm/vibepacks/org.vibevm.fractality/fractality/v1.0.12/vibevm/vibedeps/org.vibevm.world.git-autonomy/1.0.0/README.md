# `flow:git-autonomy` — routine proceeds, red lines stop and ask {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-COMMIT-PUSH-AUTONOMY-POSTURE A vibevm `flow` package that installs the **commit/push autonomy** posture: routine, authorised
large changes proceed and are committed/pushed without a confirmation handshake, while a fixed
set of non-routine, hard-to-reverse operations always stops and asks a human first. @status:impl/done

@fact:THE-RED-LINE-SET-IS-NEVER-SUSPENDED The red-line set — rewriting published history, force-push, large binary blobs, CI / signing /
secrets configuration, and the catch-all *anything whose reversal would cost work* — is never
suspended, not even by a heads-down "move fast" posture: a mode may remove the "may I proceed
with routine work?" handshake, never the "may I cross an irreversible threshold?" one. @status:impl/done

@fact:package-contents-lead This package ships: @status:impl/done

- @fact:CONTENT-THE-FULL-PROTOCOL `spec/flows/autonomy/AUTONOMY-PROTOCOL.xml` — the routine-vs-red-line line, why the red lines
  survive every mode, and how to re-derive your own red-line set. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/32-flow-autonomy.xml` — the boot snippet loaded at session start. @status:impl/done

## Install {#install}

```bash
vibe install flow:git-autonomy
```

## Composition {#composition}

- @fact:COMPOSES-THE-GIT-PRACTICES-FAMILY A member of the `flow:git-practices` family (the commit-and-push discipline). @status:impl/done

## License {#license}

@fact:license-line UPL-1.0 — see `LICENSE`. @status:impl/done

