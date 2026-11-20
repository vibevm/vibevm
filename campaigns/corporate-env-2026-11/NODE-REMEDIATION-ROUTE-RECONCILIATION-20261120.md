# Node remediation decision route — existing contract and remaining evidence

Read-only source/receipt reconciliation requested by architect after prerequisite qualification. No product repair, runtime installation, supplier acquisition, provider invocation or broad completion claim.

## Exact admitted decision

`NPM-PREREQUISITE-DECISION-01.md` distinguishes a bare missing `command:npm` from an automatic-remediation grant. `cli/pkg.rs::InstallArgs.on_missing_command` admits explicit `defer` or `install-node` only on global installation. The top-level `commands/app/global.rs` supplies manual-defer/Cancel through its decision callback; automatic remediation is a separate explicit `install-node` branch into `global/node_remediation.rs::run`. A wrapper/installer caller must pass this exact typed/CLI decision, not treat unattended mode, generic install consent or arbitrary need text as runtime-install authority.

Automatic remediation additionally requires selected typed `npm_runtime` metadata bound to a digest-verified input and exact package/version/engine range. A bare command need cannot choose a supplier. Existing canonical supplier is `org.vibevm.node/node`, not the provisional coordinate in the older broad CORP brief.

## Separate consent and recheck

The existing one-shot `install_preparing_node` returns an opaque token before target pending publication and releases its lock. `prepare_provider` selects the exact compatible managed provider or resolves the canonical source/version. Attended execution asks a separate default-No confirmation showing the selected source/version. JSON/unattended execution requires both `--on-missing-command=install-node` and explicit `--assume-yes`; unattended alone refuses. Manual defer never reaches provider acquisition.

After consent, verified npm metadata precedes provider mutation. New provider installation uses existing app owners, hidden with recommendations disabled. The derived scoped PATH must resolve actual node/npm within that exact owned provider. Actual Node version must match receipt/range; npm's receipt launcher must use the selected Node, bare npm output must match launcher output, and Unix script identity must match the declared script. No system/default Node mutation is inferred.

`continue_after_node` takes the opaque token once under a fresh target lock; it revalidates settings/store, prepared source/option/terms, prior target installation and default, then recomputes prerequisite facts using scoped command lookup before applying the incumbent plan. Changed target identity refuses; no second remediation is requested. Provider state may remain after a subsequent target failure, as documented rather than falsely rolled back.

## Existing actual evidence, not a new run

Accepted `NODE-REMEDIATION-RESULT-20261119.md`, product `b7f6d7d43ea3d5c58ce85afa02d0935c30d4c8b8`, exact source manifest SHA-256 `36dabd42f699c3af15a4655c40b268815e148fde3a0c4af98885d168948b4fb2`, binary `74612f42b6d2baab86a44a26dd2d040164221db037cd49de2656b5b4945bd380`:

- `no-explicit-provider-consent`: actual public exit2 `runtime_remediation_refused`, no target.
- `explicit-provider-and-one-target`: actual public exit0, canonical real Node26.13.1/npm11.20.0 in scoped payload, one synthetic target effect, no global Node default.
- `remove-target-keep-provider`: actual exit0, independently owned provider retained.
- `mismatched-in-root-npm-refusal`: actual exit2 `runtime_remediation_refused`, unchanged index and target-effect count.

Physically inspected receipts are under `/data/git/vibevm-node-path-real-20261119T191322Z/evidence/`, including `remediation-code-fixed-steps.json`, `remediation-code-fixed-index.json` and `remediation-candidate.json`. Some prose in the original result omitted the `evidence/` path component for individual files; the actual files were read, not inferred missing. No original receipt was edited. Library tests additionally cover handoff lock release, fresh facts and concurrently installed target; their accepted report is not substituted for a new CLI/native test.

New accepted regressions `207e9d275` and `33343894b` prove synthetic missing-command defer/retry and actual attended Cancel/manual defer separately. They do not prove a real npm-backed agent installation.

## Remaining precise gap

The core decision/consent/recheck mechanism already exists and has bounded independent acceptance. Adding a second installer handoff or repeating its accepted implementation would be redundant. Still open: a selected real npm-backed agent with its complete pinned offline dependency closure, broader provider negative/concurrency/platform cells required by scope, and eventual optional onboarding adapter through the same existing owner. A lone supplier npm tarball is not an offline closure; real Node/npm supplier evidence with a synthetic target does not close this gap.

Next finite architectural packet should select one of those exact open cells and its already admitted local artifacts/runner. Without such a packet this analysis authorizes no provider/system install, external package download, Node/npm installation or adapter product edit. Guarded source-rebind remains a separate real RED; NST and all five CORP stages remain open at their actual boundaries.
