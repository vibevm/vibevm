# Package defaults: concrete reviewed retry transport

Read-only appendix to `/tmp/nst-package-defaults-next-design.md`. No source, tests, builds, Git, network or agents changed. **Recommendation: ship the review path with defaults/profile support**, using one typed review challenge and one bound acknowledgment across attended CLI and machine calls. Rediscovery is a resumable decision boundary, not the permanent endpoint of the feature. Proposed API/flag/protocol names below are design names, not claims they exist today.

## Existing seams and the constraint they impose

- `commands/install/direct.rs::prepare_direct_install` owns one selected workspace snapshot, lease and trace identity. `execute_after_open` constructs `CliConfirmGate`, source/environment ports and `InstallExecution`, then calls `vibe_orchestrator::execute_prepared`. Keep this ownership/finalization chain.
- `vibe-install/src/plan.rs::Plan` currently has only Fresh and Ready. Ready owns the resolved plan consumed by apply. A proposal still requiring semantic review must not masquerade as Ready.
- `vibe-orchestrator/src/ports.rs::ConfirmGate::confirm_install(packages)` is materialization consent only. `install/ready.rs` calls it immediately before native apply. It receives no candidate, consumer or basis.
- `commands/install/confirm.rs::requires_confirmation` deliberately bypasses material confirmation for `-y`, JSON, unattended and typed user-project scope. **Do not reuse that boolean policy as semantic review approval.**
- `cli/pkg.rs::InstallArgs::inputs` projects ordinary surface input into `InstallInputs`; `vibe-orchestrator/src/install/args.rs` has no review input today. `plan_only` is an application/global flag, so it is not an existing generic package preview command.
- `commands/install/draft.rs::from_run` sets success and builds the registered InstallReport. A required review must not travel through that success constructor. The existing command-finalization funnel must finish exactly once even when planning needs input.
- Existing app `configuration/plan.rs:315-349` renders typed before/after review and asks explicitly, then evaluates again with reviewed authority. Reuse the presentation convention/common question widgets (`output/prompts`), not app wire records or app instance ownership.
- MCP `tools/lifecycle_run.rs` currently accepts exactly `{phase}`, constructs `InstallInputs::default`, and supplies `HostedConfirmGate`, whose materialization confirmation always returns Ok. Its strict decoder and tool schema must be deliberately extended if agents can acknowledge package review; hosted materialization policy must not implicitly approve it.

## Minimal operation contract

Add a typed **NeedsConfigurationReview** planning outcome carrying an owned proposal/challenge. It is not a parked agent task, and it creates no new durable continuation/receipt store. Keep captured source observations and actual original consumer inputs in the existing operation planning context. The proposal is non-executable until bound approval is checked; any preview of changed defaults/effects must be labeled proposed, never installed/authorized state.

The challenge has two parts:

1. **Binding**, renderer-independent and deterministically fingerprinted:
   - actual canonical workspace/installation scope and selected node;
   - digest of complete relevant authored root/consumer inputs, including distinct exact/additive/omitted/empty input, plus previous retained basis identities;
   - exact proposed configured package candidate coordinate/version and immutable source/content identity, not only package name or local feature IDs;
   - proposed semantic basis fingerprint, automatic-reliance facts and affected real consumer keys (including instance-closure review where applicable);
   - the selected/proposed dependency-effect meaning the user is being asked to approve. If the rendered effect plan promises exact selected targets, bind those targets too.
2. **Presentation data**: previous/current definition/profile/default meaning, reasons for review, original versus proposed selection and added/removed dependencies, with actual package/source and consumers named. Labels/order/terminal colors are not the binding authority. Parameterless scope excludes secret answers; do not serialize source credentials.

Call the typed binding fingerprint `review_id`. It excludes run IDs, timestamps, cache paths and progress. It is drift detection, **not authentication or proof that a human read a screen**. Approval comes from the explicitly invoked CLI/tool input. No server-side nonce database is required.

Acquisition of the exact proposed candidate may use the existing read-mostly source/cache owners before presentation. Do not promise approval of an immutable candidate if the proposal has only an unbound `(name,version)` lookup. Actual fetched manifest/content must agree with the proposal before Ready/apply.

## Attended same-operation path

Extend the existing confirmation port with a separate method, conceptually:

`review_package_configuration(&PackageReviewChallenge) -> Result<ReviewDecision>`

The default for existing/headless implementors is **Defer**, never Approve. A decision is Decline, Defer, or an acknowledgment bound to this challenge; it is not an unscoped `reviewed=true` flag. The existing `confirm_install` method remains unchanged and is still called after a fully admitted Ready plan.

CLI behavior:

1. Ordinary `vibe install --path P` reaches the typed review outcome before project semantic mutation.
2. If actually attended (terminal, not JSON/unattended), render the shared typed before/after data with existing question/confirm conventions and default **No**. Generic `-y` and automatic user-project materialization scope do not bypass this question.
3. On explicit approval, the operation owner validates the acknowledgment against its captured challenge and continues planning against the same bound observations/history. Re-evaluate only with that exact approved authority; do not restart with `previous=None` or treat all candidates as reviewed.
4. Once the final candidate/effects are still the approved ones, use the existing materialization confirmation and apply/publication owners. The selected automatic basis is written with successful lock publication; the review input file/argv is not a new durable receipt.
5. Decline stops with no semantic writes. Defer returns the machine-readable challenge. If a new candidate is required after feasibility/backtracking, prior approval does not transfer: produce a new challenge and ask again explicitly. A repeated identical unresolved challenge after approval is a no-progress error, not an endless prompt loop.

This is a review loop around planning, not another solver. The current candidate cache must not reuse an unreviewed error as a reviewed result: keep immutable metadata observations separate from the evaluation under the specific acknowledged basis (or rebuild only that evaluation over the same snapshot). Neither action may renew another consumer's basis.

## Separate-call machine retry: one input flag, no authored review key

Add one package-install input, proposed spelling:

`--configuration-review-input FILE`

It reads a strict generated `vibe-package-configuration-review-input/1` document at the binary boundary and passes decoded acknowledgment through InstallInputs. Do not add a review flag/receipt to vibe.toml, and do not reuse the application-only configuration input by accident. The relevant package route admits it; unrelated app/global routes must not silently ignore it.

Suggested minimal input shape:

```json
{
  "protocol": "vibe-package-configuration-review-input/1",
  "review_id": "<fingerprint returned by the proposal>",
  "binding": { "...": "exact typed binding returned by the proposal" },
  "decision": "approve"
}
```

The `...` above is explanatory notation, not an open Value/map schema: the real schema has the explicit typed scope/candidate/consumer/basis fields listed above. Echoing binding data provides exact candidate constraints for a fresh process without a server-side proposal store. The retry does not trust those fields: it reacquires the existing lease, loads current authored/history/source state, validates actual consumer membership, reacquires the named candidate, recomputes the typed binding and compares it exactly. The fingerprint is checked, not used to authorize an arbitrary supplied plan.

Concrete user/agent sequence:

```text
vibe install --path P --json
# Returns required review; no package effects. Client displays proposal.
# User/agent explicitly writes the bound acknowledgment to approval.json.
vibe install --path P --configuration-review-input approval.json --json --assume-yes
# Exact validation, then ordinary Ready/apply and successful install report.
```

Wrong workspace, changed consumer input/set, changed retained history, changed same-version content/definition, wrong candidate, missing reviewed candidate or changed shown effect meaning rejects the acknowledgment before effects. Return a fresh challenge when one is available. Never silently fall back to another version/default with the old approval. The bound retry can constrain the reviewed candidate exactly; if it is infeasible, fail truthfully or offer a different candidate for a new explicit review. Do not let approval become a version-independent permission.

A supplied acknowledgment must be admitted before a source-free Fresh shortcut can discard it: match it to the actual required/current binding or report unused/stale acknowledgment. Do not silently ignore a review file because the ordinary root/version snapshot happens to be fresh.

Unchanged automatic history can replay normally without supplying the file again; the existing lock's selected basis is the durable owner. Do not persist the raw acknowledgment file path or create a second approval ledger.

## Minimal output and external adapter compatibility

Preserve the registered install report root. `schemas/install_report.jtd.json` explicitly owns success/fresh/park/failure under one InstallReport; do not replace it with an unregistered success-looking review document.

Add an **optional typed `configuration_review` member** to that public report, containing a versioned package review challenge. When review is pending, report `ok=false`, `complete=false`, actual zero semantic progress and a stable review-required reason/non-success exit (proposed exit2). Do not label it Fresh, Installed, Skipped or an agent delegation. Existing non-review outputs omit the new field and preserve their byte shape. CLI direct failure/result classification must attach the typed challenge through InstallDraft, without parsing Display text, and finalize the existing trace once.

For agents using the already-installed MCP adapter, extend `lifecycle_run` with only optional `configuration_review` acknowledgment input, using the same generated input/binding representation. Existing `{phase}` calls remain valid. Keep server-fixed root/offline/provider/force policy unchanged; the supplied binding must match that actual server project. HostedConfirmGate still auto-confirms materialization only; absent acknowledgment yields review-required data instead of terminal prompting or automatic consent.

Reuse the same optional review challenge fragment in the generated lifecycle failure/report surface so MCP's existing one-finalize/structured-content funnel can carry it. Do not change the tool into a second CLI parser or add path/provider overrides. Update its descriptor's present “exactly phase” documentation, strict argument decoder and parity tests coherently. A graphical client needs only this typed proposal and acknowledgment; no GUI framework or package-specific HTML is needed.

Public changes requiring root's format review: one strict package-review input schema/corpus, one shared review-binding/challenge vocabulary, optional review output field in existing install/lifecycle reports, generated outputs, and tool/CLI capability documentation. The approved new automatic-record lock epoch remains the separate durable change. App wire/index/steps and authored package configuration grammar stay unchanged for review transport.

## Minimal implementation closure

- `vibe-install`: typed review planning outcome/proposal acceptance over existing candidate/source/consumer snapshots; Ready only after exact approval.
- `vibe-orchestrator`: InstallInputs decoded acknowledgment, separate semantic-review port method, planning/retry/defer handling before the existing Ready cell; preserve lease/trace/callback ownership.
- CLI: pkg.rs flag and inputs projection; direct.rs decode/funnel; confirm.rs attended adapter; small typed review renderer using existing prompts; draft/report optional review projection. Normal destination confirmation keeps its current semantics.
- MCP: lifecycle_run request decoder/schema and same InstallInputs; structured review result through existing finalization. No new answer host.
- Core/shared basis and automatic lock representation follow the already agreed design. Review approval does not move evaluation/instance ownership into the renderer.

## End-to-end oracle required with the feature

1. Establish an actual automatic install, then change candidate/default/profile meaning while retaining the same automatic authored policy. JSON ordinary install returns a typed review challenge with both real consumer identities, prior/current basis and proposed effects. Lock, manifests, slots, boot and hook counters are unchanged; `-y` alone gives the same pending outcome.
2. Actual PTY attended run renders that same meaning. Answer No: no effects. Answer Yes: exact selected graph applies and its new basis survives a cold process without another question. Supplying `-y` must not suppress the semantic question.
3. First machine call returns the proposal; a separate process submits the exact bound acknowledgment and successfully installs. Compare installed dependency graph/hooks/boot and durable original/basis with the proposal, not just exit0 or a filled field.
4. Between the two calls mutate one binding dimension at a time: actual consumer input/set, workspace root, prior basis, selected version, same-version source content or semantic definition. Each old acknowledgment fails before semantic writes. Re-offer/re-approve the actual new candidate and prove completion, not a permanently refusing workflow.
5. Make the approved candidate infeasible so solving would prefer a different candidate. The old approval must not authorize that candidate; a new explicit review is required. Rejected candidates never become lock receipts. Infeasible changed proposals must not be mislabeled ready.
6. MCP's phase-only call defers with the same challenge; its bound retry succeeds through the same operation owner. Hosted materialization auto-yes never approves semantic review. CLI/MCP malformed/unknown acknowledgment fields and out-of-scope consumer entries refuse; existing no-review output/tool calls remain compatible.

## Root decisions to freeze before dispatch

- Approve the single package-only acknowledgment flag and optional MCP input spelling, plus generated output placement/non-success code.
- Fix the canonical binding recipe (scope + actual full consumer/request/history + exact candidate/source + semantic/effect meaning), excluding presentation/runtime noise; decide whether displayed effect targets bind a full selected graph or only the explicitly shown package-effect subgraph. No weaker identity than the displayed promise.
- Approve this complete attended and separate-call retry path in the same defaults/profile feature slice. Do not ship the feature with only permanent rediscovery refusal or silently defer every agent approval to a future generic UI system.

These are finite operation/format choices. No new solver, authored review key, receipt store, durable proposal host or speculative GUI architecture is proposed.
