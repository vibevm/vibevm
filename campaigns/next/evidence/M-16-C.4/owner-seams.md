# M-16-C.4 owner seams after scoped MCP observers

Read-only source audits at `ecc49e3f0` mapped the next C4 boundaries. This is
campaign evidence, not an implementation or acceptance claim. No tests or
effectful operations were run for this inventory. The governing contract is
`spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE`
and the four-step task is `campaigns/next/tasks/M-16-C.json#M-16-C.4`.

## Surface-neutral and local-session gap

- `crates/vibe-orchestrator/src/machine_operation.rs` owns the existing
  surface-neutral descriptor service, but its trusted callable route is only
  pure migration planning. Descriptor-only construction cannot admit another
  effectful operation.
- `crates/vibe-mcp/src/machine_session/runner.rs` constructs that pure route.
  `state.rs` assigns connection-local context/target IDs and checks them,
  then returns an unvalidated request. Its negotiated formats omit the
  registered lifecycle-action request/reply/error formats. Framing and
  negotiation prove neither lifecycle admission nor provider effects.
- `crates/vibe-mcp/src/tools/lifecycle_operation.rs` reaches the actual
  `FixedLifecycleDecisionHost`. Its Bridge retains opaque accepted evidence
  and jobs under bounds; `host.submit_scoped` reaches the incumbent C1/C2
  reservation/journal owner. The MCP adapter is not yet a shared CLI/local
  machine service. `ServerContext::with_fixed_lifecycle_host` supplies an
  explicit physical-root-checked grant; ordinary context construction does
  not grant it.
- The already registered `lifecycle_action_request`, `reply` and `error`
  formats provide the action vocabulary. A local effectful exchange still
  needs a trusted fixed-host route, explicit inner format validation,
  owner-bound outer context/target mapping and truthful error publication.
  Pure-planner `NoEffects` helpers cannot describe a post-dispatch failure.

The next bounded implementation is a trusted fixed Validate route in the
existing orchestrator library service, with MCP and local framing delegating
to one owner-backed action service. Descriptor data alone remains inert.
Cross-surface tests need actual MCP/local/direct-owner same-key races with one
domain run and effect count, stale/foreign grants, lost response, damaged
terminal evidence and no redispatch. This first route does not complete C4,
create a general executor or widen force/trace/deploy-profile authority.

## Existing domain journals versus missing adapter proof

| Domain | Observed owner evidence | Still missing for C4 |
| --- | --- | --- |
| Fixed Cargo build | `crates/vibe-lifecycle/src/mechanism/build/journal.rs` reserves exact run/plan/source under the lifecycle lease and publishes/readbacks intent, capsule and Ready before the executor; `journal/execution.rs` gates provider work and `journal/capsule.rs` cold-attaches. | Typed public admission/results and actual CLI/MCP/local conformance. The ordinary A2 artifact record is written after execution and is not its pre-effect journal. |
| Fixed static-file package | `crates/vibe-lifecycle/src/mechanism/package/journal.rs` and its execution/capsule modules provide a separate C1-backed Ready and cold result for the exact static copy. | Public typed admission and surface adapters. Do not wrap this owner in a second C1 reservation. Other package providers remain unproven. |
| Deploy | `crates/vibe-lifecycle/src/mechanism/deploy/{transaction,state,ledger}.rs` retain intent, physical destination locks, checkpoints and cold settlement under the deployment owner. | Exact C1 run/key correlation and Ready readback, retained per-run terminal history after receipt replacement, and a bound recovery route that cannot redispatch uncertain effects. An absence search over the named files found no C1 calls, not an absence of transaction state. |
| Publication | `crates/vibe-publish/src/publication_full_plan/{prepare,journal,cold}.rs` and `calls/{driver,owner,terminal}.rs` retain C1-backed preparation and per-call Pending/Landed evidence. | Typed protected public adapter and production Finalize authority. The current production owner returns Unavailable at Finalize; test-only fixture transport does not establish a publish path. Aggregate remote CAS is unavailable. |

Follow-up domain packets need separate owners and narrow tests. Package-static
and fixed-Cargo can reuse their current journal instead of inventing a new
one. Deploy requires an owner-side correlation design first; publication
requires an explicit bounded effect scope and finalize decision. No release
or public 1.1.0 publication is implied by this inventory.
