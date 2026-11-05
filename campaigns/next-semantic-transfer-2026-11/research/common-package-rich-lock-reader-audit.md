# Rich lock reader fallbacks — actionable checkpoint

Read-only current-source audit; no tests, Cargo, codegen, product/Git edits or agents. Primary is changing rich lock8 separately. Findings below are about failure handling, independent of its pending field names. Eighteen exact source snapshots and SHA-256 are in `/tmp/nst-rich-lock-reader-evidence/sources.json`. This is a finite actionable list, not a universal lock framework proposal.

## Before enabling the rich writer: fix these two independent empty-lock fallbacks

### 1. Library install loader: unsupported epoch becomes a new empty lock

`crates/vibe-install/src/fetched.rs:52-65`, `load_or_empty_lockfile`:

- Missing path → `Lockfile::empty` (intentional initial-project behavior).
- `Lockfile::read` returns `UnsupportedLockfile` → discards the error and returns the same empty lock.
- Other parse/I/O errors propagate. Do **not** describe this as swallowing every parse error.

Actual chain: normal/prepared install entry → `plan.rs::plan_prepared_*` → `load_or_empty_lockfile` → root/freshness/solve selection → ordinary apply and lock recording. Dropping a present unsupported record removes its pinned graph, original request/default basis and ownership context before planning the replacement. A structurally readable future schema can become a fresh resolution rather than an explicit incompatibility.

Minimal owner fix: only genuine file absence can initialize empty. Faithful schema7 support must be a successful typed core read, not this fallback. Present unknown/future/rich-schema failure must propagate before solve/apply. If historical older lock regeneration remains desired, it needs an explicit recognized legacy-only policy; `UnsupportedLockfile` of arbitrary epoch is not that policy. Revise this function's existing comment/test which currently mandates empty-on-unsupported, rather than accidentally retaining it beneath the new core reader.

### 2. Orchestrator install snapshot: its own unsupported fallback can bypass the library entirely

`crates/vibe-orchestrator/src/install/mod.rs:274-290`, inside `execute_prepared`:

- Repeats the `UnsupportedLockfile` → empty behavior separately.
- `299-318`: if workspace requirements and the synthetic empty lock's root snapshot are empty, enters `EmptyWorld` and calls `regenerate_native_world(..., &[], ...)`.
- Otherwise the synthetic snapshot is handed to source/planning/completion owners. The library reader later reading strictly is insufficient to protect the earlier empty-world branch.

Actual CLI chain: install direct command → `execute_prepared` → this snapshot → either EmptyWorld (boot/native-world regeneration, optional admitted-manifest publication) or prepared library planner. Whole update also delegates the install substrate. Programmatic orchestrator callers share the boundary.

Minimal owner fix: strict admission of a present lock before choosing EmptyWorld/Fresh/new planning. Reuse a faithful7/8 result; only actual NotFound may become empty. No unconditional world from an unrecognized record. This must be fixed independently from item 1.

**Current timing correction:** the snapshot is after `manifest_mutation.apply` at line254, but current CLI `commands/install/resolver/git_source_flag.rs:35-114` now keeps the --git delta in memory; its publication was deferred by accepted A0. My initial live message suggesting that current CLI callback already writes the manifest was stale. The concrete current risk is the later EmptyWorld/Ready publication after the empty fallback. For general supplied callback safety, admission before a potentially effectful callback is still the clean owner boundary; do not claim the current CLI --git callee writes early.

**Scope of the schema counterexample:** current core `Lockfile::read:517` parses the full typed structure before checking epoch. An unknown field may therefore produce a parse error, which items1/2 already propagate. The dangerous case is a structurally readable lock with unsupported epoch (including a future epoch that uses the known rich shape once lock8 exists). Do not weaken all parse/schema errors based on a guessed `configuration` field sniff.

## One additional live routing fallback

### 3. Remap composition treats unreadable project lock as no project-required remap packages

`crates/vibe-cli/src/composition/remap.rs:389-399`, `project_required`:

`Lockfile::read(...).map(...remaps...).unwrap_or_default()` discards **all** read/parse/schema errors. `install:296` → `accepted_offers:349` → `project_required` → accepted package assigned ProjectPackage or UserPackage layer at `377-383`. A present unreadable lock therefore changes source-routing precedence instead of reporting unavailable project meaning. It does not itself overwrite the package lock, but its locator precedes package source resolution.

Minimal owner fix: make this local helper return `Result<Vec<String>>`, preserve the no-project/no-file empty cases, and propagate existing-lock failure through `accepted_offers`/composition install. Read the required project layer before any consent mutation if that call may change remap state. Do not replace a corrupt project's required layer with the user layer. The current lookup uses its supplied project root/current-dir selection; broader root discovery policy is outside this audit.

## Post-durability reporting, not a pre-effect empty-world fallback

### 4. Ready tail suppresses failed read of the just-written lock

`crates/vibe-orchestrator/src/install/ready.rs:277-296`, `finish_ready`:

`if let Ok(new_lock) = Lockfile::read(...)` emits ClosureDiff only on success; any error is silently skipped and the successful InstallRun continues. Both ordinary Ready and resumed Ready completion call it.

This does **not** fabricate an empty graph or re-resolve; apply has already completed. Its risk is concealing an unreadable/mismatched just-written rich record and dropping the promised before/after explanation.

Minimal owner recommendation: retain the already-validated written lock value for the diff when available, or surface a precise post-durability verification diagnostic/failure without pretending the completed apply was rolled back. Do not route this error through pre-commit undo. Lower priority than items1/2 for preventing loss of retained authority, but it should not be silently called proof of faithful writer/read compatibility.

## Checked consumers that already propagate present-lock failure

No equivalent empty-on-parse/schema fallback found in these directly inspected owners:

- `vibe-workspace/src/install/bootgen/materialised_read.rs:14-33`: only filesystem NotFound yields empty durable world; nonregular lock and all typed read errors refuse. Good existing pattern for the missing-versus-invalid distinction.
- `vibe-orchestrator/src/install/resume.rs:171`: parked slot resume propagates lock read, then builds its provisional world and admissions.
- `vibe-orchestrator/src/world/mod.rs:370`: effective-world load propagates present-lock failure. Its explicit PreClean/no-vibedeps branch can omit dependencies without reading the lock; that is a separate cleanup policy, not discarded parse failure or ordinary configured install evidence.
- `commands/update/inputs.rs:126`: present read errors propagate; missing lock refuses.
- `commands/reinstall/inputs.rs:131`: present read errors propagate; no file intentionally regenerates authored boot only.
- uninstall, global MCP lock rewrite/adoption, project-app requirement reads, list/tree/show features/subskills/purls/effective, registry sync/vendor and installed package bins propagate a present lock read failure. Their `exists`/`is_file` no-file branches are not evidence that parse/schema errors are ignored.
- `vibe-mcp/src/context.rs:140` returns empty only if path does not exist; present read errors propagate. Agent-skill projection similarly uses `?` for a present lock. Its separate malformed slot-manifest tolerance is not a Lockfile::read fallback and was not expanded here.

For mutating owners using `path.exists()`, note the small file-kind caveat: false is not a proof of genuine absence (broken symlink/metadata access failure). Borrow the boot reader's owner-local `symlink_metadata`/NotFound distinction where a new empty world could authorize writes. No general filesystem framework is needed.

## Ancillary read-only hits: keep separate from primary writer scope

- `vibe-doc/src/citations/sources.rs:318-319` converts lock failure to None with `.ok()?`; `SpecSources::instance_of:199-205` can then fall back to a store instance. This can misrepresent a project's selected source on unreadable lock, but is not install/boot writer activation. A future owner-specific change should distinguish “no locked instance” from “locked authority unreadable”; do not call it harmless faithful rich replay.
- `vibe-check` lock_slot_hash/lockfile_files/local_source_freshness return early on a failed read. These are secondary diagnostic cells; `manifest_validity.rs:61-68` explicitly reports the lock read error. They neither create an empty lock nor apply an unconditional world. Do not force each cell to duplicate the same error merely to eliminate every `Err` branch. Individual selected-check behavior is outside this writer audit.

## Minimal discriminator panel for primary/root (not run)

1. Valid old schema7 reads faithfully through both library plan and actual CLI install, retaining graph/provenance rather than becoming empty. Valid new rich8 reads through the same two paths.
2. A present syntactically/structurally valid future-schema lock must refuse in both owners. Use an empty authored workspace with nonempty retained lock roots to exercise the orchestrator's former EmptyWorld hole; compare manifest/lock/slots/boot/launchers before/after.
3. Malformed present lock remains an error. No lock file remains the ordinary supported empty-project case. Distinguish these three outcomes explicitly.
4. In remap composition, a malformed/future project lock must not silently demote its accepted required packages to UserPackage precedence.
5. A controlled post-apply lock read failure must not disappear without diagnostic or be labeled a successful rollback; keep that reporting cut separate from before-effect refusal.

Priority for rich writer integration: **items1+2 first**, then item3's independent routing layer; item4 is a distinct completion-reporting decision. This checkpoint is actionable source evidence only; root owns scope and acceptance.
