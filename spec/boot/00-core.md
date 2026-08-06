# vibevm — boot snippet: project foundation

@fact:project-identity **Project:** vibevm — a CLI software project manager for spec-driven AI-assisted development.
**Binary:** `vibe`.
**Source of truth:** [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) (project root). This is the entire implementation specification. @status:impl/done

## Session boot sequence

@fact:boot-sequence-lead Every session starts here. In order: @status:impl/done
1. @fact:BOOT-STEP-BOOT-DIR Read this file and the rest of `spec/boot/` end to end — including the generated `INDEX.md` (the computed boot manifest) and `STATIC.md` (the priority lane). `vibe` owns the boot order; see `VIBEVM-SPEC.md` §6. @status:impl/done
2. @fact:BOOT-STEP-WAL Read `spec/WAL.md` — current project state (checkpoint, not log). @status:impl/done
3. @fact:BOOT-STEP-PROPS Read the relevant PROP/FEAT under `spec/common/` and `spec/modules/` for the task at hand. @status:impl/done
4. @fact:BOOT-STEP-START-WORK Only then start work. @status:impl/done

@fact:WAL-STALENESS-CHECK If `spec/WAL.md` is older than 24 hours, verify the state with the user before doing destructive work. @status:impl/done

## The four non-negotiable rules

@fact:four-rules-lead See [`CLAUDE.md`](../../CLAUDE.md) (and its identical copies `AGENTS.md` / `GEMINI.md`) for the full text. Authoritative reference: [spec://org.vibevm.core/vibevm/common/PROP-000#commits](../common/PROP-000.md#commits). Summary only: @status:impl/done

1. @fact:RULE-ATTRIBUTION **Attribution — keep this repository human-authored.** Never mark commits, branches, comments, or any artefact as machine-authored. The rule itself (and its copy in PROP-000 §12.1) is the only place in the project where that topic is discussed. @status:impl/done
2. @fact:RULE-CONVENTIONAL-COMMITS **Conventional Commits** — short subject, long explanatory body answering *why*. @status:impl/done
3. @fact:RULE-ATOMIC-GROUPING **Group commits by meaning** — one logical unit per commit, split mixed working trees. @status:impl/done
4. @fact:RULE-AUTONOMY **Autonomy on routine changes only** — commit and push routine work without asking; stop and ask for history rewrites, force-push, large blobs, CI/signing changes, and anything whose reversal costs work. @status:impl/done

## Files you MUST NOT touch without explicit instruction

- @fact:NOTOUCH-REFS-BOOK `refs/book/` — the user's book, read-only reference material. @status:impl/done

## Units marked `void` — do not implement

- @fact:VOID-NOT-WORK A unit whose state is `void` (`@spec/void`, or `state="void"` in a `<status>` element) **asserts nothing** and is not work. It was either split into heirs and left as a pointer to them, or cancelled with no replacement; its text survives only so its name is not reused and inbound links do not break. Do not implement it, do not plan from it, do not count it as outstanding. @status:impl/done
- @fact:VOID-TEXT-STILL-READS The trap, and the only reason this rule is needed: the prose is unchanged. A tombstone still reads like the requirement it used to be, because it was one. The marker is the only thing saying otherwise, so read it before acting on the paragraph. @status:impl/done
- @fact:VOID-FOLLOW-HEIRS If the unit names successors, they carry the claim and the work is theirs. If it names none, the claim was withdrawn and there is nothing to do. @status:impl/done

## Reading layers (per book, `refs/book/`)

@fact:reading-layers-lead vibevm's instance of the **two-process-model** flow (`spec://org.vibevm.world/two-process-model/flows/two-process-model/TWO-PROCESS-MODEL#root`) — human and agent as two processes sharing one repository; these are its reading layers, information flowing top-down, the human winning conflicts: @status:impl/done

- @fact:LAYER-HEAD **Head** (human's memory) — not your concern, but respect that it exists. Human wins conflicts with the spec. @status:impl/done
- @fact:LAYER-WAL **WAL** (`spec/WAL.md`) — volatile, rewritten each session, describes *current* state. @status:impl/done
- @fact:LAYER-SPEC **Spec** (other files under `spec/`) — stable decisions, addressable via `spec://…` URIs. @status:impl/done
- @fact:LAYER-CODE **Code** (everything under `crates/`, including each crate's own `tests/`) — artefacts. Losing them is inconvenient; losing the spec is a catastrophe. @status:impl/done

@fact:SYNC-FROM-CODE-PATH Information flows top-down. If code changes first, reconcile up via the **sync-from-code** flow (`spec://org.vibevm.world/sync-from-code/flows/sync-from-code/SYNC-PROTOCOL#root`; also `refs/book/` chapter 3) — propose a spec update, do not rewrite code back. @status:impl/done

## Hard conventions

- @fact:CONV-LANGUAGE **Language:** Rust. See [spec://org.vibevm.core/vibevm/common/PROP-000#language](../common/PROP-000.md#language). @status:impl/done
- @fact:CONV-MANIFESTS **Manifests:** TOML. One `vibe.toml` per node — the role is set by section (`[project]` ⊕ `[package]`, optionally `[workspace]`); lockfile = `vibe.lock`. @status:impl/done
- @fact:CONV-TERMINOLOGY **Terminology:** only six installable kinds — `flow`, `feat`, `stack`, `tool`, `mcp`, `lang` (the register grows only by owner amendment to `VIBEVM-SPEC.md` §4.1; `app` is anticipated). Never say "lifecycle", "phase", "goal", "plugin" (except that "plugin" == "package" in passing context). See `VIBEVM-SPEC.md` §4. @status:impl/done
- @fact:CONV-REPO-URLS **Repository URLs:** vibevm source = `git@gitverse.ru:vibevm/vibevm.git` / `https://gitverse.ru/vibevm/vibevm`. Package registry = the GitHub organization `https://github.com/vibespecs` (deliberate split-host posture — see `spec/boot/90-user.md` and [PROP-000 §7](../common/PROP-000.md#registry)). The legacy GitVerse monorepo `git@gitverse.ru:anarchic/vibespecs.git` is read-only transition state. @status:impl/done

## Uncertainty protocol

@fact:uncertainty-lead When the spec is silent on a question: @status:impl/done
1. @fact:UNC-STEP-SPEC Re-read the relevant section of `VIBEVM-SPEC.md`. @status:impl/done
2. @fact:UNC-STEP-BOOK Re-read the relevant chapter in `refs/book/`. @status:impl/done
3. @fact:UNC-STEP-ANALOGS Look at the closest analog under `refs/src/` (cargo, uv, spec-kit). @status:impl/done
4. @fact:UNC-STEP-REVIEW-MARKER If still unclear: mark the decision with `<!-- REVIEW: … -->`, pick the conservative interpretation, proceed, flag in the end-of-session report. Never silently invent semantic behavior. @status:impl/done

## End of session

- @fact:EOS-WAL-REWRITE Update `spec/WAL.md` to reflect the *current* state (rewrite, not append — it is a checkpoint). @status:impl/done
- @fact:EOS-MILESTONE-COMMIT Propose a milestone commit if work is a logical unit. For routine work, follow rule 4 above: commit and push using rules 2–3. For non-routine operations, stop and ask the user first. @status:impl/done
