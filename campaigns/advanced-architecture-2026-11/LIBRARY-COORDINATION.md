# Coordination with VibeVM Library

VibeVM Library (the Steam-like desktop application) is developed by a separate
agent in its own closed repository. It drives vibe only through vibe's public
client contract: the CLI with `--json`, `--progress-format json`,
`--unattended`, `--invoked-by`, `--stop-file`, and the MCP server. This page is
the vibe side of the coordination. It holds no Library internals: this
repository is open.

## How the two sides coordinate

1. **vibe to Library.** The "Offered to clients" table below lists every
   client-facing feature on vibe main: the exact flags and fields, where it is
   specified, and from which version or commit. The central vibevm session
   adds a row when an ADV-CONTROL, ADV-SDD or ADV-NATIVE entry that changes the
   client contract is accepted. Once `vibe capabilities --json` ships, the
   Library learns at run time what a given vibe supports. The table then serves
   as the human-readable changelog of the contract.
2. **Library to vibe.** The Library's asks reach the central vibevm session as
   a message, either relayed by the owner or sent by the Library session to
   `vibevm-architect` when both run. The vibevm session records each ask in
   this repository (BACKLOG or the plan) and answers with the decision. Neither
   side edits the other's repository.
3. **Live relay.** When both sessions run on one machine, the vibevm session
   sends the Library session one short message per accepted batch: what
   landed, the commit, the row of the table. Otherwise the owner relays it.
4. **Joint items.** Some capabilities need work on both sides. The vibe side
   is an entry in the vibevm plan. The Library side is only named in the
   "Joint items" table, and the Library's own plan owns it.
5. **Contract rules.** Changes are additive: no exit code changes, no removed
   fields, a new format epoch for an incompatible shape. Fixtures that prove the
   contract are generated inside vibe, never copied from the Library.

## Offered to clients (vibe main, after 1.0.11; next release 1.0.12)

These already exist on main and may be unknown to the Library, which was
written against recordings of 1.0.11.

| Feature | How a client uses it | Where |
|---|---|---|
| Project install preview | `vibe install --plan-only --json` in a project: the closure plan without effects | `crates/vibe-cli/src/cli/pkg.rs` (`plan_only`) |
| Sources by policy | `--prefer-hosting <github\|gitverse\|host:HOST>`, `--exclude-hosting …`, `--no-hosting-preference` on install and update; locked sources stay authoritative | `crates/vibe-cli/src/cli/source_policy.rs`, commit `863fbbf37` |
| The source an application was installed from | recorded and shown by `vibe app list` / `vibe app describe --json` | closes vibevm BACKLOG B-254 |
| Turn an MCP entry or a skill on and off | `vibe mcp enable\|disable …`, `vibe skill enable\|disable …` without reinstalling the package; report `agent_activation_report` | `crates/vibe-cli/src/cli/mcp.rs`, `skill.rs`, `schemas/agent_activation_report.jtd.json` |
| Stop by file (all platforms, no console needed) | global `--stop-file <path>`: creating the file stops the operation at the next safe point. Accepted today by project `install` and `update`, `registry publish`, `dl`, `tool`, `user`, and `mcp` (except `serve`). Not yet by `install -g` and the `app` verbs: see ADV-CONTROL. | `crates/vibe-cli/src/composition/cancellation.rs` |
| Bytes written to disk in transfer events | `disk_written_bytes` in transfer progress records | PROP-068 `#disk-writes` |
| Import a vibe without touching shell profiles | `vibe self import <file> --tag <v> --use --no-profile` | `crates/vibe-cli/src/cli/vvm.rs` |

## Coming from ADV-CONTROL (planned)

| Feature | What the Library gains |
|---|---|
| `vibe capabilities --json` | Enable features by capability instead of hard-coded version numbers; works for branch builds |
| Stable codes and remedies in `--json` errors | Distinguish its own wrong flag (`usage`) from a refusal; act on `remedy.args` (for example ask for consent and re-run) |
| `--stop-file` on `install -g` and all `app` verbs | Cancel application installs on Windows |
| Operation records (`vibe ops list\|show <id>`, `--operation-id`) | After a restart, show the real outcome instead of "not known"; a retry with the same id never runs twice; "busy" names the holder |
| Every prompt answerable up front | `--unattended` never blocks |
| Uninstall of a configured package | Uninstall works for every package |
| MCP progress and cancel with the same events as the CLI stream | One event vocabulary for every client |

## Joint items

| Item | vibe side (vibevm plan) | Library side (Library plan) |
|---|---|---|
| Cancel on Windows | `--stop-file` for application verbs | pass a stop file and create it on Cancel |
| Outcome after restart | operation records | keep the operation id, read `vibe ops show` on start |
| Feature gating | `vibe capabilities --json` | read it once per located vibe, fall back to version gates for an older vibe |
| First real package configuration (owner decision 11) | an app in the catalog with a configuration choice (for example GPU variant) | show the choice in the install dialog and the app page |
| Other package managers as native (ADV-NATIVE) | platform adapters, global npm and pip | screens for system and global packages |
