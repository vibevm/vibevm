# Generated subagent types: design

Design by the architect session, 2026-11-22, for the SYS-AGENT-TYPES entries; the spec unit is PROP-076 #SYS-WORKER-TYPES.

GENERATED SUBAGENT TYPES (PROP-076 #SYS-WORKER-TYPES, new unit, lead-authored)

Problem. Same-type workers of one wave cache their system prompt once only when their prompts are byte-identical. `vibe agent claude --worker-prefix` (#SYS-WORKER-PREFIX-DELIVERY) provides that in print mode only. Interactive Claude Code reads custom subagent types from <project>/.claude/agents/<name>.md at session start. AGENT-TYPE-PROBE.md verified that a 250,249-byte body is accepted whole and that same-type workers share the cache. Neither `--agents` JSON nor a SubagentStart hook (10,000 characters) can carry a lane-sized body. So the types are files, written before the session starts.

Data model. It is harness-neutral and lives in vibe-workspace, in the new public module `vibe_workspace::system_prompt::agent_types`.
- `AgentTypeSpec { name, description, preface: Option<String>, body: AgentTypeBody }`.
- `AgentTypeBody` is `ProjectLane` (the lane file the parent launch passes) or `PackageFile { group, name, path }` (one file in an installed package slot).
- The spec has no harness detail. `render_claude_agent_type` is the first renderer. A later package-declared type is one more spec with a `PackageFile` body, so the model needs no change for it.

Two built-ins ship, in this order:
- `vibe-worker`: a fixed preface, then the composed lane bytes.
- `vibe-executor`: the installed `org.vibevm.world/working-rules` page `rules/worker.md`, with no lane. This keeps #SYS-WORKER-EXCLUSION.

Version 1 has no project-declared types, because `SystemPromptSection` is `#[serde(deny_unknown_fields)]` and a new key would break released binaries.

Files and ownership. Each type is written to `<node>/.claude/agents/vibe-<name>.md`.
- The frontmatter has only `name` and `description`, so model, effort and tools are inherited.
- The first body line is a fixed generated marker. Ownership is that marker and nothing else.
- A file without the marker is `foreign`: it is named, left byte-identical, and never overwritten or removed.
- All I/O goes through `vibe_safefs::Project` (`open`, `read_file`, `write_atomic` (creates parents no-follow), `remove_file`).
- The deploy/pkgskill receipts are not used. Those receipts are machine-local state for package artifacts, while these files are re-derived at every launch. A marker in the file is portable and self-describing, the same idiom `BOOT_IGNORE_BODY` uses. No other vibe code writes `.claude/agents` today (verified).
- Errors use a dedicated `AgentTypesError` (`ForeignIgnoreFile`, `Io`). `WorkspaceError::Io` is not used, because its message cites PROP-007#nesting and "check that the path exists", which would misdescribe a refusal.

Not committed. Before the first type file, vibe writes a self-ignoring `.claude/agents/.gitignore` (`/.gitignore`, `/vibe-*.md`) and never touches the root `.gitignore`. Why not `.git/info/exclude`:
- it follows the #SYS-ARTIFACTS-NOT-TRACKED precedent;
- it needs no git process;
- it works in member nodes, linked worktrees and non-git directories;
- the developer can see it.

If the ignore file exists but is not vibe's (first line differs), every write and check refuses and names it. In check mode, an ignore file that is missing or out of date turns otherwise-current types into `stale`, with that reason.

Sync algorithm (one algorithm for both modes):
1. Read every existing file and resolve every body.
2. If any body resolved, validate the ignore file. A foreign one is Err before anything is written. In Write mode, write the ignore file when it is absent or different.
3. Give each spec an outcome, in spec order.

Statuses (one enum for write, check and JSON): created, updated, unchanged, removed, current, stale, missing, foreign, unavailable. An unresolvable body is `unavailable` with a reason. If a vibe-generated copy exists, Write removes it (`removed`) and Check reports it `stale`. This is how "never an older lane" (#SYS-SOFT-LAUNCH) holds.

Body sources.
- The lane comes from the launch's own `prompt::compose_launch_flags(ctx, selected, HarnessKind::Claude, …)`, so there is no second composition. It counts as present only when `undeliverable` is None and `lane` is Some.
- When a personal layer applies, that lane is the content-addressed, git-ignored `boot/local/AGENT-<hash>.md` from `local_artifact::publish` (PROP-074 #LAYERED-PREFIX-LOCATION). The worker type is then deliberately byte-identical to the parent's lane, personal layer included. It is git-ignored exactly like that local artifact, which is already in the node tree.
- The executor page is located through `vibe.lock` at the WORKSPACE root (`selected.workspace.root`, `Workspace::lockfile_path`), not the node. The slot is `vibedeps::slot_abs_path`, or `in_place_slot_abs_path` when `is_in_place_slot`.

Surfaces.
- `vibe prompt types [--node REL] [--path DIR] [--check] [--json]`, wire `vibe-prompt-types/1` (`cli-prompt-types`, `schemas/prompt_types.jtd.json`).
  - It sits under `vibe prompt` because `vibe agent <word>` is the launcher namespace.
  - It reads the lane on disk and does not build, like `vibe prompt flags`.
  - Write mode fails on a foreign file. `--check` fails on stale, missing or foreign.
- `vibe agent claude` regenerates just before exec on the delivering path (one generation per launch = one wave). On every Claude soft or opted-out return that has a node, it withdraws only the vibe-generated `vibe-worker.md`.
- A launch never refuses over types. A foreign file or an error is a stderr warning.
- Generation needs the node's existing system-prompt opt-in. There is no new key.

What stays true.
- Only a vibe surface produces a worker prefix, and from the same lane file.
- Quiet workers and packet executors never get the lane: the lead picks `vibe-executor` or a non-vibe type. Packets sent to `vibe-executor` must still carry `##subagent-quiet-clause`, because Claude Code gives subagents CLAUDE.md memory whatever the type body is.
- A `--worker-prefix` session must not use `vibe-worker` (the lane would arrive twice) or `vibe-executor` (the flag gives every subagent the lane).
- Parent and worker do not share a cache.
- A type written mid-session takes effect at the next session.

Lead probes before shipping (recorded in the unit):
- a definition the size of the real lane (≥ 320 KB, launch.rs speaks of ~310 KB);
- where Claude Code looks for project agents when the session starts in a node subdirectory and in a member node below the git root.

Order: SAT-1, then SAT-2 and SAT-3 in parallel, then SAT-4 (after the lead's wire codegen), then SAT-5, SAT-6, SAT-7. The lead does the docs and CHANGELOG at the end.
