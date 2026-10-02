# Host-owned fixed Validate reattachment

`CliGrantService` is a library seam for an embedding that has already
established a physical project root and trusted target, principal, target OS
and policy source. It does not discover an OS user or mint authority from a
frame, CLI argument, descriptor, ambient CWD or HOME. Its stable inner C1 scope
is separate from each local session's assigned outer context ID.

First issuance holds the incumbent lifecycle lease, writes and reads back a
bounded `Issuing` pointer, initializes the incumbent shared `PolicyOwner`, then
issues exactly one C1 namespace and publishes its ID and expiry. Reattach is
lookup only: it checks physical root correlation, exact scope, source, OS,
namespace and current shared policy before constructing
`FixedLifecycleDecisionHost::from_shared_owner`. The pointer is not a second
operation journal and contains no approval, result or provider output. An
uncertain, missing, corrupt, foreign or incomplete pointer never silently
causes another issue. Expired and retired namespaces keep reserved-key
inspection while C1 forbids a fresh effect.

The exact Linux checks used `cargo +nightly test --offline --locked`:
`vibe-lifecycle --lib state::submission` passed 20/20 and
`vibe-orchestrator --lib machine_submission::cli_grant` passed 22/22 in the
worker's private target and again in the primary target. Four focused doctests
passed in the worker. The native grant test runs two separate OS processes
against one key and verifies one C1 namespace, one domain run and one counted
provider effect. It also checks current-policy revocation, stale process-local
questions, retired-key recovery and damaged/missing authority records. No full
gate panel or mutation campaign was run.

The pointer deliberately refuses weak or unsupported durable directory
checkpoints. Pending issuance has no automatic repair path; the owner must
reconcile an interrupted issue. The later CLI composition must authenticate
its host inputs and choose first issue versus reattach before it can advertise
fixed Validate. This seam alone does not make CLI fixed actions callable.
