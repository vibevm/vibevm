# vibevm — boot snippet: project foundation

**Project:** vibevm — a CLI software project manager for spec-driven AI-assisted development.
**Binary:** `vibe`.
**Source of truth:** [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) (project root). This is the entire implementation specification.

## Session boot sequence

Every session starts here. In order:
1. Read this file and the rest of `spec/boot/` end to end.
2. Read `spec/WAL.md` — current project state (checkpoint, not log).
3. Read the relevant PROP/FEAT under `spec/common/` and `spec/modules/` for the task at hand.
4. Only then start work.

If `spec/WAL.md` is older than 24 hours, verify the state with the user before doing destructive work.

## The four non-negotiable rules

See [`CLAUDE.md`](../../CLAUDE.md) (and its identical copies `AGENTS.md` / `GEMINI.md`) for the full text. Authoritative reference: [spec://vibevm/common/PROP-000#commits](../common/PROP-000.md#commits). Summary only:

1. **Attribution — keep this repository human-authored.** Never mark commits, branches, comments, or any artefact as machine-authored. The rule itself (and its copy in PROP-000 §12.1) is the only place in the project where that topic is discussed.
2. **Conventional Commits** — short subject, long explanatory body answering *why*.
3. **Group commits by meaning** — one logical unit per commit, split mixed working trees.
4. **Autonomy on routine changes only** — commit and push routine work without asking; stop and ask for history rewrites, force-push, large blobs, CI/signing changes, and anything whose reversal costs work.

## Reading layers (per book, `refs/book/`)

- **Head** (human's memory) — not your concern, but respect that it exists. Human wins conflicts with the spec.
- **WAL** (`spec/WAL.md`) — volatile, rewritten each session, describes *current* state.
- **Spec** (other files under `spec/`) — stable decisions, addressable via `spec://…` URIs.
- **Code** (everything under `crates/`, `tests/`) — artefacts. Losing them is inconvenient; losing the spec is a catastrophe.

Information flows top-down. If code changes first, reconcile up via the Sync-from-Code protocol (book, chapter 3) — propose a spec update, do not rewrite code back.

## Hard conventions

- **Language:** Rust. See [spec://vibevm/common/PROP-000#language](../common/PROP-000.md#language).
- **Manifests:** TOML. Project manifest = `vibe.toml`; package manifest = `vibe-package.toml`; lockfile = `vibe.lock`.
- **Terminology:** only four installable kinds — `flow`, `feat`, `stack`, `tool`. Never say "lifecycle", "phase", "goal", "plugin" (except that "plugin" == "package" in passing context). See `VIBEVM-SPEC.md` §4.
- **Repository URLs:** vibevm source = `git@gitverse.ru:anarchic/vibevm.git` / `https://gitverse.ru/anarchic/vibevm`. Package registry (future) = `git@gitverse.ru:anarchic/vibespecs.git`.

## Uncertainty protocol

When the spec is silent on a question:
1. Re-read the relevant section of `VIBEVM-SPEC.md`.
2. Re-read the relevant chapter in `refs/book/`.
3. Look at the closest analog under `refs/src/` (cargo, uv, spec-kit).
4. If still unclear: mark the decision with `<!-- REVIEW: … -->`, pick the conservative interpretation, proceed, flag in the end-of-session report. Never silently invent semantic behavior.

## Files you MUST NOT touch without explicit instruction

- `spec/boot/00-core.md` (this file) — user-owned.
- `spec/boot/90-user.md` — user-owned overrides.
- `VIBEVM-SPEC.md` — the owner-frozen specification document; edits require the user. (URL corrections landed at the owner's direct request.)
- `refs/book/` — the user's book, read-only reference material.

## End of session

- Update `spec/WAL.md` to reflect the *current* state (rewrite, not append — it is a checkpoint).
- Propose a milestone commit if work is a logical unit. For routine work, follow rule 4 above: commit and push using rules 2–3. For non-routine operations, stop and ask the user first.
