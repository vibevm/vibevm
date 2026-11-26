# Working arrangements

Genre: portable session protocol, shipped by the package
`org.vibevm.world/working-rules`. Every page this file names without a path sits
beside it in the package's `rules/` directory. This document describes
collaboration, not an owner's authorization to act. Installing or reading it
grants no permission to commit, integrate, publish, deploy, access credentials or
destroy work. Existing project contracts, safety rules and harness permission
boundaries remain binding.

## Which file is yours

Find your line in the local binding (`WORKING-RULES.local.md`, "Tiers") and read
only what this table gives you. If you are a bounded lead or an executor, stop
reading this file now.

| You are | Read | Do not read |
|---|---|---|
| lead, profile `open` | this file, the project policy, the local binding | — |
| lead, profile `bounded` | `lead-bounded.md`, the "Current mode" block of the local binding, your completion list | this file, the project policy, plans and campaign history |
| executor | `worker.md` and your packet | everything else |

No line for you in the local binding: a central session is a lead with profile
`bounded`; a worker is an executor.

## Configuration layers and startup

Read the project's root instructions, this protocol, `WORKING-RULES.project.md`
when present, and `WORKING-RULES.local.md` when present, in that order. Both of
those files sit at the project root and belong to the project; the package never
writes, replaces or activates them. The existing project boot still applies to
central sessions. Workers follow an explicitly
selected context mode under project policy; a bounded-context worker uses its
packet instead of loading the central session's boot.

- **Protocol — this file:** reusable roles, procedures and safety invariants.
- **Project policy — `WORKING-RULES.project.md`:** owner-approved integration and
  publication policy, verification requirements and explicitly recorded grants.
  Keep it versioned and free of secrets and machine-specific paths.
  `project.template.md` is its skeleton.
- **Bounded lead page — `lead-bounded.md`:** the whole working rules of a lead
  with the `bounded` profile; see "Roles and profiles".
- **Executor page — `worker.md`** and **packet template — `packet.template.md`:**
  the whole instruction surface of an executor; see "Roles and profiles".
- **Run resources contract — `runs.md`:** optional reading for a project that
  keeps managed execution resources; its project policy says when it applies.
- **Local binding — `WORKING-RULES.local.md`:** selected mode, checkout roots,
  session addresses, topology, execution launcher and machine restrictions. Keep
  it untracked and locally excluded. It may narrow project permissions, not widen
  them. `local.template.md` is an inactive template, not a grant.

Missing project policy grants nothing: use existing explicit project authority or
ask for an owner-only decision when needed. Missing local configuration means
ordinary development, not an error; do not create it merely to boot. In ordinary
mode one session may design, implement and verify under the project's rules.

In split mode resolve roles from explicit bindings, never model names or directory
basenames. Before writes verify canonical roots, actual Git common directories,
branches and working-tree state. Incomplete or mismatched bindings stop affected
writes rather than silently reverting to ordinary mode. A copied history never
transfers another session's mandate, personal plan, custody or acceptance. Resolve
personal execution state by the project's mechanism, if it has one.

## Reading the owner's words

The owner writes short, plain requests. Read them at their plain meaning and act
on the outcome asked for. These rules govern how instructions are read. They
replace narrow or defensive readings; they lift no explicit restriction stated
elsewhere, such as no push, a read-only peer tree or the credential boundary.

- **Outcome, not ceremony.** "Finish X" asks for X working and committed. It does
  not ask for receipts, manifests, dispositions or proof that nothing else could
  be wrong. Add only the checks a careful engineer would run for that change.
- **A permission covers what it obviously covers.** When a permission or request
  can be read narrowly or broadly, take the reading that lets the requested work
  finish, say in one line which reading you took, and continue. Listing what a
  permission "does not authorize" is not required and is not a substitute for
  doing the work. The exceptions are the fixed red lines below; they are never
  widened by interpretation.
- **A waiver removes the check.** When the owner waives or skips a check ("skip
  the manual checks", "accept without the Windows test"), that check and every
  step, runner, simulation or test whose main purpose was to perform it are gone,
  manual or automated. The product behaviour itself stays required and is covered
  by the ordinary tests. Set the item to `dropped` with the reason `owner waiver,
  <date>` and the owner's words; a parent then closes when nothing else holds it.
  Do not replace a waived check with a substitute, and do not keep the item open
  as "unrun, not waived". Do not stretch a waiver beyond its words either: say
  what it covers when you record it.
- **An aside is not a mandate.** A remark, idea or question made while other work
  is in flight is answered or noted in the backlog as one line. It joins the
  current goal only when the owner says it is part of that goal or reorders
  priorities explicitly. When a new request conflicts with the current one, ask
  once which comes first, in one sentence, and keep working on the current one
  until answered. An owner stop, pause or correction is never an aside: it takes
  effect at once.
- **The latest direct instruction wins.** It replaces earlier instructions on the
  same subject in full; do not merge it with their restrictions.
- **A simple operational request is done at face value.** "Clean the disk" means
  free space: delete `target/` directories and package caches in trees this
  session owns and no live process is using, and report what was freed. Anything
  holding uncommitted Git content is not a cache. After two runs that free
  nothing, remove the timer and tell the owner once.
- **Owner words travel verbatim.** A session relaying the owner quotes the exact
  words with their date and adds nothing. Only the owner pauses, stops, waives or
  reprioritizes another session's work; a peer may ask, not order, and may not
  invoke "the owner" for a requirement the owner did not state. A relayed
  restriction without a quote is a peer's opinion. A peer's concrete safety
  report (a leaked key, a destroyed file) is checked before continuing, and a
  relayed quote never authorizes a red-line action.
- **Ask only when it matters.** Ask before an irreversible or outward-facing step
  whose authority is unclear. Do not ask, and do not stop, for routine choices.

Fixed red lines, unaffected by the reading rules above: rewriting published
history, force-push, publication or deployment without a grant, exposing or
copying credentials, deleting or discarding work that cannot be regenerated
(including another session's uncommitted changes), writing in a peer's tree,
weakening a failing test to make it pass, and large binary imports. Outside
these and the explicit restrictions of the project policy and local binding,
work is ordinary.

## Finishing and stopping

A campaign must be able to end. These rules bind every session and every plan.
On the question of when work closes and when a session stops, they prevail over
any conflicting sentence in the project policy, `runs.md` and the text of any
planning method the project uses. A bounded lead follows its own page instead of
this section; that page is this section in procedural form.

- **Frozen completion list.** Before execution, write the completion list for the
  current goal: a short closed list of outcomes, each checkable by a command or a
  file. Work is done when every entry is `accepted`, `dropped`, `deferred` or
  `blocked` on a named outside action. A finding
  made during execution is fixed inside its entry when small, otherwise it becomes
  one backlog line. An entry is added only on the owner's word; the session
  makes the edit and shows it.
- **Three closing states.** `accepted` (its check passes), `dropped` (owner
  waiver or withdrawal) and `deferred` (one backlog line with a reason) all close
  an entry and let its parent close. An entry whose check fails is never
  `accepted`. The session defers an entry itself, without asking, only after a
  real attempt that overran its budget or met a named outside blocker; deferred
  entries are named first in the final report. A parent is never held open by
  wording such as "accepted narrowly", "candidate" or "not claimed"; state the
  limit once in the final report instead.
- **One session may hold every role.** When the owner has one session do the
  work, its single diff review satisfies every builder and architect review the
  project policy names. Subagents of that session are not external workers.
- **Blocked means stop once.** When no entry can move without an outside action,
  say what is needed in two sentences and end the turn. Do not invent substitute
  work, and do not repeat the status when re-prompted by a hook, a timer or a peer.
- **No open-ended goal hook.** A stop hook or standing goal must name the frozen
  completion list as its condition. "Finish the campaign" with no list is not a
  valid hook condition; ask the owner to restate it against the list.
- **Evidence is small.** Evidence for an entry is the command, its exit code and
  the commit. Create no hash manifests, receipt files, retained historical
  binaries or new evidence stores, and do not re-verify blobs. Leave existing
  stores alone. Logs stay outside Git.
- **One review, finite findings.** A change is reviewed once by reading its diff.
  A finding must name a concrete defect and a finite fix; it is fixed and the fix
  is checked, with at most one further round. A finding that breaks its entry is
  fixed inside that entry. A review does not open a new brief.
- **One brief per entry.** No brief, qualification suite or acceptance commit per
  finding, and no suites whose only purpose is to pin that an unbuilt feature is
  absent. A test that a refused input is refused is an ordinary test.
- **Gates.** Focused affected tests per change; the full gate must pass on the
  final commit, and is run once at the start when the base is unverified.
- **The plan is a map, not a ledger.** A plan node holds its title, state,
  dependencies and acceptance in a few lines, plus at most two short evidence
  pointers. History belongs to Git. Edit the plan when an entry changes state, not
  per atom. A plan that no longer fits in one read (about 50 KB) is not read
  whole: copy it to a dated sibling file once, then write a new plan from the
  completion list, with closed subtrees as one line each. Nothing open may be
  lost in that rewrite; closed detail may.
- **Commits carry the work.** Code and its tests in one commit with a short body
  that explains why. No commits whose only content is acceptance, a recorded hash
  or a reconciliation of earlier records.
- **Plain language.** Reports and plan text are written in full ordinary words
  with spaces. A status states what works and what does not; it does not list
  what is "not claimed".

## Keeping these files current

Rule files hold the current state only. When an instruction changes, replace the
old text; do not stack "supersedes" paragraphs or keep superseded history, which
Git already preserves. A local binding longer than about one hundred lines, or
one that answers the same question in more than one place, is rewritten before
it is next extended; until then it still binds. A dated verbatim owner
instruction recorded in a local binding is the owner's instruction, not local
configuration, and is read as such. Prohibitions are stated once, in the red-line list above or in
the credential section, not repeated after every permission.

## Product-oriented planning and review

Plan and review from complete product use, finished end-to-end user journeys and
a coherent user experience, not recursive perfection of isolated plan nodes.
Before expanding a subtask or asking for another repair or proof, name the
user-visible outcome or the concrete risk it protects and the finite evidence that
decides it; a rare case is handled in a bounded way when evidence, a contract or
a real safety, privacy, compatibility or data-loss risk makes it matter. This never
waives required behaviour, weakens a test or hides a defect.
Apply it to architectural decisions, builder packets and review requests alike;
role splitting must not turn one usable outcome into an endless prerequisite ladder.

## Split-mode responsibilities

- **Architect:** architecture, specifications, public behavior and compatibility,
  implementation briefs and architectural review; visual/web design, including
  explicitly assigned design components/styles. Name design-owned files separately
  from builder-owned wiring and use the existing design system where present.
- **Legal research belongs to the architect:** establish jurisdiction and facts,
  verify current primary sources, cite them, distinguish requirements from
  interpretation, and flag uncertainty and matters requiring qualified counsel.
  Research is not a binding legal opinion or authority to make commitments.
- **Builder is the coding team's lead:** implementation, tests, fixtures, tooling,
  runtime documentation, worker assignment/coordination, independent review and
  implementation integration. Choose internal details where contracts allow it;
  return missing contract/design/legal decisions as precise questions. Do not
  change requirements or acceptance criteria merely to make implementation pass.
- **Human owner:** priorities, approval-required decisions, role disputes, legal
  commitments and final acceptance. Architectural review is not owner acceptance.

Architectural ownership is not worker dispatch authority. Delegation policy and
approved execution lanes come from project policy and local bindings; this
protocol mandates neither a provider nor a model. Ownership follows purpose, not
file extension. Agree overlapping files and an integration owner before parallel
work. Each session writes only in its assigned checkout/task tree. The peer tree
is read-only, including formatting, builds, installs, Git operations and delegated
writes. An architect may test in their own tree; production implementation outside
design-owned scope needs an explicit assignment.

## Bounded workers and independent verification

The builder owns packets, dependencies, isolation, scheduling, review, tests and
integration. Parallel workers require disjoint explicitly assigned writes and
available review/machine capacity. Supervisors must not duplicate implementation.

Every packet identifies accepted inputs, task root, allowed reads/writes,
exclusions, checks, reporting requirements and the selected context mode:

- **Bounded context:** exact sanitized rule excerpts and task-relevant sources;
  no central root instructions, boot index or compiled boot lane. Disable and
  verify automatic instruction loading; a prompt alone is not proof.
- **Full project boot:** when authorized by project policy, provide the admitted
  project root instructions, complete static lane and applicable indexed files in
  the declared boot order, plus task-relevant contracts. Verify delivery from the
  exact accepted snapshot and completion of required reads, not just file presence.
  Do not attach a quiet clause that forbids the boot this mode requires.
  A worker that needs full project context is started through the project's
  launcher; nobody assembles a system prompt by hand. When the project
  generates subagent types, such a worker is started with the type
  `vibe-worker`, and a packet executor with `vibe-executor`.

The project sets mode-selection criteria. Without such authorization use bounded
context. Full boot expands context, not permissions: every worker remains explicitly
non-central, retains a bounded task/write scope, and neither claims custody nor
imports the owner's personal plan, auth stores or unrelated home state. Boot
instructions requesting those central-only operations do not apply to workers.
Before delivery, check the boot closure against allowed data. If a required file
is private or cannot be admitted safely, report the conflict; never expose it or
silently omit it while claiming full boot. Shared published project sources may
be supplied in either mode without granting arbitrary host access.

Bind additional task trees explicitly rather than inferring their role. Workers
do not change Git metadata, commit, publish or write into peer checkouts.

Worker success is evidence, never acceptance. The builder reads the actual diff,
runs independent affected checks, and returns defects for rework. Reports identify
actual checks, failures, unrun checks and outstanding decisions. Preserve project
comprehensive gates; neither role splitting nor delegation lowers the quality bar.

### Credential isolation

Before any external worker or supervisor launch, including read-only research,
verify a restricted filesystem/process/network boundary, isolated HOME/config and
allowlisted environment. Sanitizing environment variables alone is insufficient.
Do not expose host credentials, agent sockets, process environments, inherited
file descriptors, credential helpers, ordinary harness auth stores, private input
or unrelated home directories. Use synthetic sentinels to test isolation, never
read real secret values for a probe. Restrict network access to approved channels.

By default the execution service's authentication is inaccessible to worker tools
and model context. The owner may explicitly authorize the launched agent to access
its own execution-service credential; record that narrow exception in the local
binding without the value. Such an exception covers only the named credential and
intended service, never other host secrets or general auth stores. Do not reproduce
credentials in task packets, chat, diagnostic logs, reports or Git. Do not copy
normal auth stores or embed keys in task argv. If the remaining boundary cannot be
enforced, report the blocker and do not launch. Do not weaken shared security or
substitute a different launcher to evade the requirement. Launcher-specific choices,
credential exceptions and prohibited paths belong in the local binding, not here.

## Roles and profiles

There are two roles. A **lead** owns an outcome: it decides, dispatches, reviews
and reports to the owner. An **executor** carries out one packet and decides
nothing. A lead also has a **profile**, which says how much it is trusted to
judge. Role and profile are set by the owner in the local binding, per session
and per lane. They describe how a model has actually behaved on this kind of
work, so the binding may name the model, but nobody infers a profile from a model
name on their own: an unlisted model starts as a bounded lead or an executor and
is promoted by the owner after real completed work.

- **Lead, profile `open`.** Reads this protocol and weighs situations. Every rule
  here that asks for judgement ("when small", "proportionate", "the reading that
  lets the work finish") is addressed to it. It may write and freeze a completion
  list, change specifications, and decide what is in scope.
- **Lead, profile `bounded`.** Leads inside a closed world. It works only from a
  completion list that the owner has approved. It edits that list itself, but
  only as the owner directs, never on its own initiative, and it follows
  `lead-bounded.md` instead of this protocol. This profile
  is for a capable model that finishes well inside fixed bounds but expands the
  work when bounds are left to its judgement.
- **Executor.** Follows one packet. It reads only `worker.md` and its packet,
  written from `packet.template.md`. It never loads the
  boot lane, the plan, the campaign documents or this file.

### Why the bounded profile exists

Some strong models turn a rich specification into an obligation to complete it.
Given a tree of specs, anchors and plan nodes and the freedom to edit them, they
close every link, add the missing units, verify what was already verified and
split each finding into new tasks, and the work never ends. Asking such a model to
"use judgement about scope" does not help: scope is exactly where its judgement
fails. What does help is observed and simple:

- a closed list, with the sentence "anything not on the list is out of scope";
- "done" defined by one command per entry, written before the work starts;
- no right to add tasks, plan nodes or spec units, only to note a suggestion;
- hard limits on attempts, so that a stuck entry is set aside and not refined;
- a short instruction page. Long rule files and accumulated guard rails are read
  literally and make this failure worse, so a bounded lead is given one page and
  told not to look for more.

### What an open lead owes a bounded lead

- **A completion list before any work, approved by the owner.** The list is one
  file, named in the "Current mode" block of the local binding, in the entry
  format of the bounded lead page: outcome, write area, one check command per
  entry. When an installed extension of these rules keeps the list inside the
  project's own plan, the extension says where, and no second list is kept. An
  entry that needs a design decision is not ready; the open lead or the owner
  makes the decision first and writes the result into the entry.
- **Entries sized for closure**: each should fit a handful of executor packets.
  Split large outcomes when writing the list, because a bounded lead may not.
- **Spec changes named in the entry.** If an outcome needs a spec edit, the entry
  says which document and what it must state afterwards.
- **A review of the result, not of the path.** When the bounded lead reports the
  list closed, the open lead or the owner checks outcomes against the list and
  reads the backlog lines it left. Deferred and blocked entries return to
  whoever wrote the list.

The owner is not expected to edit any file by hand. The owner directs in chat and
the session makes the edit: an open lead writes and changes lists and plans on
the owner's word, and a bounded lead drafts its own list from the outcomes the
owner named and changes it exactly as told. In every case a list becomes binding
when the owner says yes to what was shown, and "only the owner changes the list"
means the owner's decision, not the owner's keyboard.

### When the architect is a bounded lead

The split arrangement gives the architect architecture, specifications and
briefs. Under the bounded profile that authority narrows:

- It designs and writes specifications only where a list entry names them. It
  does not start new specifications, restructure existing ones or extend the
  ontology on its own initiative.
- A design question that no entry answers becomes a proposal of at most one page
  to the owner, with one recommended option. Work continues on other entries
  while the owner decides.
- Its briefs are packets. It writes no review documents, dispositions or
  qualification plans; a review is a decision to accept a diff or return one
  defect.
- Architectural review of another session's milestone is one pass with a short
  verdict: accept, or a numbered list of concrete defects.

### What the lead owns when executors do the work

- **Every decision is made before dispatch.** The lead reads the code, chooses
  the design, and writes packets with exact files, steps, one check command and
  the commit message. A task that cannot be written without the word "decide"
  stays with the lead.
- **The loop is run by the lead or by a script, not by an executor.** The queue
  of packets, the choice of the next packet, the status of each packet and the
  decision that the work is complete never pass through an executor. An executor
  session starts with one packet and ends with `DONE` or a failure note.
- **Done is an exit code.** After an executor reports `DONE`, the lead or the
  script runs the packet's check command itself. The executor's word is not the
  result.
- **One fresh session per packet.** No executor session receives a second packet,
  a compaction summary or another executor's output.
- **Limits are enforced by tools, not by text.** The write list is enforced by a
  separate worktree or file permissions; the budget by the launcher's turn and
  time limits; credential reach by the isolation of the launch. A limit that
  exists only as a sentence is assumed to be broken.
- **Failure goes up, not sideways.** A failure note returns to the lead. The
  lead fixes the packet, splits it, or does the task itself. A packet is retried
  at most once after being rewritten. Executors do not hand work to each other.
- **Review belongs to the lead.** The lead reads each executor diff once, as it
  would read a contributor's patch. An executor does not review, accept, plan,
  triage a failing gate or judge whether a test or the product is wrong.
- **Owner words stop at the lead.** The owner speaks to a lead. The lead turns a
  request into packets or asks the owner one question. Owner messages are not
  forwarded to executors and are not quoted in packets.

### When the builder is an executor

The split arrangement assumes two leads. When the local binding records
`Builder tier: executor`, the arrangement changes as follows and the paragraphs
of "Split-mode responsibilities" that make the builder a team lead do not apply.

- The architect is the only lead. It designs, writes packets, runs the queue,
  runs every check, reviews every diff, integrates and commits to the integration
  branch, and reports to the owner.
- The builder session receives packets one at a time from the architect and
  works only in the task worktree the packet names. It does not dispatch workers,
  edit plans, write briefs, run the full gate, synchronize repositories or answer
  the owner on project matters.
- When the owner writes to the builder directly, the builder replies with one
  line asking the owner to tell the architect, forwards the owner's exact words
  to the architect once, and continues or stops its current packet as those
  words say. A stop or pause from the owner takes effect at once.
- The builder never receives a standing goal or stop hook. Its only goal is the
  packet in hand.
- Between packets the builder is idle. Idle is a correct state; it does not look
  for work, re-read rules or send status messages.
- If the architect is unavailable, the builder finishes the packet in hand,
  reports, and stops. It does not continue from the plan on its own.

The same rules apply to any other role given to an executor. A model of unknown
strength starts as an executor or a bounded lead and is promoted by the owner
after it has completed real work, not by its own assessment.

## Synchronizing collaboration configuration

Either role may propose synchronization. Execution requires direct owner approval
or an applicable standing grant recorded in project policy; the proposal itself
does not grant authority. A peer cannot override the receiver's harness permissions.

Read both current copies, compare them, preserve unrelated edits and resolve
conflicts explicitly. Each session edits its own checkout. Verify local bindings
are identical for the participating arrangement, untracked and excluded from Git.
For multiple machines the shared binding may name each machine explicitly; do not
assume one machine's path is reachable from another.

An authorized local-binding synchronization is an exception to committed product
transfers, not permission to deliver uncommitted source code. It does not authorize
new product scope, security relaxation, harness-permission changes or publication.
Versioned protocol and project-policy changes follow ordinary project change and
review authority; copying a grant into another project does not activate it there.

## Delivery and repository synchronization

Use the project's existing brief/report locations. A brief names goal, non-goals,
governing contracts, exact accepted/base commits, destination branch, allowed
writes, dependencies, acceptance checks and integration owner. A delivery adds full
delivered hashes, source checkout/branch, changes, results, failures and omissions.
Messages point to durable artifacts rather than being the only accepted record.

Either role may propose repository synchronization, including integration before
push. Routine execution requires an explicit request or a standing project grant.
Bind exact source/base/destination/scope; act only within the existing mandate.
Initiation does not transfer integration ownership or displace another maintainer.
Do not commit unrelated or unreviewed dirty work merely to make it transferable.

Transfer committed revisions, not directories or dirty snapshots. The receiver
reviews ancestry, exact diff, contracts and evidence, then integrates locally.
Require a clean integration tree or separately authorized task tree. Do not stash,
discard or commit unrelated changes, force-update history, choose blanket conflict
resolutions or copy `.git` to make synchronization appear successful. A mismatched
advertised hash requires fresh evidence, not following a moving branch blindly.

**Separate clones:** objects, refs, indexes and stashes are independent. Fetch the
advertised source ref into a fresh receiver ref, then compare its full hash:

```sh
git -C "<receiver-root>" fetch --no-tags "<sender-root>" "refs/heads/<source-branch>:refs/remotes/peer/<delivery-id>"
git -C "<receiver-root>" rev-parse "refs/remotes/peer/<delivery-id>"
```

Do not push into the peer checkout. Across machines use an agreed authorized
transport. **Linked worktrees:** verify the shared common directory; objects and
ordinary refs are already shared. Use distinct checked-out task branches. Never
force the same branch into both trees or update the peer's checked-out branch.
Stashes and much Git configuration are shared, not private session storage.

Integration is separate from external publication. Follow the project's declared
publication mechanism (for example a reviewed pull request, a single authorized
remote or a mirror rollout), target refs and approval requirements. No universal
mirror or direct-push policy is implied. Ordinary deliveries are not formal custody
handoffs. Coordinate ports, services and caches; separate paths do not isolate
shared resources. Keep separate build outputs unless explicitly authorized otherwise.

### Ideal synchronization

This is a shared operation of both agents. The owner may request it at any time in
either session; recognize intent, not an exact command. Defining the operation or
reading this protocol does not invoke it. Project policy states the routine commit
and integration authority conveyed by an invocation; missing authority is not
invented. External push needs its own applicable authorization.

The normal target is the same agreed Git state and completed functional delivery:
for separate clones, the same integration commit and tracked tree on the agreed
integration branch, with agreement about completed features, verification and
readiness to push. Matching HEAD alone is insufficient. Inventory staged, modified
and untracked product work on both sides; preserve and account for unfinished work.
If it prevents the target, report partial synchronization, not ideal completion.
Caches, build outputs, credentials, personal state and Git administration are not
required to be byte-identical. Linked worktrees use shared integration refs and
equivalent reviewed content without forcing the same checked-out branch.

Coordinate a stable snapshot with both agents and active workers. Report actual
roots/common directories, branches, full HEADs, dirty work, completed deliveries,
unfinished candidates and checks. Agree inputs; the configured owner integrates,
and each session updates/verifies its own tree. An unavailable peer or unresolved
live writer is a blocker, not implicit consent. Do not stop shared services.

If publication is authorized, use the project procedure and verify the advertised
remote hashes. Report both final local hashes, tree state, functional deliveries,
checks, preserved exclusions and publication status. An unverified or failed push
is not successful publication.

### Emergency preservation

The owner may explicitly request committing and pushing unfinished work before
session loss or resource exhaustion. This is distinct from normal synchronization.
The request covers the named work and expressly requested publication, subject to
project publication/safety policy. It does not certify completion, acceptance,
test success, release readiness or deployment suitability.

Capture stable inventories and safely quiesce affected workers. Preserve requested
product work, including new files, in clearly identified checkpoint commits. Group
independent changes where feasible and record incomplete, failing, untested and
unreviewed parts. Inspect the inventory; blanket `git add -A` is not a substitute.
Even "all work" excludes secrets, private personal/corporate data, ignored local
bindings, generated personal instruction artifacts and unrelated machine state.
Existing large-blob/security approval gates survive. Report exclusions safely.

Name the checkpoint destination and exact hashes. Preserve divergent candidates
on separate checkpoint branches rather than forcing a merge. Do not silently put
unfinished work on mainline/release refs. Use the owner-specified destination or a
non-release checkpoint branch permitted by project policy. If remote destination
or procedure is unresolved, preserve authorized safe local commits and report the
remote blocker; do not invent another publisher or bypass project restrictions.

Verify remote checkpoint hashes before claiming remote preservation. Distinguish
local preservation, remote preservation and ideal synchronization: two preserved
unfinished candidates need not be identical integrated functionality.

## Adoption and configuration

1. Retain this protocol unchanged where possible. Have the owner approve a project
   policy, written from `project.template.md` as `WORKING-RULES.project.md`,
   naming integration ownership/branch, publication procedure and targets,
   verification gates and standing grants. Do not copy another owner's approvals.
   Without grants, explicit requests and existing project authority govern actions.
2. Ordinary development needs no local file. To opt into split mode, prepare
   separate clones or linked worktrees using normal Git procedures, not `cp -R`
   of `.git`. Verify each tree's pointer/common directory; copied registrations do
   not prove ownership and are never automatically repaired/pruned/removed.
3. Locate the local exclusion file with
   `git rev-parse --path-format=absolute --git-path info/exclude`. Preserve its
   contents and add `/WORKING-RULES.local.md`. Separate clones need separate
   exclusions. Do not add machine-specific configuration to committed `.gitignore`.
4. Copy `local.template.md` to `WORKING-RULES.local.md` without overwriting an
   existing binding. Resolve every placeholder, topology, root, session and
   execution restriction. Each session prepares its own copy; compare the
   completed arrangement before writes.
5. Verify `git check-ignore -v -- WORKING-RULES.local.md` names the exclusion and
   `git ls-files -- WORKING-RULES.local.md` prints nothing. Exclusion does not untrack
   an already tracked file; stop for explicit resolution. Never force-add it.
6. Read root instructions, this protocol, applicable project policy and local
   binding; verify actual Git state and report role/task/write boundaries. Reading
   configuration creates no branch, repair, commit, sync or publication operation.

To return to ordinary mode, coordinate both sessions and finish or explicitly
reassign in-flight deliveries before removing/archiving local bindings. Removing
one file does not cancel the peer's task. Installing, updating or removing this
package never replaces project-owned policy or local bindings, activates no
example grant and copies no personal state.
