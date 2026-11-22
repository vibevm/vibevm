# Local working arrangement — inactive template

Genre: local session configuration. Copy this file to `WORKING-RULES.local.md` at
the project root only for an owner-selected arrangement, and exclude the copy from
Git as `protocol.md` "Adoption and configuration" describes. Do not overwrite an
existing binding. This template grants no authority and is not active merely
because it is installed or read. Keep the filled copy to the current state only.

A lead with profile `open` also reads `protocol.md` and `WORKING-RULES.project.md`.
A bounded lead reads only "Current mode" and "Tiers" here. The project policy, not
this file, records owner-approved integration, publication, verification and
standing synchronization permissions; never infer grants from this template.

## Current mode

- Mode: REPLACE_WITH_ordinary_OR_split_OR_single-central-session
- Completion list: REPLACE_WITH_PATH_OF_THE_LIST_FILE_OR_EXTENSION_PAGE
- Full gate: REPLACE_WITH_ONE_COMMAND
- Executors for a bounded lead: REPLACE_WITH_HOW_ONE_FRESH_EXECUTOR_IS_STARTED_PER_PACKET
- Task worktrees: REPLACE_WITH_DIRECTORY_FOR_PACKET_WORKTREES
- Publication: REPLACE_WITH_none_OR_THE_PROJECT_POLICY_GRANT_THAT_APPLIES
- Owner instructions in force: REPLACE_WITH_DATED_VERBATIM_QUOTES_OR_NONE

## Checkouts

- Architect checkout: REPLACE_ARCHITECT_MACHINE_AND_ABSOLUTE_PATH
- Architect session address: REPLACE_ARCHITECT_SESSION_ADDRESS
- Builder checkout: REPLACE_BUILDER_MACHINE_AND_ABSOLUTE_PATH
- Builder session address: REPLACE_BUILDER_SESSION_ADDRESS
- Topology: REPLACE_WITH_separate-clones_OR_linked-worktrees
- Integration policy: REPLACE_PROJECT_POLICY_POINTER
- Task branches: actual branch and full base hash belong in each task brief.

Verify canonical roots and actual Git common directories; identical branch names
are not role or topology evidence. For multiple machines name each machine and
authorized transport. Keep the completed arrangement identical across
participating checkouts; each session edits its own. A mismatched or incomplete
binding stops affected writes, not ordinary-mode fallback.

## Tiers

- Architect: lead, profile REPLACE_WITH_open_OR_bounded
- Builder: REPLACE_WITH_lead_open_OR_lead_bounded_OR_executor
- Worker lanes: executor
- Profile by model (owner's record of observed behaviour): REPLACE_OR_NONE
- Any other model: `bounded` until the owner says otherwise.

## Execution and credentials

- Approved external worker launcher: REPLACE_APPROVED_LAUNCHER_OR_NONE
- Real-harness test launcher: REPLACE_APPROVED_TEST_LAUNCHER_OR_NONE
- Isolation evidence location: REPLACE_EVIDENCE_POINTER_OR_NOT_YET_VERIFIED
- Narrow credential exception: REPLACE_NAMED_CREDENTIAL_AND_SERVICE_OR_NONE
- Private input exclusions: REPLACE_PATHS_OR_NONE_WITHOUT_SECRET_VALUES

`none` authorizes no external launch. An approved launcher name alone is not proof
of isolation. Before any external worker or supervisor, satisfy the protocol's
credential-isolation gate with synthetic probes. Keep secret values and auth stores
out of this file and worker input. Do not switch launchers, expose host agents or
weaken security to bypass an unavailable lane. Packets select bounded context or a
full project boot that the project policy authorizes; without such a policy use
bounded context. Full boot never imports central personal state or widens task or
write permissions.

## Machine

- Build-output locations: REPLACE_PER_CHECKOUT_OUTPUTS
- Shared resource constraints: REPLACE_PORT_CACHE_SERVICE_TOOLCHAIN_CONSTRAINTS
- Disk and placement rules: REPLACE_OR_NONE

Do not stop services or change shared configuration without authorization.
Record machine-specific incidents here, not in the portable protocol.

## Startup

Verify roots, common directories, branch and dirty state; report role, assigned
task and write boundaries. Resolve personal execution state for this exact
checkout if the project uses such a mechanism. Do not inherit the peer's mandate
or acceptance. Reading this file does not create or switch branches, repair Git,
synchronize, commit or publish. Missing settings require clarification, not
invented permissions.
