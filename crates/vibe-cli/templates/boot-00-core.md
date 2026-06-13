# Project boot snippet — `{project_name}`

User-owned. `vibe install` / `vibe uninstall` never touch this file.

## About this project

_TODO: one paragraph describing what `{project_name}` is and who it is for._

## Session boot sequence

Every AI session starts here. In order:
1. Read every file in `spec/boot/` in filename order.
2. Read `spec/WAL.md` — current project state (checkpoint, not history log).
3. Read the relevant PROP/FEAT documents under `spec/common/` and
   `spec/modules/` for the task at hand.
4. Only then begin work.

If `spec/WAL.md` is older than 24 hours, verify the state with the user before
doing destructive work.

## Memory layers

- **Head** (human): persistent but private.
- **WAL** (`spec/WAL.md`): volatile, rewritten each session, current state only.
- **Spec** (other files under `spec/`): stable decisions, addressable via
  `spec://<module>/<doc>#<section>` URIs.
- **Code** (`src/`, `tests/`): artefacts, regenerable.

Information flows top-down. When code changes first, reconcile up with a
Sync-from-Code proposal before rewriting code back to spec.

## Conflict resolution

Priority: **Human > Spec > Tests > Code.** When the AI believes the spec is
wrong, add a `<!-- REVIEW: … -->` marker, implement what the spec says, and
surface the disagreement in the end-of-session report.
