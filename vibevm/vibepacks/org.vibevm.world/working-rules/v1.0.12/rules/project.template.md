# Working arrangement policy — REPLACE_PROJECT_NAME

Genre: project session instructions — template. Copy this file to
`WORKING-RULES.project.md` at the project root, replace every placeholder with what
the owner approved, delete the sections the project does not use, and commit it.
Read it with `protocol.md`; it is the project-owned configuration, not part of the
portable protocol. A filled copy authorizes nothing in another project merely by
being copied there. Keep it free of secrets and machine-specific paths.

## Roles and integration

- Integration owner: REPLACE_ROLE
- Integration branch: REPLACE_BRANCH
- Final acceptance and approval-required decisions: the human owner.
- Implementation execution and delegated workers: REPLACE_ROLE

Roles and lead profiles are defined in `protocol.md` "Roles and profiles". The
local binding records the role and profile of each session and lane.

### Worker context configuration

REPLACE_WITH_WHEN_A_PACKET_USES_BOUNDED_CONTEXT_OR_FULL_PROJECT_BOOT_OR_DELETE.
Without a rule here, use bounded context.

## Standing grants

- REPLACE_WITH_DATED_OWNER_GRANT_AND_ITS_EXACT_SCOPE_OR_NONE

Grants preserve, rather than expand, existing mandates. Loading this document does
not invoke any granted operation.

## Routine task branches

REPLACE_WITH_THE_OWNER_RULE_FOR_TEMPORARY_AND_TASK_BRANCHES_OR_DELETE.

## External publication

- Publication procedure: REPLACE_PROCEDURE_POINTER
- Targets: REPLACE_TARGETS
- Standing authorization: REPLACE_DATED_GRANT_OR_NONE

External push, package publication, release, deployment and shared service changes
need explicit owner authorization for that step unless a grant above covers it.

## Managed execution resources

REPLACE_WITH_WHETHER_THE_RUN_RESOURCES_CONTRACT_APPLIES_OR_DELETE. The contract
is `runs.md` beside `protocol.md`.

## Verification and durable work

- Focused checks per change: REPLACE_HOW_AFFECTED_TESTS_ARE_CHOSEN
- Full gate: REPLACE_ONE_COMMAND
- Language of normative documents: REPLACE_LANGUAGE

### Architectural quality of delegated implementation

REPLACE_WITH_THE_REVIEW_A_LEAD_OWES_TO_EXECUTOR_RESULTS_OR_DELETE.

### Cross-cutting architectural review at substantial milestones

REPLACE_WITH_WHICH_MILESTONES_GET_A_CROSS_CUTTING_REVIEW_OR_DELETE.
