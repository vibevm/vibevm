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

**Authority boundary before adapter work.** `PolicyOwner` currently persists
`approve_captured_script` and three control permissions, with a generation
derived from those facts. It has no Install permission; using that script bit
would silently widen an existing grant. The new nonempty Install submit entry
therefore remains visible only inside the lifecycle adapter. The next typed
Install admission must add its own explicit trusted policy fact and bind the
current generation, exact source witness and resulting plan before any C1
reservation. The ordinary MCP `assume_yes` path cannot mint that evidence.
