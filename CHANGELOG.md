# Changelog

This file is a curated chronicle of what landed when, organised by milestone rather than commit-by-commit. The single source of truth for "what changed in this commit" is `git log`; this file is the source of truth for "what does this milestone mean".

Format roughly follows [Keep a Changelog](https://keepachangelog.com/), grouped by milestone. The "Unreleased" section accumulates work-in-progress.

---

## [Unreleased]

- **Refresh the user home's registry cache; publish updates the index; a stale `by-name` file no longer picks the version.** `vibe registry sync -g` — and a plain `vibe registry sync` outside a project, which used to refuse — refreshes every package clone cached for the registries of `~/.vibe/registry.toml` (the set a global install resolves through) to its newest release tag and re-asks each registry's index handshake, under the settings root `$VIBE_SETTINGS` names; `--json` reports `{ok, command: "registry:sync", scope: "user", home, refreshed: [{registry, ref, via, clones}], skipped}`, the human form draws one row per registry and the trailer `✻ refreshed N registries · <time>`, and `--offline` refuses. `vibe registry publish --index-checkout <path>` (or `[index] checkout = "<path>"` in `~/.vibe/registry.toml`) adds the pushed release to a local checkout of the git-hosted index through the index's own writer, rewrites `repomd.json` and commits `index: <group>/<name>@<version>`; `--push-index` pushes the commit, `--dry-run` names what it would add, and the report carries `index_checkout: {path, added, committed, pushed}`. The index client checks each `by-name/<name>.json` it reads against the `sha256` `repomd.json` lists for it and, on a mismatch, enumerates the package's tags instead of choosing from a copy a cache still serves (B-227).

- **The machine progress stream names the requirement a subtree installs.** Under `--progress-format json`, the `start` line of the task that installs a requirement of another application package now carries `requires` (`key`, `scope`, and `step` when a step asks for it), in the words of the plan's `dependencies[]`, so a desktop client keys that subtree to the plan without parsing its label. The label is unchanged, no other line carries the member, and the protocol stays `vibe-progress-event/1`.

- **`vibe check` names the field a manifest finding is about.** A refusal of the application validators (`[application]`, `[[installer_options]]` with its steps, wizard, launchers, icons and requirements) now carries the TOML path of the field it refuses, spelled as the manifest writes it with zero-based array indexes: `installer_options[1].steps[0].sha256`. `vibe check --json` reports it as the finding's `field`, with `line` and a new `column` pointing at that field in `vibe.toml`; a TOML parse error gives the parser's `line` and `column`, and a `field` only when the parser names a table outside any array. Human output shows the field after the path, dim: `at   vibe.toml:31 · installer_options[1].steps[0].sha256`. The messages themselves are unchanged. The desktop shell's package editor uses this to put each finding under its field.

- **`vibe init --kind app` makes a package that plans.** The scaffold of an application package used to stop at `[package] kind = "app"`, which `vibe app plan` and `vibe install -g` refuse for want of an `[application]` slot and an installer option. It now also writes `[application] id = "<name>"`, one `[[installer_options]]` `main` for the platform and architecture you run it on with one `recipe` step `build`, the script that step runs (`recipes/build.sh` with `posix_sh`, `recipes/build.ps1` with `windows_powershell` on Windows) and an empty `icons/` directory, in the root form (`--type package`) and the nested form (`vibe init package`) alike. `vibe check` passes on the result and `vibe app plan` plans it from a registry without an edit. No launcher is written and no new question is asked: the application id is the package name.

- **One look in the terminal.** Every human-readable output now draws through one module, `output::language`, built from PROP-067: a stream of blocks separated by one blank line, a one-column gutter of status glyphs (`●` running, `✓` done, `✗` failed, `○` skipped, `✻` the trailer, `?` a question, `!` a warning), metadata after ` · ` in a dim role, and five colour roles mapped to the terminal's own attributes. A running step shows its last five lines under `│` with `(l to show the live log)`, and collapses when it ends to `✓ build  Build vim · 2m 10s · 412 lines`; a failed step keeps its last thirty lines and names `exit N` and its log; the command's own task no longer leaves a row, because every command that did work ends with `✻ <what happened> · <steps> · <time> · log <run directory>` (a failure with `✻ failed at <step>`, a parked run with the command that resumes it). The plan opens with a rule `── plan for <key> ──` and lists its steps in measured columns with their requirements as `└` children; the wizard's answers collapse into one list above it; a refusal is `✗ <code>  <what was refused>` with `why`, `fix` and `see` label columns; a warning is one `! <sentence>` line. `vibe remap list|explain`, `vibe app list|runs|work|running|launchers|installers` draw the same way, identifiers without backticks. The new global `--plain` writes each block once, when it ends, with no colour, no cursor movement and no live fold; `NO_COLOR` and a redirected stream drop every attribute, and a redirected stream is no longer wrapped or cut. `--json`, `--progress-format json` and the documents are unchanged. Golden captures live in `crates/vibe-cli/tests/golden/display/`. The rest of the CLI followed: project `install`, `update`, `uninstall`, `cache import`, `init`, the `vibe self` verbs and a foreground `vibe app start` end with a trailer, `vibe self ls|current` is a measured list marked `[active]` and `[running]`, `vibe check` lists its findings as blocks under a rule, and every question (`init`, install and uninstall confirmations, `mcp`, `self remove|gc`, `clean`, `doc shell`, the remap consent offer) is one `?` block that collapses to `✓ <prompt>  <value>`; `vibe tree --plain` draws its branches from the same module, in ASCII where the terminal has no vocabulary, and `--quiet` keeps every one-line summary scripts already read.

- **A taskbar's worth of runtime for desktop clients.** `vibe app running` lists the launchers a detached `vibe app start --json` left running, with process ids and start times, and prunes the ones that exited; `vibe app stop KEY|PID` asks one to end, waits five seconds and then ends it without asking. On Windows the ask reaches a `tui` or `command` launcher started from another console: `vibe app stop` attaches to the launcher's console for the console break instead of reporting every stop as `ended without asking`. `vibe app watch --json` prints one line per change of the home (`installed`, `updated`, `removed`, `launchers-changed`, `collections-changed`, `started`, `exited`) with the new revision, read from a small events journal beside the revision counter, until its input closes. An update now carries your collections, pins, overrides and start counts over to the new version's launchers, and a rollback carries them back. `vibe app launchers --export-system` writes the system's own menu entries for the launchers you would see (`.desktop` files and hicolor icons on Linux, Start Menu shortcuts on Windows, `.app` wrappers on macOS), records every path in `applications/exported.json`, prunes on the next export and removes them with `--remove-system` or the package's uninstall. `vibe app modify` takes `--keep-work` like `install -g`.

- **Steps read as a tree.** The plan `vibe app plan` prints and the plan an attended install shows first now draws its steps as a tree: one node per step with its number, its `run` / `skip` word, its `<id> (<kind>)` and its title in columns that line up across the whole plan, the skip reason and the needs on continuation lines under the label column, and each package a step requires as a child node of that step — `installed` when a version already on the machine satisfies it, otherwise installed first, with its own plan headed `plan for <key>  (required by <package> for step <id>)` so it is no longer read as the plan of the package you asked for. The step panel nests the same way: a required package's whole install is a subtree under its own `requires` row, and the folded build output sits under the running step (the panel's exact drawing is the one described above, under PROP-067). The panel's `Ctrl-C` hotkey now works on Windows too: raw mode takes the console's own Ctrl-C away, so the panel generates `CTRL_C_EVENT` for the console it shares with the running child, which is the same interruption the signal handling already records as `interrupted`. `--json` plans and plain, quiet and redirected output are unchanged.

- **Remap: every artifact source is replaceable.** A vibe installation now runs where the public hosts are unreachable, filtered, or forbidden by policy. A `[[remap]]` row says what it matches (`sha256`, `package`, `registry` or a URL `prefix`), what it redirects to (`url`, a local `path` searched as `<path>/<sha256>/<filename>`, or `deny = true`), and optionally where it applies (`scope = { package, step }`) and why (`reason`). Rows come from `~/.vibe/settings.toml`, `.vibe/settings.local.toml`, the existing PROP-002 `[[mirror]]` / `[[override]]` tables — read as rows now, with nothing a user wrote stopping working — and the invocation (`--remap <file>`, repeatable, or `VIBE_REMAP=<file>[,<file>]`); the most specific key wins, then the longest prefix, the narrower scope, the higher layer and the later row. `[policy] unmatched = "allow"|"deny"` refuses a fetch no row matched, and the strictest policy of every layer applies. One resolution point in `vibe-core` answers every fetch path — registry clones and their mirror chain, the index and raw-file endpoints, embedded sources, git-source dependencies, the `sources`, `archive` and `native` inputs of an application step, `vibe self` distributions, the documentation shell, and `[proxy] https|http|no_proxy` in `~/.vibe/config.toml` for every HTTP client and every child `git` — and the install scripts honour `VIBEVM_RELEASE_ORIGIN` for every download they make. **A remap changes only where bytes are looked for:** each candidate is verified against the same digest as before, so a row can make a fetch fail or succeed and can never change what is installed. A refusal is `remap_denied` (exit 2) before any connection, naming the row and its reason; `steps.json` and the run record say which candidate served each input, while the package's own URL and the lock stay untouched. `vibe remap list [--json]` shows the active rows by source with the scheme, private/public/local classification and the `unencrypted` / `broad` / `unknown` notes computed here rather than taken from the row, plus a per-host footer; `vibe remap explain <url|sha256:…> [--json]` prints the whole chain for one fetch and the digest that will be checked. The documents are `vibe-remap/1`.

- **Remap packages, and the question they ask.** A set of remap rows now travels as an ordinary package. `[package] kind = "remap"` is the ninth installable kind (VIBEVM-SPEC §4.1): it carries `[[remap]]` rows, the optional `[remap] description` / `priority` above them, an optional `[policy]`, and nothing else — a `[[binary]]`, a `[boot_snippet]` or a dependency in one is a validation error naming the table, because accepting a redirection must never be a way to run someone's code. `vibe install -g <group>/<name>` puts it in the user home and a project names one in `[requires.remaps]`, pinned in `vibe.lock`'s own `[[remap]]` table so every clone sees the same list. **Installing it records an offer, never an obligation:** nothing applies until that user accepts it. The default is a question — an attended terminal shows the package, its description, every row with the scheme, private/public/local classification and the `unencrypted` / `broad` / `unknown` notes, the per-host footer, and asks `apply?`; an unattended run refuses with `consent_required` (exit 2) naming `vibe remap accept <package>` and carrying the `vibe-remap-offer/1` document a graphical client renders. The flags answer it without a terminal: `--accept-remap` (bare, or repeatable with a package) and `--reject-remap` for this run only. `vibe remap accept|reject|enable|disable <package>…` manage the ledger `<home>/remap/consent.json` (`vibe-remap-consent/1`), bound to the package's content hash so a new version is a new offer; `vibe remap list` shows each package's `offered` / `accepted` / `rejected` / `disabled` state and, per row, which packages offer it, derived into `<home>/remap/providers.json` (`vibe-remap-providers/1`). Uninstalling a package that offers accepted rows never silently changes what the machine fetches: the rows that would stop applying and the rows another accepted package still offers are shown, and `keep these N rows as local rows?` — `--keep-remap` copies the orphans into your own `[[remap]]` table with `from = "<package>@<version>"` and the original reason, `--drop-remap` lets them go, and an unattended run without a flag refuses. `vibe remap test [--json]` walks what a project's lock or a home's applications and distribution manifest declare, resolves each through the same Locator every fetch uses, and asks each winning target whether it answers — a `HEAD`, a `git ls-remote`, a `stat` — downloading nothing, exiting 1 when any target is unreachable and recording the answer so every later view states it with its date (`vibe-remap-test/1`). For a closed network, `vibe remap export <package> --out <dir> [--with-artifacts]` writes a directory that is both a `file://` registry and an `artifacts/<sha256>/<filename>` tree with a `MANIFEST.json` (`vibe-remap-bundle/1`); `vibe remap import <dir>` installs the package from it as an offer, and `vibe cache import <dir>` verifies every digest and places the bytes in the machine store and the application download cache, never overwriting an entry that already verifies. Where bytes actually came from is recorded in the sidecar `vibe.lock.provenance` beside the lock rather than on a `[[package]]` row, because the lock refuses unknown fields and an older vibe must keep reading it. And the `<sha256>` cross-run cache that `sources` inputs had now covers `archive` and `native` inputs too, so a second install after `vibe app work --clean` fetches nothing.

- **One prompt theme for the wizard.** Every question `vibe` asks — the platform, architecture and installer of an application install, the setup type, the feature tree, each parameter, the license, and every confirmation in `clean`, `uninstall`, `cache clean`, `init`, `mcp`, `skill`, `reinstall`, `update` and `self` — now shares one theme built on the progress renderer's own glyphs and its colour decision: `?` before an unanswered question, a cyan `❯` on the row under the cursor, `◉` / `○` on a multi-choice row, and `✓ Platform  Windows (windows)` once it is answered, so the answers and the plan that follows them read as one block. A list's rows are aligned columns — the name, then the id dim, then `[detected]` in green, then the description — so `Windows  windows  [detected]` replaces the old `Windows (windows)  [detected]` run together, and a nested feature keeps its indent inside the name column. Each question is spaced off the text above it and the first list carries one dim key legend (`↑↓ move · enter choose · space toggle · esc cancel`). Where colour is off — a pipe, `NO_COLOR`, a dumb terminal — the columns and the glyphs stay and only the colour goes.

- **The launcher catalog a desktop is drawn from.** `vibe app launchers --all --json` is now a catalog rather than a list: every launcher of every ready application carries a stable key `<group>/<name>@<version>#<launcher id>`, the collections it sits in, where it is pinned, who hid it, its effective name and icon beside the package's own, and how often and when it was last started; each installation carries its `hidden` / `held` state; and the document carries the home and a `revision` the home bumps on every change, so a client polls `vibe app launchers --revision` instead of refetching. The home keeps the user's own arrangement in `applications/launchers.toml`, edited only through commands: `vibe app collection create|rename|delete|add|remove|order|list`, `vibe app pin|unpin KEY [--to taskbar|favorites]` and `vibe app launcher KEY --hide|--show|--name "…"|--icon PATH|--reset` — a name or icon of your own survives update and reinstall, and a hidden launcher stays in the catalog marked `hidden_by: "user"`. `[application] menu_group` seeds a collection of that name on install and never reorders or reinstates what you arranged; an uninstall takes the package's keys back out. `vibe app start KEY --json` starts the launcher detached and answers its process id, command line and log instead of waiting for it, so a start menu is not blocked by the program it launched; without `--json` the start stays attached exactly as before. The documents are `vibe-app-launchers/2`, `vibe-app-launchers-all/2`, `vibe-app-collections/1`, `vibe-app-revision/1` and `vibe-app-start/1`.

- **A step panel for application installation.** On a terminal, `vibe install -g`, `vibe app update` and a project install that materialises application requirements now draw one row per step — `<id> (<kind>) <title>` with a spinner and elapsed time, `✓` and the elapsed time when it finishes, the reason when it is skipped — with a byte bar for every download whose size the package declares or the server announces, and the long build output folded into the last five dim lines under the running step instead of scrolling past. `l` replaces the fold with the live stream and folds it back; the run log still gets every byte, and a failing or interrupted step prints the last 30 lines of its log before the error names the step and `--resume`. `--log <auto|live|tail|quiet>` and `--log-tail N` (fallbacks `VIBE_APP_LOG`, `VIBE_APP_LOG_TAIL`) choose what reaches the terminal; redirected output, `--quiet`, `--json` and `--no-progress` keep the flat records and the live stream exactly as before.

- **Kept work directories.** `vibe install -g` and `vibe app update` take `--keep-work`: a successful run leaves its work directory `cache/app-work/<key>/` in place with every step recorded `done`, names it (`work directory kept: <path>`, and `work` in the JSON report), and a later `--from-step ID` restarts the build inside that tree without fetching a byte again — the point of it for a C++ source build whose `sources/` took minutes. A fresh run of the same key still replaces the directory and says so first; `--resume` on a directory whose steps have all run refuses and names `--from-step`. `vibe app work [PACKAGE] [--json]` lists what is kept, with each directory's state, change time and size split over `sources/`, `out/`, `inputs/` and the rest, and `vibe app work PACKAGE --clean` / `--clean --all` deletes it again; neither touches the download cache or the run records.

- **Readable plans and wizard answers.** `vibe app plan` and the plan shown before an attended install now print the package on its own line, the installer, features, download and plan-level requirements as aligned fields, and every step as a numbered block with its title, skip reason, needs and the requirements it pulls on indented lines; `app install` separates each plan with a blank line, and the platform, architecture and installer questions ask for one named thing each instead of a sentence.

- **Fix: a developer's embedded registry no longer hides application packages.** A source-installed `vibe` (`vibe self`, origin `external`) treats its checkout's `vibevm/vibepacks` as the embedded registry; `vibe install -g`, `app plan` and the other application verbs used it as the only source and answered `not in the registry` for every package it did not carry. Per PROP-030 `#precedence` the embedded registry is now first in the chain and a package it lacks falls through to the declared registries; an explicit `--registry` stays the whole source. Release bundles (origin `binary`) were never affected.

## [1.0.8] — 2026-10-31

**Upgrade notes.**
- Every `[[installer_options.launchers]]` row must declare `kind` (`gui`,
  `tui` or `command`); an application package published without it is
  refused by this version. `org.vibevm.apps/vim@1.1.2` and
  `org.vibevm.apps/mingw-w64@15.2.1` carry it; both declare
  `min_vibe_version = "1.0.8"`. The registry cache refreshes an hour after
  its last pull — `vibe registry sync` fetches the new tags at once.
- A project `vibe.lock` that records application requirements carries the
  new optional `required_by` field; vibe 1.0.7 and older refuse such a lock,
  while every older lock still loads here.
- A project resolves one version per package, so two applications requiring
  different versions of one toolchain cannot share a project; the user home
  keeps several versions of a slot side by side.
- The license of a required application is asked for when that requirement
  is planned, after earlier requirements may already be installed;
  `--accept-license` admits every requirement unattended.
- Application index records written by 1.0.7 (`vibe-user-applications/1`)
  are refused, not migrated (B-216).

- **Start-menu launchers and desktop entries (PROP-059 `#menu`,
  `#desktop-entries`).** Every launcher now declares `kind` (`gui`, `tui` or
  `command`); `terminal` follows from it and a disagreeing value is refused.
  A launcher appears in a graphical client's menu only with `menu = true`;
  `[application] menu_group`, `categories` (the freedesktop main categories)
  and `keywords` set the group and defaults, and an icon is a PNG/SVG path or
  an icon set (`svg`, `png` by pixel size, `ico`, `icns`, `symbolic`,
  `maskable`). Launchers also carry `generic_name`, localised `names` and
  `descriptions`, `window_class`, `mime_types`, `actions` and the `{file}`,
  `{files}`, `{url}`, `{urls}` placeholders. `launchers_from` derives launchers
  from the payload's `.desktop` files and hicolor icons after installation, an
  authored row of the same id overrides field by field, and the index marks
  each row `authored` or `desktop-entry`; records written before kinds load as
  `tui`/`gui` from `terminal` with `menu = false`. New: `vibe app launchers
  --all [--menu]`, `vibe app icon PKG --size N [--prefer …]`, `vibe app
  import-desktop FILE…`, and the `application_icons` check of `vibe check`.
  `describe`, `app list --json` and `app launchers --json` carry the new fields
  with icon files resolved to absolute paths. A package-relative icon (of
  `[application] icon` or of a launcher, not `$`-templated) is a file of the
  package tree: install copies it into `<home>/applications/icons/<key>/`
  keeping its relative path, for owned and external results alike, and the
  listings resolve it there; `modify`, update and reinstall refresh the copies,
  `uninstall` removes them, and `storage move` leaves them with the home.
- **Application requirements in projects (PROP-065 `#app-dependencies`).** An
  application package that a project depends on and whose installer option or
  steps require other application packages now pulls them into the ordinary
  resolution: each required package is a transitive `vibe.lock` row whose new
  `required_by` field names the requiring row and the scope (`build` /
  `runtime`); locks without the field load unchanged. The project home installs
  a requirement from its lock row only (offline-capable, never from a
  registry), and removing the dependency row removes the requirer and its
  orphaned requirements from the project home (`#uninstall-tree`).
- **Licenses of requirements (PROP-065 `#license`).** A required package that
  declares a license is accepted exactly like the package that requires it:
  `--accept-license` admits it unattended, an attended terminal asks once, and
  `--assume-yes` alone is a planning refusal naming the requirement.
- **Agent steps see their requirements.** The child of an `agent` step receives
  the same requirement-first PATH (and `VIBE_APP_USE`) a recipe step of the same
  option receives (PROP-065 `#app-dependencies`, PROP-064 `#envelope`).
- **Native payload separators (B-223).** Removal plans, uninstall summaries and
  every other owned-payload path are joined component by component, so Windows
  prints `opt\apps\vim\…` instead of `opt\apps/vim/…`. Every `vibe app` verb
  that selects a home takes `-g` / `--global` with the `app list` semantics, and
  `not_installed` names the home it searched (`the user home` or `the project
  home <path>`) and, for a project home, hints `use -g for the user home`
  (B-225).
- **Busy handler executables are retried.** A lifecycle handler whose spawn
  hits `ETXTBSY` (a just-written script still open in a concurrently forked
  child) is retried up to 20 times, 25 ms apart, before the spawn is reported.

- **User applications rebuilt on a clean contract (PROP-059).** `vibe install -g`
  installs an app package through its author-declared platform / architecture /
  installer options — shell commands, ZIP and tar.gz payloads, Windows EXE and
  MSI, or a build from pinned sources (`url` or package files, verified by
  digest, cached for `--offline`, run through a `sh` / PowerShell recipe).
  Launchers run natively; every public command gets a tiny native proxy in
  `~/.vibe/opt/bin`; several providers of one slot install side by side with
  `vibe app use` for the global default and `VIBE_APP_USE` for a per-shell
  choice; `vibe shell [--use SLOT=KEY]` opens the parent shell with the
  applications on PATH; `vibe storage` adds, attaches, detaches and lists
  storage libraries, moves portable installations between them, and `gc`
  removes what an interruption left behind. A missing disk is reported as
  unavailable, never repaired around. Durable state is one index file, one
  `storage.toml` and plain directories.
- **Upgrade note.** The 1.0.7 application index (`vibe-user-applications/1`,
  under `~/.vibe/applications/`) is not migrated: remove it and reinstall the
  applications. The retired source-versus-binary bridge model
  (`[application_source]`, `installer_package`, `commands`, `distribution`,
  `--from-source`, `--binary`, `--local-source`) is gone; packages declare
  `[[installer_options]]` instead.
- `org.vibevm.doc/web` installs through a source-build recipe and puts
  `vibevm-doc` on PATH.
- **Agent hooks (PROP-064).** An installer step of kind `agent` hands a
  package-authored prompt to a coding agent the user chooses: `--agent NAME`,
  else a command an invoking program registers through `VIBE_AGENT_COMMAND` /
  `VIBE_AGENT_ARGS`, else `default` in `~/.vibe/agents.toml`. Six presets
  (Claude Code, Codex, OpenCode, Qwen Code, Gemini, Cursor) are data that a
  definition may override, so a wrapper launcher is one `command` line;
  `vibe agent list | presets | use | add | remove` edits the registry. vibe
  shows every prompt and asks before any step runs (`--assume-yes`, or a
  trusted group, skips the question), writes `prompt.md` and `context.json`
  into the step's run directory, runs the agent interactively or in print
  mode, and maps its exit and optional `result.json` to the step outcome; the
  run record keeps the envelope and transcript. Without an agent, agent
  steps are skipped with one warning at planning time; inside a hosting
  agent the step is parked as an outbox task, the command exits 75, and the
  same command with `--resume` continues once the answer is written.
- **Application lifecycle (PROP-065, PROP-059).** An installer step of kind
  `env` sets, unsets and prepends to PATH in the user's durable environment —
  `HKCU\Environment` on Windows (value kinds kept, change broadcast), a
  managed `~/.vibe/env.sh` / `env.fish` plus one managed block in the shell
  startup files elsewhere — and uninstall removes exactly what it applied;
  `vibe shell` applies the recorded entries at once. An installer option or a
  single step may `requires = ["group/name@req"]` other application packages:
  the planner shows them as `dependency` entries ahead of the needing step,
  resolves them from the home's index first and then through the same
  registries as the package, installs build-scope requirements hidden and
  runtime-scope ones exposed, and puts their commands on the needing step's
  PATH. `vibe install --hidden` installs without exposure; `vibe app expose |
  hide` change it; `vibe shell --with PKG` reaches a hidden installation for
  one session. Every installation carries a manual/auto mark (`vibe app mark`);
  `uninstall` lists orphans, `--autoremove` and `vibe app autoremove` remove
  them, a runtime dependent refuses removal unless `--cascade`, and
  `--plan-only` prints the removal plan. `vibe app modify` changes the
  installer option, features, profile or parameters of an installed version
  in place. `vibe app outdated [--json]` compares installed versions with the
  registries. `vibe app update` keeps the replaced version hidden until the
  next update (`vibe app rollback`, `--purge-previous`), and `vibe app hold |
  unhold` keep a version out of `vibe update -g --all`.

## [1.0.7] — 2026-10-04

- Source self-update freshness handles opaque untracked nested Git worktrees
  instead of rejecting them as non-file inputs. Local verification caches are
  ignored as machine outputs. Regression coverage includes the actual host
  checkout with an isolated temporary store; repeated calls reuse the sealed
  instance without rebuilding. Verbose progress distinguishes unavailable
  snapshots from missing legacy evidence. An unchanged source update explains
  why nothing needs updating and names the selected source checkout.

## [1.0.6] — 2026-10-04

- `vibe self update` checks source freshness before Cargo and reuses unchanged
  source builds without copying a new instance. An already-active verified
  binary release skips redundant activation and installation messages; JSON
  no-op output no longer requests a `vibe-index` restart. `--force` and
  `self reinstall` retain deliberate refresh behavior.

## [1.0.5] — 2026-10-04

- Package uninstall removes newly orphaned transitive dependencies using the
  installed lock graph, without downloading or reinstalling packages. Shared
  dependencies and declarations in other workspace nodes remain installed;
  pure-transitive removal is refused. Confirmation and in-place protection
  cover the complete removal set, and boot is regenerated from the pruned world.
  Empty package containers are removed while other versions and untracked
  siblings remain untouched. JSON reports the removal count, complete slot
  paths and whether the selected package was retained by another declaration.

## [1.0.4] — 2026-10-03

Global application installation accepts platform-specific launcher variants.
Each command needs a native launcher, and a package may publish several
variants without requiring a Windows `.cmd`/`.ps1` pair on Linux or macOS.
The shared ownership checks also recognize `.exe` and `.bat`; update,
rollback and uninstall retain exact file/hash protection. Windows management
entries select the correct batch/native or PowerShell wrapper; POSIX entries
keep shell argument forwarding.

Zap's tracked source installer was corrected alongside this release to report
its deployed native launchers instead of demanding Windows files everywhere.
Published binary witnesses and application protocol versions are unchanged.

## [1.0.3] — 2026-10-02

Global MCP installation now keeps preparation, dependency installation and
agent registration visibly active. Resolution messages distinguish version
selection, metadata reads, download-source lookup and in-place checkout
instead of repeating an indistinguishable "Resolving" label.

- **Agent and server selection:** interactive checkbox menus use Space to
  toggle and Enter to confirm. Install starts with only All checked; All and
  individual subsets are exclusive. Checked rows use `[+]`. Multi-server
  packages offer the same selection, with explicit `--agent`/`--server`
  filters for scripts and JSON string arrays for exact server names.
- **Selective global uninstall:** inspect native client configurations and
  initially check only agents with an exact owned installed registration.
  Show missing, changed and pending states. Remove only selected owned
  entries; retain the package while another agent still owns registrations.
  Refresh preserves each agent's chosen server subset. Drift protection and
  rollback preserve foreign entries and later edits; JSON reports whether
  the package was removed.
- **Installation scope:** the known global MCP service user-project is
  materialized automatically with a human notice naming its full destination.
  A concrete project keeps its existing confirmation policy and names the
  full target path. Automatic service installation does not bypass unrelated
  binary consent or agent/server choices.
- **Initialization:** `vibe init` can choose a root `[project]` or `[package]`
  declaration in the same scaffold. `--type project|package`, `--group` and
  `--name` provide explicit script inputs. Interactive init explains the
  group/name coordinate before asking group, name, type and remaining fields.
  Metadata flags and the chosen package kind reach the manifest; reinit
  preserves authored files and refuses silent role conversion. Interior
  hyphens such as `package-name` are supported. Legacy nested package/group
  forms remain available.
- **Prompt coexistence:** progress stays paused for the entire field-entry
  dialog, however long a person leaves it unanswered, and resumes afterward.
  Native client-directory injection isolates global MCP fixtures on Windows.
- **Release validation:** the full workspace gate now provisions its pinned
  generator and clean-snapshot dependency slot. Unix lock guards explicitly
  release inherited file-description locks; placement rejects incomplete
  manifests through typed errors. Portable fixtures retain identity, rollback,
  capability-refusal and stream assertions across operating systems.
  Windows scratch Git enables long paths and gates select Git for Windows
  Bash explicitly. Native macOS gates use an owned
  case-sensitive temporary volume and physical paths; Unicode fixtures
  exercise the same refusal rules when APFS cannot represent distinct names.
  Health preparation fixtures seal their own regular tool assets. The
  workspace also passes current Rust 1.98 Clippy without lint suppressions;
  hash recipes, wire formats and their golden data stay unchanged.

## [1.0.2] — 2026-10-01

MCP packages can declare a remote Streamable HTTP endpoint alongside the
existing local stdio binary form. `vibe mcp install` accepts a package and
optional server selector, and `vibe install -g mcp:<group>/<name>` maintains
an exact-pinned user inventory and registers the endpoint with selected
agents. Global update and uninstall follow the same inventory. Per-agent
registration receipts protect operator-owned and hand-edited entries.

Qwen Code joins the agent integration matrix with project and user scopes.
Codex project MCP configuration follows its current `.codex/config.toml`
surface. Remote URL projection uses each client's native configuration
shape; Claude Desktop reports that direct remote configuration is unsupported.
An endpoint-only package needs no local binary or build.

## [1.0.1] — 2026-09-30

A patch release with three fixes found while packaging Anatoly Levenchuk's
FPF corpus (37 hierarchical packages, 843 pattern documents) for VibeVM
and publishing it to the `vibespecs` registry. No new commands, no
manifest or wire changes; the documentation package stays at its version.

- **Markdown converter: nested list items with multibyte text no longer
  panic.** `vibe-specdoc`'s `item_unit` sliced a nested item's opener line
  at `indent + list_marker_len(line)`, but `list_marker_len` already counts
  the indent; on a nested item whose text starts with a multibyte character
  (`  * \`ΓFoldRef.edition?\``, FPF-Spec Part G) the slice landed inside the
  character and `vibe install` aborted with «byte index is not a char
  boundary». On ASCII the same off-by-indent silently misplaced the GFM
  task-box probe, so nested `[ ]` boxes were not restored. Regression tests
  pin both cases; the docs-corpus test is repinned at its 54 pages, with the
  plain legal page and the deliberately unparsable tutorial projection
  listed by name.
- **Boot lane: nested static umbrellas install in XML projects.** The
  once-each pass of the lane compiler (B-006, `desubstitute_covered_units`)
  decided every unit-STATIC substitution against one pre-pass snapshot.
  When a boot-bearing member of a zone itself statically links a child (a
  DPF package requiring a shared base package under a suite umbrella), the
  umbrella stayed substituted and, in a `spec_format = "xml"` project, its
  compiled per-unit `STATIC.xml` was read as a document: `vibe install`
  failed with «unexpected text content» while a Markdown project accepted
  the same graph. The pass now runs to a fixpoint, with a regression test
  for the umbrella → member → base shape.
- **`vibe search` works against the default registry index.** The index
  of a GitHub registry is a static mirror (`raw.githubusercontent.com/
  <org>/index/<ref>`): files, no routes. The client found it through its
  handshake and then asked the live-server-only route `/v1/packages`,
  reported the mirror's 404 as «registry unreachable», and every default
  project searched nothing. The client now reads the mirror's
  `primary.jsonl` and scores it locally with the server's own tokeniser
  and ranking, which moved into `vibe_wire::behaviour::index_search` as
  the one home for both readers; a base that publishes neither the route
  nor the catalog keeps the 404. PROP-005 `##INT-SEARCH` and
  `##TEXT-INDEX` record the two shapes.

## [1.0.0] — 2026-08-20

The first numbered release: the closed-alpha cut the release campaign
(`campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md`) built in one
autonomous marathon. **Semver honesty:** 1.0.0 will break — the
pre-publication regime (PROP-044 D13, `public = false`) stays on, and
`docs/ALPHA-NOTES.md` carries the operator recipe for breaking updates.
Highlights of the release arc, on top of everything chronicled below:

- **2026-09-17 application republish — observable long operations.** Every
  finite CLI family now selects an explicit progress policy and reports live
  work on stderr before network, subprocess, inventory, hashing, compilation,
  or filesystem waits. Package and self maintenance expose named component
  stages and truthful terminal outcomes; lifecycle, registry/cache, workspace,
  documentation, deploy, clean, scrape, and inspection paths share the same
  invocation-owned observer without changing stdout or machine documents.
  `--verbose` adds bounded diagnostics, while `--no-progress` and
  `VIBE_NO_PROGRESS` disable activity without suppressing ordinary results.
- **The machine-global package store** (`~/.vibe/cache/`, PROP-010):
  write-once entries validated against the lockfile's `content_hash`, the
  `vibe cache path|list|add|clean|check --repair` operator family, and the
  store as a real resolution source — a version deleted from every registry
  still installs from the store, and `--offline` (flag / `VIBE_OFFLINE` /
  `[net].offline`) resolves from local sources only with loud misses.
- **Wire truth wave** (PROP-044): integers wider than 32 bits ride as
  canonical decimal strings (rule §4.2b + breaks/003); the journal's
  `removed.version` is strictly required-nullable (breaks/004) with
  absence a loud refusal across all nine generated required-nullable
  fields; the catalog write is idempotent by content; seventeen
  machine-JSON envelopes (7 `vibe-index` CLI + 10 HTTP `/v1/**`) are
  minted formats with schemas, registry records, corpora and round-trip
  oracles.
- **Config truth**: `[[registry]].index_url` exists with the
  env → key → default ladder; `vibe-index` gains the four-rung
  configuration ladder with a visible source per value
  (`vibe-index config`); PROP-005 stops asserting the unbuilt (the
  reindex HTTP trigger is withdrawn, trust-point wording narrowed).
- **Hygiene**: machine JSON prints one POSIX path form on every surface;
  the package-kind vocabulary and its thirty-one hand copies sit under one
  parity test; the discipline engines forget the word `vibedeps`
  (policy `skip_dirs`).
- **The 1.0.0 world**: all 42 canonical packages minted additive v1.0.0
  slots (old slots frozen in place), every cross-package edge re-pinned
  into the 1.0.0 world, the host workspace (20 crates) and the project
  manifest at 1.0.0, vibedeps rematerialised, the boot lane regenerated.


### M1.19 — Qualified package naming (PROP-008)

Reverse-FQDN `group` qualification — the Maven `groupId` shape — replaces the flat `<kind>:<name>` namespace, so two unrelated authors can each ship a `wal` without colliding. Design lock: [PROP-008](vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml). The whole milestone — the identity core (Phases 1–4 + 7), index-backed short-name resolution, and collision detection — landed under MFBT.

- **`group` and the identity tuple.** `[package]` gains a mandatory `group` field — dot-separated `[a-z0-9_-]` segments, reverse-FQDN by convention (not enforced, exactly as Maven does not enforce `groupId` shape). Package identity becomes `(group, name, version, content_hash)`; a `name` is unique within its `group`, not within its `kind`. `kind` leaves identity entirely — it stays a mandatory `[package]` field, but it is pure metadata: it places content, drives the `--kind` filter, and is a UX signal in a kind-prefixed pkgref, while identifying and naming nothing.
- **pkgref grammar `[kind:][group/]name[@version]`.** The qualified `org.vibevm.world/wal` form is what manifests and the lockfile store. `flow:org.vibevm.world/wal` adds a kind prefix, validated against the manifest after resolution. The short `wal` / `flow:wal` form stays as CLI-only sugar.
- **Group-native registry.** `NamingConvention` gains `fqdn` — repo name `<group>.<name>` (`org.vibevm_wal`) — and `fqdn` becomes the default. Registry resolution keys on `(group, name)`; `LocalRegistry`'s layout, `fixtures/registry/`, and every embedded test manifest moved to the `org.vibevm/<name>` shape. The legacy `kind-name` / `name` / `kind/name` conventions remain for registries that have not adopted `group`.
- **Group-native package index.** The `vibe-index` entry schema gains `group` (mandatory) and `workspace_origin` (optional); the `by-name/` layer is re-keyed to the candidate-set file `by-name/<name>.json` — every group sharing a bare name in one file, so a short-name lookup is one GET per registry. The `vibe-registry` index client and the `vibe-publish` post-publish hook follow.
- **Index-backed short-name resolution.** `vibe install wal` — a bare, unqualified pkgref — is resolved to the qualified `org.vibevm.world/wal` at the CLI input boundary, once, before the depsolver runs; manifests and the lockfile only ever store the qualified form. The lockfile is consulted first (a locked package of that name wins outright), then registry candidates are enumerated — a local-directory registry by directory scan, a multi-registry resolver by one `by-name/<name>.json` GET per registry.
- **Collision detection — exit code 7.** A short name matching packages in two different groups is a collision the resolver refuses to guess past: `vibe install` fails with the new exit code `7` ("ambiguous package", distinct from `3`, a dependency conflict) and prints the numbered qualified alternatives so the operator can pick one. `VIBEVM-SPEC.md` §9.4 records the code.
- **Lockfile schema v5.** Each `[[package]]` gains a `group` field; `CURRENT_SCHEMA_VERSION` is `5` (v4 was PROP-007's). vibevm is pre-release — an older lockfile is rejected, not migrated.
- **Spec + docs.** `VIBEVM-SPEC.md` §7.1 (identity, the pkgref grammar) / §7.3–§7.5 / §8 are rewritten for qualified naming under the standing owner sanction; `docs/` (glossary, lockfile-format, the install / version-syntax / git-source references) and this changelog follow.

### PROP-005 — Package index (`vibe-index`) (2026-05-22)

A registry-org metadata index, so `vibe` can `list` / `search` / `outdated` / shortlist versions against cached, mirror-able metadata instead of a live `git ls-remote` per package. Design lock: [PROP-005](vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml). The index layer is strictly optional — a registry without one keeps working through the live path unchanged. This block records work that landed across earlier sessions but was never written up here, plus the 2026-05-22 reconciliation that brought it green.

- **`vibe-index` — server + CLI** (`crates/vibe-index/`, PROP-005 slices 1–8). One binary, two modes: a CLI that builds an index data directory from a registry's package repos (`reindex --from-clones` / `--from-github`, plus `init` / `add` / `remove` / `get` / `list` / `search` / `capabilities` / `purls` / `outdated` / `verify` / `dump`), and an axum HTTP server (`serve`) exposing the same surface — read routes open, write routes bearer-token-authed, with a per-token / per-IP rate limiter. The on-disk format is RPM-shaped: a `repomd.json` manifest over `primary.jsonl`, `by-name/`, `by-cap/`, `by-purl/`.
- **Publisher hook** (slice 9). `vibe registry publish` optionally POSTs the new entry to the registry's index after a successful push; failure is non-fatal — the next `reindex` covers the gap.
- **Consumer fast path** (slice 10). `vibe-registry` gains an `IndexClient`: before falling back to per-repo `git ls-remote`, the resolver consults `<registry>/repomd.json` and reads `by-name/<kind>/<name>.json` for the version shortlist. Index entries stay advisory — `content_hash` is still verified against the fetched bytes, so a stale or hostile index can mislead version selection but never substitute content.
- **`vibe search`** (M2.10). Index-backed package search across configured registries, `--purl` lookup for "what describes this upstream library?", a `--full-scan` indexless fallback, results cached under `~/.vibe/search-cache/`.
- **2026-05-22 de-rot.** `vibe-index` was then a standalone Cargo workspace, outside the routine `cargo test --workspace` gate, and it had silently rotted against the M1.17 unified manifest and the M1.18 loading model: the duplicated `vibe.toml` parser still expected `[writes]` / `[dependencies]` / `[boot_snippet].filename` and rejected every current manifest, and the `content_hash` parity test had drifted off a renamed fixture. The scanner was realigned with the current schema and made lenient against future additions; the golden fixture and parity hash were re-synced (cross-checked against the canonical `vibe-registry::compute_content_hash`); a current-schema scanner regression test was added; the dead slice-1 scaffolding was retired. The suite is green again — 169 tests, clippy `-D warnings` and rustfmt clean.

- **2026-05-22 fold.** The de-rot exposed the root cause (PROP-005 §9 item 11): a standalone workspace hand-duplicating a `vibe.toml` parser, nothing cross-checking it against `vibe-core`, and the workspace itself outside `cargo test --workspace`. The fix moved `vibe-index` from `services/vibe-index/` into `crates/vibe-index/` as a member of the vibevm workspace, deleted the duplicated parser, and switched the scanner to parse through `vibe-core`'s own `Manifest` / `SubskillManifest`. The index schema can no longer drift from the manifest schema, and the routine workspace gate now covers the crate. `tools/self-check.sh` drops its standalone-workspace special-case.

### M1.21 — Incremental install (PROP-011) (2026-05-22)

PROP-009's `vibe install` re-ran the depsolver and re-copied every `vibedeps/` slot on every invocation, whatever had changed. PROP-011 makes it incremental — the `cargo build` / `npm install` shape. Design lock: [PROP-011](vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml).

- **Skip resolution when fresh.** Before the depsolver runs, a freshness check asks whether `vibe.lock` is still a correct resolution of every node's `[requires]` — a cargo-style satisfiability test, no manifest digest stored. When it is, a bare `vibe install` skips the depsolver entirely: no registry walk, no network, just a whole-tree boot regeneration. This also makes `vibe install` **lockfile-respecting** — an unchanged `[requires]` honours the locked versions verbatim, ending the silent version drift the unconditional re-resolve caused.
- **Materialise only the diff.** Materialisation skips a `vibedeps/` slot already present for the resolved version — versions are immutable, so the content is correct; only a new or version-bumped dependency is copied. A `slot_integrity` key in the vibevm user config (`trust-presence` default, or `verify`) governs the skip; `verify` re-materialises every slot.
- **Minimum-churn re-resolution.** When `[requires]` *has* changed, `vibe install` re-resolves but holds the locked version of every dependency the change did not touch, so an untouched dependency never drifts; a held-pin conflict falls back to a full re-resolve.
- **No new flag, no schema bump.** `vibe update` and `vibe reinstall --force` stay the bypasses; `vibe.lock` is unchanged — the lockfile itself is the freshness baseline. `VIBEVM-SPEC.md` §9.1 records the lockfile-respecting contract.

Boot-artifact regeneration deliberately stays whole-tree — the cheap, self-healing phase. Skipping the registry walk for an unchanged subtree (true incremental re-resolution) is deferred to PROP-003's SAT solver. Every phase landed clippy-clean with its test suite green.

### M1.18 — Loading model (PROP-009 + PROP-012) (2026-05-22)

The flat `spec/boot/NN-*.md` boot model is replaced by a computed loading model. Design locks: [PROP-009](vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml), [PROP-012](vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml). Shipped across seven phases.

- **Two trees.** A node's authored `spec/` and its materialised dependencies are physically separate. `vibe install` copies each resolved package's published tree verbatim into a committed `vibedeps/<kind>-<name>/<version>/` slot at the workspace root, and never writes into authored `spec/`. The `[writes]` package section is retired — a materialised package *is* its subtree.
- **Computed boot.** Each node's boot sequence is computed from the unified resolution — inherited foundation + the node's own authored boot + dependency boot + user overrides — and projected into generated `spec/boot/INLINE.md` (the verbatim inline priority lane) and `vibevm/vibespecs/boot/INDEX.md` (a TOML manifest of `static` / `dynamic` entries). Three inclusion types — `inline` / `static` / `dynamic` — are declared per dependency via `link`. The `NN-` filename prefix is retired; `vibe` owns ordering by `category` band.
- **OS-gated boot snippets.** A package's `[boot_snippet]` may declare a `when = "os:<name>"` condition (`windows` / `macos` / `linux`). `vibe` renders the contribution as a `dynamic` `INDEX.md` entry carrying the condition — irrespective of any `link`, since a condition cannot be inlined — and the agent reads the file at boot only on a matching operating system. This closes the dynamic-entry `when` gap PROP-009 §2.3 flagged at Phase 4. The same OS probe is reserved as `if_os` in the subskill `[activation]` vocabulary, so the two mechanisms share one grammar.
- **`vibe reinstall`.** Regenerates the materialised state and boot artifacts without re-resolving; `--force` re-fetches every locked package from source.
- **The managed `<vibevm>` block (PROP-012).** `vibe` no longer overwrites the whole of `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` — it owns only a delimited `<vibevm>` … `</vibevm>` block and preserves every byte outside it, so the developer and other tools can co-tenant the file. A malformed block is a hard error, validated before any mutation; `vibe install` maps it to exit code 3.
- **The boot-directory linter** (`vibe check`) stops enforcing the retired `NN-` filename pattern; it now verifies only that `vibevm/vibespecs/boot/` exists and holds markdown.
- **vibevm self-migration.** vibevm migrated to its own loading model — `vibevm/vibespecs/boot/INDEX.md` generated, a `<vibevm>` block appended to its `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (every hand-authored line, the four rules included, preserved).
- **Docs.** `VIBEVM-SPEC.md` §6 and the rest of the retired-model footprint rewritten for the loading model; the `docs/` sweep covers the new model and `vibe reinstall`.

Every phase landed clippy-clean (`-D warnings`) with its full test suite green.

### M1.17 — Workspace (multi-package projects) (2026-05-21)

The cargo-`[workspace]` / Maven-multi-module shape: a project decomposes into member packages, each published independently — or not at all. Design lock: [PROP-007](vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml). Shipped in five implementation phases plus documentation.

- **Unified manifest.** The two manifest files — `vibe.toml` and `vibe-package.toml` — collapse into **one file, `vibe.toml`**, carried by every node. The role is set by which sections are present: `[project]` (a consumer) and `[package]` (a publishable artifact) are mutually exclusive; `[workspace]` composes with either or neither. One `Manifest` type in `vibe-core` replaces `ProjectManifest` + `PackageManifest`; `Manifest::validate` enforces the role rules.
- **Hard compatibility break — all manifest legacy removed.** vibevm is pre-release; there is no migration path and none is needed. Gone: the `vibe-package.toml` filename, the `[dependencies]` section, the array-form `packages = ["…"]`, the singleton `[registry]` table, and the `vibe.lock` v1/v2/v3 readers. A manifest or lockfile using a removed form is a hard error.
- **Workspace model + discovery.** A new `vibe-workspace` crate. `Workspace::discover` walks up from anywhere inside the tree to the topmost `[workspace]` that transitively encloses the start node — the absolute root, where the single `vibe.lock` lives. `[workspace] members` accepts glob patterns; nesting recurses to arbitrary depth; nesting cycles are rejected. A standalone single-package project is a degenerate workspace, so discovery is the universal entry point.
- **Path-source dependencies.** A `[requires.packages]` entry may be `{ path = "../sibling", version = "^0.1" }` — a third source-kind beside registry-resolved and git-source. Resolution priority is `[[override]]` > path > git-source > registry-walk. `vibe.lock` bumps to schema v4 with `source_kind = "path"`, whose `source_url` is the member's path relative to the workspace root — portable, never absolute.
- **`[workspace.versions]` placeholders.** Named version-constraint placeholders (Maven `<properties>`). A member references one as `{ version.var = "core" }`; it resolves bottom-up through the enclosing-workspace chain, nearest wins.
- **Selective publish.** `[package].publish` (`true` / `false` / `["registry"]`) declares each node's posture. `vibe workspace publish` walks the self-publishing members dependency-first and publishes each as its own repository, reusing the per-package publish machinery; the development monorepo is never modified. Each published copy carries an `[origin]` provenance marker, a "generated copy — contribute upstream" README banner, and a `PULL_REQUEST_TEMPLATE.md` STOP notice.
- `VIBEVM-SPEC.md` §4.2 / §7 document the workspace model.
- **Not yet wired:** `vibe install` / `vibe build` do not yet discover the workspace for unified multi-member resolution — a follow-up milestone that turns on a per-member materialisation decision PROP-007 §2.4 / §3 leaves open. Standalone single-package projects — every project today — are unaffected.

Every phase landed clippy-clean (`-D warnings`) with its full test suite green.

### M1.16 closer — `vibe registry redirect-update` (2026-05-19)

Closes the v0 manual-procedure gap surfaced in the M1.16 ship-complete WAL: editing an existing stub's marker used to require a hand-driven `git clone` / edit / `git commit` / `git push` recipe. The new CLI command automates it.

- **`vibe registry redirect-update <pkgref>`** — partial-update CLI for an existing stub's `vibe-redirect.toml`. Each flag is optional; omitted fields retain their current value. Flags: `--to`, `--ref-policy`, `--pinned-ref`, `--target-auth`, `--target-token-env`, `--description`, `--clear-description`, `--registry`, `--trust-redirect`, `--resync`, `--path`, `--dry-run`. Refuses with `no changes requested` if the computed marker is byte-identical to the existing one — never records an empty commit on the stub's history.
- **Trust model.** Per PROP-002 §2.4.2, changes that alter what content consumers materialise (`target_url`, `ref_policy`, `pinned_ref`) require `--trust-redirect`. Operator-side metadata (`auth`, `token_env`, `description`) does not. Without `--trust-redirect` for a content-affecting change, the command bails with the list of trust-required fields detected and a pointer at the flag. Mirrors the `--trust-mirror` shape on `vibe install` / `vibe update`.
- **Auto-clearing cross-field invariants.** Switching `auth` away from `token-env` clears `token_env` automatically (otherwise the marker would fail to re-parse). Switching `ref_policy` to `pass-through-tag` clears `pinned_ref` automatically. Switching `ref_policy` to `pinned` without `--pinned-ref` (and without an existing pinned_ref to preserve) is a hard error.
- **JSON envelope.** `--json` emits `{ ok, command, registry, pkgref, stub_url, target_url, ref_policy, target_auth, changes: [{field, before, after}], trust_required, dry_run, sync? }`. The `changes` array is the canonical per-field diff (target_url, ref_policy, pinned_ref, auth, token_env, description). `trust_required` is `true` when any change in the diff touches a trust-gated field — CI gating can decide on manual review from this alone.
- **`vibe-publish::git_publish::commit_and_push`** — new helper for committing in-place on an existing clone and fast-forward-pushing `main` to a remote. Symmetric to `push_initial` but for the "existing clone" path: refuses to record an empty commit if `git status --porcelain` is empty after `git add -A`. 2 unit tests against a local bare origin.
- **15 new unit tests** for `compute_updated_redirect_section` + helpers: partial-update detection, clear-description, switch-to-pinned with and without explicit ref, switch-to-pass-through clears pinned_ref, auth flip clears token_env, all rejection paths (empty `--to`, `--pinned-ref` on pass-through, `--target-token-env` without matching auth, switching to pinned with no ref). 4 new hermetic e2e tests for args-level guard rails: `--help` flag coverage, `--description` + `--clear-description` mutual exclusion, invalid pkgref, missing `vibe.toml`.
- Docs: new `docs/commands/registry-redirect-update.md` (full operator reference). `docs/registry-redirect.md` gains a "Rewriting an existing stub's marker" section pointing at the new command; "Out of scope for v0" no longer lists this item. `docs/commands/registry-redirect.md` "Error surface" + pipeline references the new command instead of the manual procedure. `docs/README.md` index gets a new row.

The M1.16 deferred-list is now empty.

### Test fixture re-homing (2026-05-12)

End-to-end and smoke fixtures moved out of the canonical `vibespecs` org (and `olegchir` personal namespace) into dedicated test orgs. The canonical orgs now host only real, installable packages.

- **GitHub `vibespecstest1`** — registry-side test fixtures. Holds `flow-vibevm-github-smoke` (used by `cli_live_e2e::install_github_smoke_alone`) and `feat-helper` (M1.16 redirect stub).
- **GitHub `vibespecstest2`** — external-author / target-side test fixtures. Holds `vibevm-m1-smoke-flow-internal` (M1.15 git-source target), `vibevm-m1-smoke-feat-helper` (M1.16 redirect target), `vibevm-private-probe` (M1.14.4 private-probe smoke target, kept private).
- **GitVerse `vibespecstest3`** — GitVerse-side test fixtures. Holds `vibevm-direct-push-smoke` (used by `cli_live_e2e::install_gitverse_smoke_alone`). Reached over SSH; the live test uses `git@gitverse.ru:vibespecstest3` to bypass GitVerse's HTTPS-requires-auth posture on test repos.
- **`cli_live_e2e.rs`** — `init_project` now overwrites `vibe.toml` with `[[registry]]` blocks pointing at the test orgs. Asserts updated: `registry = "vibespecstest1"` and `"vibespecstest3"` instead of the canonical names. All three live tests still pass.
- **`manual-tests/M1.15-git-source-smoke.md`** / **M1.16-redirect-smoke.md** — recipes rewritten to provision repos via `POST /orgs/vibespecstest2/repos` (instead of `/user/repos` under `olegchir`); install steps write `[[registry]] url = "https://github.com/vibespecstest1"` after `vibe init` so the consumer routes through the test org.

### M1.15 — Git-source dependencies (2026-05-10)

The Cargo / npm / Poetry / Bundler / Go-modules-style affordance — declare a dep as `{ git = "https://...", tag = "v0.1.0" }` instead of resolving it through `[[registry]]`. Spec: [PROP-002 §2.4.1](vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml#git-source).

- **`[requires.packages]` table-form schema** in `vibe-core`: `Vec<PackageRef>` (legacy) and `BTreeMap<PackageRef, GitPackageDep>` (modern) parse transparently; round-trip writes the modern map. Inline-table values declare git-source: `"flow:internal-helper" = { git = "...", tag = "v0.1.0", auth = "token-env", ... }`.
- **`GitPackageRegistry::open_single_package`** — single-package URL constructor that bypasses `org_url + naming`. Reuses M1.14 token-injection / bootstrap-with-scrub plumbing.
- **`MultiRegistryResolver` short-circuits** the registry walk for any pkgref in `[requires.packages]` git-source declarations. Resolution priority: override > git-source > registry-walk.
- **`vibe install` flags** — `--git <url>`, `--tag/--branch/--rev`, `--git-auth`, `--git-token-env` add a git-source declaration without hand-editing `vibe.toml`.
- **Lockfile** — new `source_kind = "registry" | "git" | "override"` discriminant per `[[package]]`. Wire-compatible — `Option<SourceKind>` defaults to `None` for pre-M1.15 lockfiles.
- **Hermetic e2e** in `vibe-cli/tests/cli_e2e.rs` — install with `--tag`, install with `--branch`, repeat install rejection, uninstall removal from both `requires.packages` and `requires.git_packages`.
- **Production smoke walk** documented at `manual-tests/M1.15-git-source-smoke.md`. Validated against `https://github.com/vibespecstest2/vibevm-m1-smoke-flow-internal` — `git archive --remote` → shallow-clone fall-back exercised on the GitHub case. Smoke fixtures live in dedicated test orgs (`vibespecstest1/2/3`) so the canonical `vibespecs` org stays populated with real packages only.
- **Bug fix** along the way: `fetch_manifest_at_ref` (used by git-source path) now falls back to `refresh_package` when the host refuses `upload-archive`, matching `fetch_dep_manifest`. Without this, GitHub-hosted git-source targets failed at resolution time.
- **Bug fix**: `vibe uninstall <pkgref>` now removes the entry from BOTH `requires.packages` and `requires.git_packages` (was a one-list-only walk).
- Docs: new `docs/git-source-dependencies.md` operator reference (in M1.15 spec landing); `docs/commands/install.md` extended with the new flags.

### M1.16 — Registry redirect (delegated package via stub repo) (2026-05-10)

The Linux-distro-style virtual-package mechanism — a registry org's stub repo carries `vibe-redirect.toml` pointing at an external git repo where the package's actual content lives, instead of carrying the content directly. Spec: [PROP-002 §2.4.2](vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml#redirect).

- **`vibe-redirect.toml` schema** in `vibe-core::manifest::redirect`: `[redirect]` block with required `target_url`, optional `ref_policy = pass-through-tag | pinned`, `pinned_ref` (required iff pinned), `auth` / `token_env` (target-side, mirrors PROP-002 §2.2.1), `description`. Mutually exclusive with `vibe-package.toml` at the same ref (`AmbiguousStub`). 11 unit tests.
- **`MultiRegistryResolver::follow_redirect`** — resolver detects the marker after a registry-walk success, opens a synthetic single-package registry on `target_url`, fetches manifest at the pass-through-tag (or `pinned_ref`). Hop limit = 1: target cannot itself be a stub. `MultiResolution.via_redirect` carries the stub URL through the resolve→fetch boundary; `redirect_target_auth` / `redirect_target_token_env` propagate target-side auth.
- **`MultiRegistryResolver::fetch_manifest`** — new redirect-aware DepProvider entry point. Reuses `resolve()` to converge on the same `MultiResolution` the install pipeline already saw, then reads the manifest from whichever URL the resolution recorded (target for redirects, declared URL for git-source, registry's URL otherwise).
- **`fetch_via_redirect`** — clones target into `<cache>/__redirects__/<kind>-<name>/clone/`, distinct from registry / override / git-source cache tiers. Token-discipline preserved.
- **`try_fetch_redirect_for_url`** — two-path read: `git archive` first (cheap, file://-friendly), shallow-clone fall-back when the host refuses `upload-archive` (the GitHub case). Marker-first hop check fires before manifest fetch so chain rejection works against stub-only target repos.
- **`vibe registry redirect <pkgref> --to <url>`** — CLI helper that creates the stub repo automatically through the `RepoCreator` infrastructure. Flags: `--ref-policy`, `--pinned-ref`, `--target-auth`, `--target-token-env`, `--description`, `--sync` (mirror target tags immediately), `--dry-run`. Refuses if the stub already exists (editing is manual for v0).
- **`vibe registry redirect-sync <pkgref>`** — mirrors target tags into the stub. Reads stub's `vibe-redirect.toml`, lists target tags, pushes the missing ones. Refuses for `pinned`-policy stubs (semantically meaningless to sync).
- **Lockfile** — `via_redirect` field per `[[package]]` records the stub URL; `source_url` carries the target URL; `source_kind = "registry"` (redirect-resolved packages came through a registry stub, just delegated).
- **Hermetic e2e** in `vibe-cli/tests/cli_e2e.rs` — pass-through-tag install, pinned-policy install, identity-mismatch reject, hop-limit chain reject. Plus 9 helper unit tests for `parse_target_auth`, `build_redirect_readme`, `derive_target_token_env`, `inject_token_into_url`, `build_target_fetch_url`.
- **Production smoke walk** documented at `manual-tests/M1.16-redirect-smoke.md`. Validated against a `vibespecstest1/feat-helper` stub → `vibespecstest2/vibevm-m1-smoke-feat-helper` target pair on real GitHub: `vibe registry redirect`, `vibe registry redirect-sync`, then `vibe install feat:helper@^0.1` resolving through the stub. Lockfile records `via_redirect = "https://github.com/vibespecstest1/feat-helper.git"` and `source_url = "https://github.com/vibespecstest2/..."`. Smoke fixtures live in dedicated test orgs so the canonical `vibespecs` org stays populated with real packages only.
- Docs: new `docs/commands/registry-redirect.md` and `docs/commands/registry-redirect-sync.md`; `docs/registry-redirect.md` updated with the CLI workflow (manual procedure kept as a fallback section).

### v0.1.0-ready package-management bundle — 2026-05-08

The 2026-05-08 push bundled four milestones in one day. They land here under one block because the surface-consistency closer (M1.14.3) only makes sense in the context of M1.14 having shipped first; together they constitute the v0.1.0-ready package-management story.

### M1.12 — `vibe.toml` `[requires]` + cargo-shape install (2026-05-08)

- `ProjectManifest` gains `[requires].packages` / `.capabilities` re-using the `Requires` type from `vibe-package.toml`.
- `vibe install <pkgref>` now writes the user-supplied pkgref to `vibe.toml` after a successful apply — the cargo / npm pattern. `vibe install` with no arguments installs everything in `[requires]`.
- `vibe uninstall <pkgref>` symmetrically drops from `[requires]`.
- First-run migration: pre-`[requires]` projects get the manifest seeded from `vibe.lock` `meta.root_dependencies` on the next install.

### M1.13 — Cargo-shape version constraints (2026-05-08)

- `VersionSpec::parse` collapses to a single `semver::VersionReq::parse` call: bare semver `0.3.0` is shorthand for `^0.3.0` (caret), matching Cargo / npm / Poetry. Use `=0.3.0` for strict equal.
- `vibe install <pkgref>` (no version) records caret-of-resolved in the manifest. Explicit constraints are preserved verbatim.
- `--exact` flag (npm `--save-exact` shape) overrides both with `=<resolved>`.

### M1.14 — Authenticated registries (2026-05-08)

The big one — turns vibevm from "public registries only" into "production-ready for private repos."

- **Per-registry `auth` axis** (PROP-002 §2.2.1): `none` (default) / `token-env` / `credential-helper` / `ssh`. CLI: `vibe registry add --auth --token-env <NAME>`.
- **Token-env runtime**: `VIBEVM_REGISTRY_TOKEN_<HOST>` (or explicit `token_env`) loaded once at registry-open, injected as `https://x-access-token:<TOKEN>@host/...` in per-package URLs only at git-invocation time. Token never persists on disk — `set_remote_url(.., "origin", plain_url)` scrubs the credential out of `.git/config` immediately after `bootstrap`. `MissingToken` precheck before spawning git.
- **Auth-aware 401 classifier** (PROP-002 §2.3.1): public-registry 401 walks past as "no public answer here"; authenticated-registry 401 halts with actionable error. Closes the GitVerse-returns-401-for-missing-public-repo regression that surfaced via opencode + glm-flash.
- **TTY-aware credential silencing** in `apply_common_env` — non-TTY / `--unattended` runs silence GCM, `credential.helper`, `core.askPass` so a 401 cannot become a blocking GUI window.
- **Stderr classifier** extended for `could not read Username/Password`, `User cancelled dialog`, `HTTP 401/403`, `401 Unauthorized`, `403 Forbidden` (M1.14.1).
- **`--auth-required` flag** for strict CI gating: public-401 halts instead of walking, useful when fallback to a public substitute would be wrong (M1.14.2).
- **Aggregated per-registry error report** — `PackageNotFoundEverywhere { kind, name, summary }` lists each walked registry with URL, auth regime, and outcome. Inline multi-line `Display` flows through the standard error chain (M1.14.2).
- **`toml_edit`-based comment-preserving writes** — operator's hand-edited comments in `vibe.toml` survive every `vibe install` / `uninstall` / `registry add` mutation. Three layers preserved: header, per-table prefix, document trailing (M1.14.2).
- **Surface consistency closing slice** (M1.14.3): MCP `--yes` flag wired to actual TTY confirm prompt (was vestigial); `--assume-yes` alias on every MCP confirm-skip flag for symmetry with package commands; `--exact` extends from `install` to `update` (cargo `cargo update --precise X.Y.Z` shape — re-resolve and tighten manifest in one step); `--auth-required` extends from `install` to `update` + `outdated`.

### Other UX

- **Global `--unattended` flag + `VIBE_UNATTENDED` env-var**: implies `--assume-yes` / `--yes`, blocks wizards from opening, stamps `unattended: true` on JSON envelopes. Replaces the awkward `--invoked-by user-provisioning` workaround.
- **`docs/registry-auth.md`**: new operator reference covering all four auth regimes, env-var conventions, walk-vs-halt matrix, troubleshooting.
- **`docs/version-syntax.md`**: new operator reference for semver constraints (caret / tilde / equal / range), the two-file model (manifest = declaration, lockfile = materialisation), Cargo / npm / Poetry / Bundler comparison.
- **`vibe mcp install --scope both` works without `vibe.toml`**: provisioning scripts on a fresh user account succeed (project-leg silently skipped, user-leg writes as normal).

Phase A of M1.1-revision shipped earlier on `main` between 2026-04-23 and 2026-04-25; M1.7 (vibe-mcp server) shipped 2026-05-05; M1.10 (`vibe outdated`) shipped 2026-05-04. The next major milestone is M1.5 (LLM-based generation) — non-routine, needs separate sign-off.

---

## M1.1-revision Phase A — 2026-04-24 / 2026-04-25

The decentralized per-package registry refactor. Replaced the M1.1 monorepo-shaped registry with the model spelled out in [`PROP-002`](vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml): one git repo per package under an organization URL, identity = `(kind, name, version, content_hash)`, `[[registry]]` array + `[[mirror]]` + `[[override]]` schema, transitive dependency resolution, maintainer-side publish command, JTD-driven wire contracts.

### Documentation slice (2026-04-24)

- Added `spec/common/PROP-000` §15–§19: dependency-weight pragmatism, JTD + codegen pattern, production-architecture-in-prototype lens, complexity-≥-RPM expectation, load-bearing setup-doc obligation.
- Added `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` "Memory discipline" section: project facts live in the repo, never in tool-specific user-memory.
- Added repo-root `DEV-GUIDE.md` and `RUNTIME-GUIDE.md` scaffolds.
- Amended `VIBEVM-SPEC.md` §7.3 / §7.4 / §7.5 / §8.1 / §8.2 / §8.3 / §8.4 / §8.6 / §11.2 / §16 for the per-package registry, capability-based deps, lockfile schema v2, and the new `vibe registry publish` command.
- Marked `PROP-001` §2.3 / §2.4 / §2.6 superseded by `PROP-002`; pruned the size-based argument in §2.1 per PROP-000 §15.
- Added `vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml`: full design lock for the new registry model.
- Added `ROADMAP.md` M1.1-revision active section + M1.6 multi-registry-polish queued section.
- Added repo-root `TASKS.md` as the live work-slice checklist.
- Refreshed `vibevm/vibespecs/WAL.xml` for the new phase.

### Schemas and codegen foundation (2026-04-25)

- `chore(git)` — `.gitattributes` pins LF line endings everywhere; `content_hash` is now OS-stable.
- `build(tools)` — JTD codegen scaffolding: `xtask` crate carries `cargo xtask codegen` / `check-codegen`, `tools/jtd-codegen/` README pins version 0.4.1 with per-platform install commands, `crates/vibe-wire/` placeholder ready to receive generated types, `.cargo/config.toml` aliases the runner.
- `feat(schemas)` — seven JTD schemas under `schemas/` document every CLI `--json` wire format: `init_report`, `install_plan`, `install_report`, `list_report`, `registry_sync_report`, `registry_publish_report`, `uninstall_report`. Schema-first authoring; struct migration follows when `jtd-codegen` is installed.

### Core types (2026-04-24)

- `feat(core)` — capability-based package dependencies. New `CapabilityRef` type (`<namespace>:<name>[@<version>]`). `PackageManifest` gains `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]`. Legacy `[dependencies]` compact form auto-migrates via `normalize_legacy_deps` on read; on next write the manifest renders in modern shape.
- `feat(core)` — `vibe.toml` schema v2: `[[registry]]` array (with `name` / `url` / `ref` / `naming`), `[[mirror]]` (with `of` / `url` / `priority`, where `of = "*"` matches any registry), `[[override]]` (surgical pkgref pin with `pkgref` / `source_url` / `ref` / `reason`). Legacy singleton `[registry]` form auto-migrates on read with `name = "default"` and `naming = "kind-name"`. `NamingConvention` enum (`KindName`, `Name`, `KindSlashName`) is a per-registry property.
- `feat(core)` — `vibe.lock` schema v2: `[meta]` gains `schema_version`, `solver`, `root_dependencies`; per-`[[package]]` gains `registry`, `source_url` (renamed from `source` with serde alias), `source_ref`, `resolved_commit`, `dependencies`, `overridden`. v1 files auto-migrate on next write. `Lockfile::looks_like_v1_on_disk()` heuristic for future UX nudges.

### Registry layer (2026-04-25)

- `feat(registry)` — shallow `ShellGit` primitives: `list_tags` (via `git ls-remote --tags`, deduped peeled-form), `fetch_file_at_ref` (via `git archive`, in-process tar extraction). Resolver walks N candidate versions of a package with N `git archive` round-trips, not N clones.
- `feat(registry)` — `GitPackageRegistry` for the per-package model: addresses each package as `<org>/<naming(kind, name)>.git`, versions are git tags, lazy clones (`bootstrap` / `update` only when committing to a version, not during dep walk). `fetch_dep_manifest` reads `vibe-package.toml` via `git archive` without cloning.
- `feat(registry)` — `MultiRegistryResolver` orchestrates priority + override + mirror dispatch. `[[override]]` short-circuits with manifest-identity verification (refuses if the manifest at the pinned ref names a different `(kind, name)`). `mirrors_for(name)` exposes priority-sorted mirror chain (runtime mirror dispatch + cross-source `content_hash` verification deferred to M1.6 Phase B).
- `feat(registry)` — `MultiRegistryResolver::refresh_lockfile_clones` walks lockfile and refreshes per-package clones; registry-served and override-served entries refresh through their respective subtrees; legacy / local entries reported as skipped.
- `refactor(registry)` — `CachedPackage` carries lockfile-v2 provenance (`registry_name` / `source_ref` / `resolved_commit` / `overridden`). All registry impls populate per their semantics. Dropped intermediate `MultiCached` wrapper.

### Resolver (2026-04-25)

- `feat(vibe-resolver)` — new crate. `DepProvider` / `DepSolver` traits. `NaiveDepSolver` (DFS, no backtracking) handles concrete deps + capabilities + obsoletes + conflicts + simple disjunctions. `MultiRegistryProvider` adapts `MultiRegistryResolver`; `LocalRegistryProvider` adapts `LocalRegistry`. `ResolvedNode.dependencies` post-processed to exact-pinned `=<version>` for the lockfile. Resolvo / libsolv impls reserved behind the same trait.

### Install pipeline (2026-04-25)

- `feat(install)` — switched `vibe install` to `MultiRegistryResolver`. `git+` prefix stripped at the backend boundary. `cli_e2e::install_from_git_registry` rewritten for the per-package fixture layout.
- `feat(install)` — transitive install via `NaiveDepSolver`. `vibe install` runs the solver before fetching; transitive packages materialise after roots; lockfile entries' `dependencies` populated with exact-pinned pkgrefs; `[meta].root_dependencies` carries the user-typed roots; CLI step output marks transitives as `(transitive)`.
- `feat(install)` — content_hash integrity check on plan. Lockfile-pinned `content_hash` must match a fresh fetch's hash for the same `(kind, name, version)`; mismatch surfaces as `InstallError::ContentDrift` with the pinned vs actual hashes named. PROP-002 §2.1 invariant enforced at plan time.
- `feat(registry)` — per-package `vibe registry sync` walks the lockfile and refreshes every per-package clone (`MultiRegistryResolver::refresh_lockfile_clones`).

### Publish tooling (2026-04-25)

- `feat(vibe-publish)` — new crate. `RepoCreator` trait + `GitVerseCreator` (Gitea-compatible HTTP via reqwest blocking + rustls). `Publisher` orchestrator (manifest read → repo create/reuse → init+push+tag). `Token` with debug/display redaction (renders as `***`). `vibe registry publish <path> [--registry <name>] [--dry-run]` subcommand. Error surface per PROP-002 §2.10 (auth-forbidden / org-not-found / push-denied / tag-collision / host-unreachable). Live API verification deferred to first real publish run.

### Fixtures, manuals, and end-user docs (2026-04-25)

- `chore(fixtures)` — relocated `packages/` → `fixtures/registry/` via `git mv` (history preserved). `packages/` is now reserved for the future dogfooding tree (vibevm using vibevm).
- `test(manual)` — `manual-tests/M1.5-gate-v2-per-package-smoke.md` walkthrough for the per-package model end-to-end against `vibespecs/`. Companion to the existing legacy-monorepo smoke.
- `docs(commands)` — reference pages under `docs/commands/` for every shipped CLI subcommand: `init`, `install`, `list`, `uninstall`, `registry sync`, `registry publish`, `version`. Each page has usage, flag table, examples, exit codes, schema links, related references.
- `docs(authoring)` — per-kind authoring guides under `docs/`: `authoring-flow.md`, `authoring-feat.md`, `authoring-stack.md`. Manifest examples, capability-name conventions, versioning rules, publish procedure.
- `docs` — repo-root `README.md`: hero, status, quick start, doc map, the four kinds, workspace layout, build/test, contributing, license. The landing page for anyone hitting the GitVerse repo URL.
- `docs(architecture)` — `docs/architecture.md`: contributor-facing tour of the workspace. Mental model (package / registry / pipeline), per-crate purposes with dep direction, key traits with future-impl slots, ASCII pipeline diagrams for install / publish / sync, wire formats, cache layout, file-tree quick reference, reading order for a new contributor.
- `docs` — `docs/lockfile-format.md`: exhaustive reference for `vibe.lock` v2. Field-by-field semantics, identity model, v1 → v2 migration, jq snippets for tooling, worked example.

### Test count

vibe-core: 38 → 63 tests. vibe-registry: 19 → 55. vibe-install: 6 → 11. vibe-cli: 11 + 6 e2e (unchanged). vibe-resolver: new — 14. vibe-publish: new — 10. vibe-wire / xtask / vibe-graph / vibe-llm / vibe-check: 0 each (placeholders or built-in-Rust modules with no Rust tests yet). Workspace total at the close of Phase A: **170+ tests**, clippy clean with `-D warnings`.

---

## M1.5-gate content slice — 2026-04-22 / 2026-04-23

Content for the M1.5-gate target: three demo flows live on the (then-monorepo) `anarchic/vibespecs` registry, end-to-end installable as a multi-package smoke.

- `feat(packages)` — published `flow:wal@0.1.0`, `flow:sync-from-code@0.1.0`, `flow:atomic-commits@0.1.0` to `git@gitverse.ru:anarchic/vibespecs.git`. Each ships a boot snippet at a distinct numeric prefix (`10-` / `20-` / `30-`).
- `test(manual)` — `manual-tests/M1.5-gate-multi-package-smoke.md`: three-package end-to-end smoke against the live monorepo registry. Distinct prefixes coexist; one shared clone under `~/.vibe/registries/<hash>/`; symmetric uninstall preserves user-owned files byte-identical.
- `feat(cli)` — `vibe init` writes the default `[registry]` (legacy singleton form) pointing at the public registry on first scaffold. Override with `--registry-url` / `--registry-ref`; opt out with `--no-registry`.
- `docs(wal,roadmap)` — checkpointed M1.5-gate content complete.

---

## M1.1 — 2026-04-22

Git-backed registry. Decisions pinned in [`PROP-001`](vibevm/vibespecs/modules/vibe-registry/PROP-001-git-backend.xml).

- `feat(registry)` — `GitBackend` trait with `ShellGit` impl: shells out to system `git`, no `libgit2` runtime dep. Windows-specific spawn flags (`CREATE_NO_WINDOW`, `LC_ALL=C`, `GIT_TERMINAL_PROMPT=0`) so the child never flashes a console window or hangs CI. Stable stderr classification for `RepoNotFound`, `AuthFailed`, `NetworkUnreachable`, `RefNotFound`.
- `feat(registry)` — `Registry` trait at the crate root with `LocalRegistry` (M0 path, kept) and `GitRegistry` (new) implementations. `git+<transport>://` source URIs in the lockfile.
- `feat(registry)` — `~/.vibe/registries/<hash>/` cache layout with first-use clone + 1-hour freshness TTL. `VIBE_REGISTRY_CACHE` env-var override.
- `feat(cli)` — `vibe install` reads the `[registry]` section from `vibe.toml`. Added `vibe registry sync` to force-refresh the registry cache.
- `refactor(core)` — lifted UTC timestamp helper into `vibe-core`.
- `test(manual)` — `manual-tests/M1.1-git-registry-smoke.md` against the real GitVerse registry.

---

## M0 — Walking skeleton — 2026-04-16 / 2026-04-17

The M0 milestone — proves the file-management mechanics work end-to-end.

- `chore` — repo scaffold, `.gitignore`, `LICENSE.md` (proprietary EULA placeholder).
- `docs` — recorded the four non-negotiable project rules (attribution, Conventional Commits, group by meaning, autonomy on routine changes only) in `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`. Bootstrapped the `spec/` tree per `VIBEVM-SPEC.md` §14.1. Pinned the registry URL.
- `build` — Cargo workspace with seven crates: `vibe-cli`, `vibe-core`, `vibe-graph`, `vibe-registry`, `vibe-install`, `vibe-llm`, `vibe-check`.
- `feat(core)` — manifest schemas (`PackageManifest`, `ProjectManifest`, `Lockfile`), `PackageRef` / `PackageKind` / `VersionSpec`, `ValueTag` typed-value tags exchanged between graph nodes.
- `feat(registry)` — `LocalRegistry`: directory layout `<root>/<kind>/<name>/v<ver>/`, content-addressed cache at `<project>/.vibe/cache/<kind>/<name>/<ver>/`, `sha256:<hex>` content hashes computed deterministically across OSes.
- `feat(install)` — plan / apply / register / unregister loop. Boot-snippet conflict detection (filename + numeric `NN-` prefix). User-owned-paths guard. Exit codes per `VIBEVM-SPEC.md` §9.4.
- `feat(cli)` — `vibe init` / `install` / `list` / `uninstall` with plan → confirm → apply discipline. Output as human / `--json` / `--quiet`.
- `feat(packages)` — hand-wrote `flow:wal@0.1.0` as the canonical registry payload (the test-fixture template every smoke uses).
- `docs(wal)` — recorded the verified GitVerse push command and ready-to-publish state.
- `docs(spec)` — reconciled `VIBEVM-SPEC.md` with the shipped M0, pinned the mirror layout convention.
- 64 tests green at the M0 tag.

---

## Format notes

This file is curated, not auto-generated. Each milestone block is a hand-written rollup of conventional-commit subjects since the previous milestone, organised by area. Conventional Commits per [`PROP-000 §12.2`](vibevm/vibespecs/common/PROP-000.xml#conventional-commits) make the rollup mechanical; the value-add of this file over `git log --oneline` is the milestone framing and cross-references to PROPs / SPEC sections that explain *why* a change happened.

Future format tightening: once we have a tagged release, `[Unreleased]` becomes a normal milestone block dated when the tag was cut, and a new `[Unreleased]` opens at the top.
