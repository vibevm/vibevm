# C.4 remaining routes: source audit

Checked against all four steps of `campaigns/next/tasks/M-16-C.json#M-16-C.4` at
`0151e0426`. This is a frontier record, not C.4 acceptance.

1. **Typed selection, input and approval.** The registered decision message and
   `FixedLifecycleRoute` bind real Validate questions and accepted evidence to
   the owner. `fixed_lifecycle/dispatch.rs` explicitly refuses any other phase.
   `machine_submission/lifecycle_adapter.rs` admits only Validate and an
   empty-world Install; its one-package Install path is test-only. Real
   nonempty Install and later lifecycle phases therefore have no equivalent
   machine decision/admission route.
2. **Requested controls.** Fixed Validate carries approved force and trace.
   Fixed Deploy selects one authored profile at host startup, but the current
   fixed lifecycle request rejects `deploy_profile`. The legacy MCP
   `lifecycle_run` accepts only `phase`, sets `assume_yes=true`, force and trace
   false, and passes no deploy profile. It remains a separate legacy surface;
   its defaults cannot be used as machine-mode approval.
3. **Structured phase and partial effects.** Fixed Validate, package, Cargo and
   Deploy routes publish owner-backed result/error envelopes, including the
   focused partial cases. The default lifecycle MCP runner still treats
   failures before its final report as `PreExecution` text. A general
   phase-aware owner snapshot and typed missing-report projection remain open.
   Fixed Deploy still lacks one isolated interruption *inside* apply; further
   post-effect variants do not address that case.
4. **One C.1/C.2 owner across adapters.** The four fixed routes use their
   incumbent reservation/journal owners and have scoped CLI/MCP/local API
   evidence. `DefaultLifecycleRequest` runs the ordinary inclusive phase chain
   and is not itself a C1 submission grant. The general nonempty lifecycle
   route and its cross-adapter identity remain open. Coverage inventory gaps
   are declarations, not executable services.

**Next broad implementation frontier:** extend the incumbent lifecycle owner
from the fixed empty-world Install to one real, trusted-source nonempty Install
request. The domain journal must reserve the exact source/plan under the
incumbent lease and prove Ready before effects. Produce the owner-bound typed
question and retained result/error first; register CLI, MCP and local API only
after that route exists. Bind tests to that owner seam and affected adapters.
The ordinary `lifecycle_run` shortcut cannot supply this authority.

## Exact local source binding progress (2026-10-05)

The internal one-package Install entry now compiles in production code and
requires a `BoundOnePackageSource` host port. Its root and exact tree witness
are checked before C1 reservation, after the durable install-entered stage,
and again before the attesting source factory is built. The concrete
`PinnedLocalFactory` implements that port with its retained registry/root
identity and original witness. The orchestrator's actual one-package install
and replay test passes, as does the post-stage source-drift refusal; the
package-source port test accepts the original witness and refuses a changed
witness or source bytes. This is a source-binding seam, not a registered
machine operation: no typed Install question, approval, controls or
CLI/MCP/local API route has been added. All four C.4 steps remain open.

The next internal atom removes the fixture package identity from that seam:
the trusted bound factory now retains the exact tree witness and pin, while
the owner checks a simple project with that one exact requirement and a
declaration-free source package. A second package and a different project
identity complete an Install and replay the same run; a mismatched project
requirement refuses before reservation or fetch. The 13 focused orchestrator
Install cases pass serially. This still exposes no new machine operation or
general registry choice; typed approval and cross-surface admission remain
the next owner boundary.

**Authority boundary before adapter work.** At `1cc67a12f`, `PolicyOwner`
persisted only `approve_captured_script` and three control permissions; using
that script bit for Install would widen an existing grant. The nonempty Install
submit entry was therefore confined to the lifecycle adapter. A subsequent
policy atom adds `approve_exact_local_install`, denied by default for old
records and every existing host startup. Enabling it changes the policy
generation while the old denied generation remains byte-compatible. The
permission still grants no effect by itself: a typed Install question,
approval, exact source/resulting-plan binding and current-generation check
before C1 remain to implement. The ordinary MCP `assume_yes` path cannot mint
that evidence.

**Typed local Install preparation.** The retained host now captures the
one-package request, pre-effect plan, source, physical root, scope and current
Install policy generation under the incumbent lifecycle lease. It issues the
registered JTD approval question with explicit requested options, validates a
bounded answer against the original question, and rechecks current policy and
source before returning process-local comparison evidence. A denied policy
bit, false approval, foreign scope, stale generation, forged plan, changed
source and unsupported force/trace/deploy-profile/control are refused; the
three focused tests pass. Source read failures retain `SourceUnavailable`
instead of being misreported as successful recapture with `SourceDrift`.
This evidence cannot invoke Install or approve the Install confirmation gate:
there is still no sealed C1/C2 submission profile or CLI/MCP/local API route.
The four C.4 steps remain open.

**Sealed exact local Install submit.** The owner now consumes its retained true
answer into a private profile. An admitted request and plan wrap the v3 source
and plan with scope, policy generation, approved options and resulting plan;
same-key replay checks the seal and physical root before attaching to C1, even
after Install has changed manifest/lock. A new key repeats current policy,
source and v3 plan checks under the incumbent lease immediately before C1.
The actual nonempty Install test invokes once and replays the same run without
a second fetch; a revised policy and changed source refuse before C1, and a
different admitted intent conflicts on the same key. Typed refusal causes
survive the adapter. The focused fixture family passes 16/16 and the typed
decision family 4/4. The accepted-answer seal is process-local, so cold result
inspection is available but effectful submit cannot be reconstructed after a
process restart. No CLI/MCP/local API route or general phase result projection
is claimed; C.4 remains open.

**Trusted local Install action and cold outcome.** A separate exact-local
route now accepts the registered prepare/answer/submit/inspect actions and
publishes the existing generated local envelope. Two route instances using
one retained host and scoped key observe one C1 run and one package fetch.
The sealed Install profile writes an admitted-input capsule; cold inspection
validates its C1 hashes and exact Install request/plan formats before assigning
the Install operation ID. A missing or damaged capsule is RecoveryRequired,
and an unsealed Install cannot project as this operation. After one actual
Install, deleting only the retained report payload yields a structured local
Error with the same run and phase, partial owner view and Uncertain effect,
never NoEffects. The focused fixture family passes 18/18 and the unsealed
identity negative passes 1/1. This is a trusted local API route only; MCP,
CLI and broader lifecycle phases remain open.

**Trusted Install grant binding.** An explicit host service now persists an
Issuing fence before policy and C1 namespace issuance, then binds the pointer
to physical project root, exact scope, policy source/generation, target OS and
canonical framed local-package witness. Reattach looks up the existing
namespace and policy; it never reissues. The six focused tests cover same
binding, changed root/source/policy, denied Install bit, incomplete issuance
and read-only reattach with a retained source port after byte drift. A strict
PinnedLocalFactory cannot be reconstructed from changed bytes after restart;
the service does not infer a new effect grant from that state. CLI startup and
MCP registration still need to compose this service with the action route.
