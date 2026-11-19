# DS-PACKAGE-01 — clean globally installable claude-ds

Date: 2026-11-15. Bounded implementation brief. Builder relays explicit owner
request to put the clean launcher in its owned vibepacks package and make it
installable with `-g`. Architect owns architecture/brief/review; builder owns coding,
workers, synthetic tests and integration. No external publication or real host
installation is authorized by this brief.

## Inputs and binding

Architecture: `campaigns/claude-ds-2026-11/ARCHITECTURE.md` delivered with this
brief. Predecessor architect commit `99dd6d36623b8fc73975dcf12a4fa2abaeb1c6a5`;
exact delivery commit is sent after creation. Source checkout
`/fast/git/vibevm-architect`, branch `spec/user-project-root`. Verify the advertised
commit and parent, not a moving tip. Builder records its actual assigned task
branch/root and committed contract base before writes; preserve USER-PROJECT
candidates and unrelated work. Product scope is a separate package, not completion
of USER-PROJECT or broad publisher hardening.

Candidate observer first reported module SHA-256
`20df2ce466502260e0e3e403c710126f77463039e96d4f9afbda24ea26d6e3f6`, tests
`ddf5c4c534354afa84bb46c028362df59d3a668fa730857d0e36f1c154cf897c`.
These are unaccepted historical input references, NOT approved promotion bytes.
Builder binds exact reviewed rework identity, disposition of known blockers and
independent tests before integrating the observer. Do not modify a live worker's
packet or copy its moving output. Architecture/manifest/install fixtures can be
implemented independently while observer rework completes; final acceptance waits.
No old launcher/config dump or private runtime tree is an input.

## Deliverable and owners

Create source only under
`vibevm/vibepacks/org.vibevm.world/claude-ds/v1.0.12/`:

- Manifest: coordinate `org.vibevm.world/claude-ds`, version `1.0.12`, kind `app`,
  valid `[application] id = "claude-ds"`, verified Linux installer options, ordinary
  recipe step, owned launcher and `public_commands` exposure named `claude-ds`.
- Clean launcher, small focused stdlib source modules for config, enforcement/runner,
  fixed-destination transport and reviewed observer, plus synthetic tests.
- Relocatable recipe/payload, README, license/provenance, fictional config example,
  package-local admission/check tooling and English package contracts under its
  `vibevm/vibespecs/`. Do not install a host boot snippet or delegation policy.
- Minimal focused host integration tests using existing app fixture helpers, e.g.
  `crates/vibe-cli/tests/common/app_registry.rs` and adjacent app step/roundtrip
  test conventions. Prefer a new focused test target rather than changing unrelated
  application behavior. Builder names exact test files before editing.

Allowed host code changes: NONE by default. If existing manifest/recipe/public
command machinery cannot express this package, return the exact failing owner/test
and bounded necessary adjustment before editing core/app/CLI behavior. No new Cargo
or Python dependency, CI/toolchain/accelerator change, host version bump, mirror,
remote, nested Git, submodule, specspace registry or root instruction change.

The normal application lifecycle is authoritative for settings, selection, payload,
ownership, proxy conflict, rollback, update and uninstall. The recipe uses only
allowlisted package inputs and `VIBE_OUT`; it neither recursively copies a private
launcher directory nor discovers machine config. No external runtime download,
agent/provider invocation, credential/config write, profile edit, background service
or hook during install. No bespoke install script into the user's PATH outside
normal owned application exposure. Other tool package deploy conventions are not
proof of global application installability.

## Runtime interface and safety

Configuration-free `--help`/`--version`; explicit `--config <absolute-path>` for
execution; `--check-config` validates that private config's bounded closed schema
and metadata/prerequisites without reading auth content, launching or networking.
No secret-bearing CLI options or configure command that writes a key. Config data
holds operator-supplied runtime executable, endpoint, model, credential-file path,
private external state roots and approved isolation/network parameters; actual values
never enter source/examples/tests/reports. No real endpoint/model/machine defaults.
Missing/invalid config refuses before credential read/process/network start with
path-only actionable guidance. Do not print resolved config or environment.

Clean reconstruction must preserve existing authorized safety requirements, not
copy/redact the private wrapper. Structured argv only, allowlisted environment,
private HOME, restricted filesystem/process/network, closed unrelated descriptors,
no host SSH/GPG/cloud/other-provider auth, pinned explicitly approved network
endpoint, finite transport budgets and no proxy/unsandboxed fallback. Caller/worker
arguments cannot override protected endpoint/auth/mount/config settings. Linux/POSIX
supervisor and Linux enforcement initially; do not advertise native Windows/macOS
support. Unsupported host/prerequisites refuse safely. Runtime own-service credential
access still requires applicable operator authority; packaging creates none.

All state is external/private, never beside source or installed payload. Uninstall
and update preserve external config, credential files, run state and dialogues.
No live-run migration, cleanup manager, broad scanner, process killing or replay.
Observer behavior must meet WORKING-RULES.runs.md, with worker milestones distinct
from runner heartbeat, exact Write/result correlation, bounded protected logs,
explicit loss records, actual wait evidence and separate observer failure. Reuse
mechanisms as contracts, not this pair's private machine settings or owner grants.

## Verification and source admission

Use synthetic fixtures exclusively. Record exact commands, selected counts, passed,
failed and unrun gates; worker selftests do not establish independent acceptance.

1. Ordinary manifest validation/application resolution accepts kind app plus
   application identity and complete installer options. Keep the validator unchanged;
   a tool-kind manifest with installer options must still refuse. Offline local-registry `vibe install -g` in isolated
   settings produces the normal owned `claude-ds` command/proxy. No extra host code
   route or personal-spec selector is needed.
2. Install with fake runtime/network/auth sentinels proves none are invoked/read or
   written during plan/install. The payload is relocatable, not tied to source cwd.
   Installed `--help`/`--version` work without private config. Test update/uninstall,
   existing command conflict and retention of external synthetic config/state.
3. Missing config, unknown/oversized fields, unsafe paths, symlink/substitution,
   protected argument overrides and missing sandbox/runtime prerequisites refuse
   before effects. `--check-config` neither reads auth content nor launches/connects.
   Root/capability-dependent fixtures name their privilege assumptions.
4. Synthetic runtime verifies exact argv/environment, intended own-service auth
   boundary, isolation from unrelated sentinels, fixed approved destination and
   below/exact/over transport budgets. No real service or real credential input.
5. Observer worker tests and independent blackbox/race tests pass against exact
   promoted modules: log exclusive/no-follow identity/permissions/hardlinks, rotation
   gaps, disk failures, actual exit with inherited pipes, worker idle vs heartbeat,
   emitter attribution and terminal/cancellation/timeout separation. Module splits
   must preserve verified semantics; report unsupported cases honestly.
6. Package-local source admission uses an exact bounded allowlist/regular-file
   inventory and synthetic secret/private-state canaries. Test unexpected/force-added
   source/example files, path escapes, symlink/hardlink/nested Git, opaque/oversized
   inputs, synthetic archive/Git-tree payload identity and final absence of canaries.
   Never scan real auth stores or dump the private wrapper. Pattern scanning alone
   cannot prove no secrets; provenance and independent review remain mandatory.
7. Report tested Linux architecture(s), Python and enforcement prerequisites. Add no
   installer option without correct selection/payload semantics and coverage; do not
   claim unexecuted native architecture/platform tests passed.

Generic host exporters remain outside scope and are not credential gates merely
because package-local tests pass. No actual export/publication/install into real user
state is part of verification. Tests may create disposable synthetic Git repositories
only in explicitly bound scratch roots, never nested Git in the package/host tree.

## Delivery, review and stop

Deliver exact contract base and frozen candidate manifest/hash, source inventory,
provenance/licenses, independent gate evidence, known observer blocker dispositions,
install/output samples without secrets, applicable platform claims and unrun cases.
Architect reviews the integrated launcher/config/isolation/observer/application
lifecycle before milestone closure. No final acceptance from successful installation
alone. If a safety seam cannot be enforced, preserve candidate and report a blocker;
never fall back to direct unsandboxed runtime or a different provider/credential.

This brief authorizes bounded implementation under the relayed owner scope, not
provider calls, retries, source commits, external push, package publication, release,
remote creation, actual host deployment or deletion. Follow currently applicable
local execution authority separately. Resume USER-PROJECT only under the agreed
owner priority after the observability/package milestone boundary is accounted for.

## Manifest admission correction — 2026-11-15

The initial kind-tool recommendation was incorrect for an executable globally
installable package. Independently inspected
`crates/vibe-core/src/manifest/document/validation.rs:225-244` requires both
application identity and package kind app for installer options. The failed
isolated install reported by builder exposed this earlier missing validation check.
Use kind app; do not weaken the validator or change host routing to accommodate
the previous draft. Group/name/version and external credential boundaries remain
unchanged. A successful config unit suite does not prove installability.
