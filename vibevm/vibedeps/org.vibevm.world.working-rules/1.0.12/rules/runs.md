# Run ownership and cleanup

Genre: project operational session contract. This companion to `protocol.md`
is the authoritative home for managed execution-resource retention and cleanup.
It is provider-neutral; launcher names, machine paths and authentication exceptions
belong in the untracked local binding. Commands, active run records and recovery
receipts belong in the responsible lead's private runbook, not this document.
Status: contract defined; implementation and destructive-use verification pending.
No package, daemon or product command is introduced by this contract.

## Authority and scope

The implementation lead owns managed runs, independent acceptance and cleanup
coordination, without acquiring ownership of the peer checkout. Reading this
contract, an empty holder set, a successful worker exit or an archive does not
authorize deletion. Apply requires applicable explicit owner authority for the
exact scope. A later owner restriction overrides an earlier cleanup authorization.
Unknown legacy directories are report-only, never implicitly adopted or deleted.
Build outputs and package caches (`target/`, compiler and dependency caches) are
not managed resources: this contract never protects them, and freeing them is
ordinary work under `protocol.md`.

Owner refinement, 2026-11-15: delete retained task resources only after the task has
actually been accepted and continuation of the same subagent dialogue is no longer
needed. Therefore deletion eligibility is the conjunction of:

1. Independent acceptance is verified for the exact candidate/input revision.
2. The responsible lead has explicitly closed the relevant dialogue for resume,
   including any dependent continuation or child session.
3. No named holder or retained dependency needs the resource.
4. Required preservation and evidence obligations are satisfied.
5. Resource identity, allowed deletion scope and owner authorization are verified.

False or unknown on any condition protects the resource. Acceptance is not a
worker claim, exit code or process completion. Dialogue closure is not inferred
from inactivity, a timeout, a failed process or age. An archive cannot substitute
for acceptance or dialogue closure. Failed/unaccepted candidates remain protected.
Immediate cleanup means eligible without a mandatory waiting period, not automatic
cleanup upon task completion. There is no default TTL or age-based deletion.

## Resources, holders and dependencies

Use bounded resource bundles, not a universal per-file collector: source snapshot,
session/transcript with resume dependencies, temporary execution artifacts, and
compact outcome/test/recovery evidence. Closing a transient socket or process
handle is normal teardown; it is not authorization to erase retained task state.

Each managed resource has a stable id, type, admitted root-relative locator,
identity/generation, lifecycle state, named holders and disposition/evidence.
Holders name their purpose and stable owner: worker, review, resume or explicit pin.
Store named acquisitions, not only an integer reference count. A release identifies
its exact acquisition and generation, is idempotent, and cannot release a later
acquisition with the same owner name.

Register protection before exposing a resource to a consumer. Register a durable
review holder before starting a worker so a crash cannot leave its output unowned.
Acquire successor review/resume protection before releasing predecessor ownership,
or perform the handoff in one atomic metadata transaction. Completion releases only
the completed worker's holder. Independent acceptance does not close a dialogue.
A review rejection does not grant permission to discard the candidate.

Dependencies protect required ancestors, attachments, tool-result files and source
locations transitively. Copied transcripts are not automatically independent.
Initially keep resumable bundles pinned; support shared dependency cleanup only
when its closure can be verified. Inseparable state may be one bundle. Unknown
cycles or dependencies protect resources rather than triggering a guessed release.

## Preservation is not acceptance

Source recovery and dialogue recovery are different proofs. A source recovery set
includes the exact retained base plus all meaningful changed/new/deleted content,
binary data, file types and modes, and relevant untracked/ignored task files.
Document intentional input exclusions. Preserve symlink metadata without following
it, or refuse unsupported types. Validate paths and restore into an independent
location; compare the resulting inventory, not merely the archive's own checksum.
Do not archive an entire HOME, credentials or unrelated private state.

An accepted source snapshot needs no duplicate archive when all meaningful work
is represented by independently verified, durably retained Git content, required
evidence is retained separately, and no resume dependency requires the original
location. A hash without retained objects is not availability. Dirty integration
or a promise to commit later is not preservation. Never delete the only retained
base or receipt on which another recovery proof depends.

A session may require its original absolute location and adjacent state. A single
resume smoke test proves only its tested path, not universal portability. If the
required state cannot be enumerated or relocation is unproven, preserve the original
location. Verified recovery still does not permit deletion before the acceptance
and dialogue-closure gates above.

## Coordination and deletion protocol

All managed allocation, start, resume, pin, finalize and cleanup operations share
one coordination boundary. PID alone and a stale textual registry are not liveness
proof. Use process start/boot identity and supervised lifetime evidence; unknown
children, stale leases or incomplete crash records remain protected. Unregistered
readers cannot be made safe by a reference count: restrict initial destructive
scope to controlled private resources and cooperating consumers.

A report lists exact resources, sizes, holders, dependencies, protection reasons
and potential eligibility. Apply revalidates the report against current state;
a prior report is never deletion authority by itself.

Under the common lock, verify exact resource generation, all eligibility gates and
current references. Durably transition the selected resource to `retiring` with a
unique operation id before deletion. New acquisitions, resumes and pins on that
resource are refused thereafter. Release the lock during physical deletion only
if every consumer honors that durable fence; otherwise retain coordination.

Delete only through anchored no-follow operations verifying the selected entry's
identity. A path-prefix check or `realpath` followed by recursive removal is
insufficient. Refuse symlink escapes, mount boundaries, nested repositories or
worktrees, foreign trees and unsupported entries. Never move unmanaged directories
into a managed namespace just to bypass admission.

On success retain a compact `deleted` receipt with resource identity, exact plan
and verification references. A crash in `retiring` leaves a protected reconciliation
case, not permission for a broad sweep. Missing metadata or failed receipt
publication cannot be interpreted as successful cleanup. A quarantine rename is
optional, reclaims no space and does not prove safety or stop existing writers.

## Initial implementation and verification boundary

Initial scope is one private managed source bundle, named holders, read-only report,
explicit finalize and exact-plan manual apply. Resume state and compact evidence
remain retained. No background cron, global directory scan, general cycle collector,
provider-specific integration or automated TTL is needed. Legacy failed snapshots
stay report-only until their own acceptance and dialogue closure are established.

Before destructive use, independently review the implementation and test synthetic
roots only. Cover every false/unknown acceptance and closure combination, duplicate
and late release, acquire/finalize races, crash boundaries, stale process identity,
failed metadata writes, substituted paths/symlinks/mounts, missing recovery inputs,
binary/new/deleted/mode changes and shared resume dependencies. Worker success does
not satisfy this gate. Production cleanup requires a separately verified exact plan.

Reserve space for active growth and preservation/restore work on each relevant
filesystem. Existing machine free-space floors remain binding. If no resource is
eligible, pause new allocations and report; do not remove holders, kill active work
or delete candidates to satisfy a quota.

## Run observability — heartbeat and milestones

Scope: local supervisor evidence for explicitly authorized runs. This section adds
no product command, shared service, daemon, launcher permission, retry policy or
cleanup authority. The implementation lead owns integration and verification;
architectural review precedes declaring this observability milestone complete.
Do not change an active run's packet or instrumentation contract mid-attempt.

### Evidence classes and event identity

Keep these observations distinct:

- A supervisor heartbeat proves only that the observer reached its sampling point
  and records the child state it can establish. It is not worker progress, useful
  work, model responsiveness or proof that an inactive-looking process is hung.
- A worker milestone is a validated worker-authored progress claim with the tool
  completion evidence described below. It is not independent verification that a
  test passed, an artifact is correct, a task is accepted or a dialogue is closed.
- A process outcome is established by launch/wait/supervision evidence, not by a
  worker's final text, a milestone called done, pipe EOF or a silent interval.

Every stored event has a fixed protocol/type and allowlisted payload, stable run
and attempt identifiers, a strictly increasing supervisor-assigned sequence number,
an emitter identity, and supervisor observation time. Use monotonic time for elapsed
intervals/deadlines; wall time is diagnostic, not ordering authority. Worker-provided
times or ids cannot replace these fields. Do not infer a total ordering of actual
worker actions across independently read stdout and stderr; sequence orders observer
records. Heartbeats are emitted at a configured bounded interval (initially 30
seconds), including during quiet worker output, without replaying missed intervals
as fictional historical samples. Start a fresh attempt identity on a new process;
continuation does not reuse a prior sequence space without explicit recovery rules.

### Incremental supervision and milestone admission

Drain stdout and stderr concurrently in bounded chunks for the entire supervised
lifetime. Handle fragmented UTF-8, split/coalesced records, malformed JSON and
oversized/unterminated input without an unbounded buffer or abandoning either pipe.
Do not wait for a newline on one stream while the other can fill. Keep parsing,
stream draining, heartbeat scheduling and child wait independently observable.

A milestone is admitted only from a successful, matching tool-result for an
allowlisted Write operation, or the exact shell-write channel below, to one
dedicated, packet-declared progress file. Its
exact path is a writable private progress mount, not a product/source file; machine
paths belong in local configuration. Validate the captured Write payload against a
small closed schema with bounded strings and enumerated milestone kinds. Correlate
by tool-call id within the same run/attempt, handle duplicates idempotently, and
bound pending correlations. An unmatched, failed, malformed or expired tool call
is not a milestone. A successful result confirms completion of that Write, not the
truth of the worker's claim. Free-text stdout, a tool name alone, worker final text,
or a file's mere existence/mtime cannot substitute for correlation.

Capture the exact validated payload associated with the completed Write. Do not
reread the latest overwritten file and attribute its bytes to an earlier tool id.
Parallel/out-of-order calls must not misassociate payloads. No arbitrary file reads,
path traversal, symlink following or executable progress directives are permitted
by milestone collection. Changing the file or adding a milestone never authorizes
commands, broader writes, retries, acceptance or deletion. Unknown schema fields
and event kinds are rejected or recorded as bounded parser diagnostics, not run.

#### Optional exact shell-write channel

A packet may explicitly admit one alternative when the worker exposes Bash rather
than a structured Write tool. The only accepted command has the literal form
`printf '%s\n' '<JSON>' > <dedicated-progress-path>`: one literal printf invocation,
one single-quoted format, one single-quoted JSON argument and one literal absolute
path fixed by the packet. Require the entire command string to match a canonical
encoding, not merely tokens that look equivalent after shell parsing. No leading
or trailing command, environment assignment, executable path substitution, shell
options, append redirection, comments, pipelines, chaining, extra redirections,
command substitution or alternative quoting is admitted. The bounded JSON argument
contains no literal apostrophe, newline or NUL; use JSON Unicode escapes when
needed. It passes the same closed milestone schema as the Write channel.

A shlex-style tokenizer alone is not a shell grammar or quoting proof: tokenization
can erase whether substitution was active. Validate the exact original command
against the canonical encoding after extracting and validating the bounded JSON;
reject any mismatch. Correlate the captured payload with the successful result of
that exact Bash tool-call id in the same attempt. Reject background launches and
asynchronous submission receipts; require completed foreground execution with no
error indication and, when an exit status is supplied, status zero. Unknown result
semantics refuse admission. Deduplicate and bound pending calls as for Write.
Record which channel supplied the worker claim; the emitter remains the worker,
not the supervisor or shell. Never execute, evaluate, source or replay the parsed
command in the observer, nor reread the progress file to reconstruct the payload.

This channel provides equivalent correlation evidence, not proof of filesystem
contents, task correctness or acceptance. The worker's writable progress directory
and dedicated file must be admitted independently as private scratch with no
pre-existing symlink/hardlink and no writable host targets; shell redirection does
not supply no-follow safety itself. If that boundary is unavailable, refuse this
channel rather than broaden shell syntax. The packet must explicitly name the
channel before an attempt; do not retrofit an active run or count earlier prose as
a milestone. Synthetic tests cover exact valid writes, quoting/substitution and
operator variants, wrong paths, invalid/oversized JSON, failed/unmatched/duplicate
results and asynchronous receipts. Live success is not required to authorize
implementation and does not authorize further provider calls.

### Private bounded logs and diagnostic failures

Store the event stream in a private, bounded per-attempt JSON-lines log, with
restrictive directory/file permissions and no-follow creation/opening. Validate
log destinations against the managed scope; refuse links or substituted entries
rather than appending to an arbitrary path. Bound event size, pending parser/tool
state, total retained log bytes and diagnostic volume. Ordinary provider/runtime
and tool errors remain useful local evidence under the credential boundary; never
intentionally log auth headers, keys, environments or unrelated host data. Review
logs before wider sharing. This is not a raw transcript or an instruction to discard
ordinary errors merely because they originated in the approved execution service.

Rotation, overflow or suppressed events must be explicit: retain a gap/rotation
record identifying the affected sequence interval or, when that cannot be known,
the loss as unknown. A bounded log is not represented as complete history. Preserve
sequence continuity across rotations within one attempt. If disk-full, permission,
encoding, parsing or observer failure prevents durable event recording, expose that
failure through an independent bounded status/error channel where available; never
claim a log receipt was written when it was not. Do not recursively flood the same
failed sink trying to log its own failure. Keep draining child pipes safely even
when parsing/logging fails, or apply an explicitly authorized supervision policy;
an observer failure is not itself a child failure or permission to kill/retry it.

### Terminal evidence and verification

Record launch failure, actual process exit code or signal, and supervisor-requested
cancellation/timeout separately. A requested termination is not a verified exit;
retain the actual wait result or explicitly unknown outcome. Distinguish provider
budget/transport errors observed in the stream from process termination. If both
occur, retain both without guessing causal attribution. Observer failure and child
outcome are separate dimensions. Account for descendants retaining pipe handles
with bounded post-exit draining; open pipes do not overrule an observed child exit,
and child exit does not prove all descendants stopped. No terminal state grants
acceptance, dialogue closure, preservation completion or cleanup eligibility.

Before integration, independently exercise synthetic children only: fragmented and
coalesced records, malformed/oversized streams, stderr saturation, quiet long runs,
duplicate/unmatched/failed/out-of-order tool results, forged milestone paths and
payloads, rotation/loss, disk-full/failed writes, launch failure, exit zero/nonzero,
signal, cancellation, timeout and surviving pipe holders. Verify bounded memory,
continued dual-pipe draining, heartbeat timing, exact milestone correlation,
terminal coverage and observer/child failure separation. Live provider verification,
if needed, requires its own currently applicable lane authorization and isolation;
synthetic success does not establish provider availability. Record actual checks,
failed/unrun cases and the exact reviewed implementation identity.

## Reporting and evolution

Distinguish execution completion, independent acceptance, dialogue closure,
verified preservation, eligibility and deletion in reports. Record actual failures
and unrun checks. The local runbook links this contract and retains operational
evidence; it does not redefine the gates. Improve the process through the existing
working-rules procedure, not a second policy registry.
