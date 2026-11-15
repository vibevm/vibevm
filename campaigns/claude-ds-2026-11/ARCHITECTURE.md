# claude-ds source-home architecture proposal

Date: 2026-11-15. Status: architecture and bounded implementation direction;
implementation remains subject to the accompanying brief and independent review.
Builder relayed the owner's expanded request: create the owned claude-ds package
and clean launcher, globally installable through ordinary `vibe install -g`.
This does not authorize publication, installation into actual host state, changing
a live endpoint, or moving live runs. Access credentials must never enter source.

## 1. Repository and package boundary

Recommended canonical source slot:
`vibevm/vibepacks/org.vibevm.world/claude-ds/v1.0.12/`.
Proposed coordinate `org.vibevm.world/claude-ds`, kind `tool`, version `1.0.12`,
UPL-1.0, subject to existing manifest validation and dependency/license review.
Builder relayed the owner's explicit version decision: this package's initial
version matches the next VibeVM version, 1.0.12. This does not authorize a host
version bump or publication, or establish an automatic future version-sync policy.
These are source-home metadata, not a published version or accepted implementation.
Nearby world packages use this version-slot topology; the existing
zai-glm-claude tool demonstrates the manifest/artifact convention, not authorization
to reuse its provider, credential, endpoint or deployment configuration.

Here "repository" means an owned source home inside the current repository.
Do not initialize nested Git, add a submodule, remote or mirror target, or register a
specspace. A separately hosted repository is a distinct owner decision. The current
source mirror protocol remains unchanged and no push is authorized. Creating this
source home does not install the tool as a host dependency or add it to host boot.

Architect owns the architecture/contracts/brief and review. Builder owns source
selection, implementation, synthetic verification and integration in its own tree.
The owner retains final acceptance and publication/deployment decisions.

## 2. Minimal layout and initial scope

Proposed layout (not created by this document):

```text
v1.0.12/
  vibe.toml
  README.md
  LICENSE.md
  src/                 portable config, supervisor and approved adapters
  launchers/           thin explicit-entry wrappers, only when reviewed
  tests/               synthetic children, fixtures and bounded negative tests
  examples/            fictional config only; no live defaults
  vibevm/vibespecs/     package contracts and trust/operation documentation
```

Use Python standard library for the initial reviewed supervisor if the current
candidate supports it; isolate system-tool adapters instead of importing host
launcher state. Initial supervisor support is Linux/POSIX only: selectors, signals
and process-group supervision do not establish Windows compatibility. Document
Python/platform/system-tool prerequisites, versions actually tested and unsupported
platforms. The isolation adapter is initially Linux-specific and may depend on
bwrap and an admitted network adapter. Neither POSIX-oriented source nor use of
Python establishes native macOS or Windows verification. Missing enforcement
prerequisites cause refusal, never an unsandboxed fallback.

The requested delivery includes a clean launcher and valid application capability,
not an observer-only inert tool. Keep `[package] kind = "tool"` and add a valid
`[application] id = "claude-ds"` with ordinary Linux installer options, recipe steps,
launchers and `public_commands`. Capability, not kind alone, makes `install -g`
choose application lifecycle. Use the existing application payload/ownership/proxy
machinery; do not invent a parallel installer or edit the user's shell files.
The exact manifest must pass ordinary validation and application resolution before
claiming global installability; an application id alone is insufficient.

The recipe copies only admitted source into a relocatable `VIBE_OUT` payload and
provides `bin/claude-ds`, exposed by an ordinary owned `claude-ds` public command.
It does not download/install an external agent runtime, invoke an agent, contact a
provider, write configuration/authentication, modify harness settings or start a
service. Missing runtime/sandbox prerequisites are checked when execution is
requested, not fulfilled through hidden install-time effects. Support only Linux
architectures whose manifest selection and synthetic install tests are verified.
Use normal source/package artifact and application recipe conventions, not the
unrelated receipt-installed launcher deployment pattern from another package.

Source scaffolding, manifest/recipe and synthetic install fixtures may proceed
while the observer is under rework; final acceptance requires independently
reviewed observer, clean launcher, external configuration and enforced isolation
with fixed-destination transport. No runnable candidate is represented as accepted
merely because a manifest installs. The unaccepted lifecycle/cleanup manager is
excluded from the runnable/shipped tree; retain its protected candidate outside
this package. Grouping sources never promotes unfinished components implicitly.

## 3. Configuration and credential boundary

Public source contains configuration structure and fictional examples only. There
is no built-in service URL, private host/address/port, account, model identity,
credential, personal directory or machine-specific launcher path. Do not copy the
existing private wrapper and redact it afterwards: reconstruct a clean portable
adapter from reviewed requirements, using synthetic configuration from the start.
Examples use reserved fictional names and unmistakable dummy values, never real
endpoint fragments or plausible live secrets.

Actual executable/endpoint/model/auth reference, sandbox policy, approved network
destination and run/output roots are provided through explicit private runtime
configuration outside source and publication inputs. Authentication references a
user-supplied protected credential file or approved runtime channel, never a key
in tracked config, command line, process title, packet or report. The launched
service may need its own credential under an owner-approved exception; that runtime
exception does not permit storing or publishing the credential in package sources.
Reject missing/unsafe configuration before launching. Avoid ambient credential
search, unrelated HOME imports, implicit endpoint defaults and inherited auth
stores. Private config is data, never executable shell interpolation or Python.

Configuration authority is not worker authority. A packet cannot change endpoint,
credential file, network destination or host mounts. Network restrictions pin an
explicitly admitted destination with bounded transport; do not add generic CONNECT,
proxy or host-network fallback. Namespace/process/filesystem enforcement excludes
host/server/cloud/other-provider credentials, SSH/GPG sockets, unrelated environment,
file descriptors and host process state. Verify with synthetic sentinels, never
read live credentials to test exclusion. Public examples must not authorize a
particular private network or assume access to the owner's hardware.

Runtime model metadata is an operator assertion unless verified by an applicable
protocol; no advertised model is inferred from the executable's name.

### Explicit configuration surface

`claude-ds --help` and `--version` are local, configuration-free operations. Execution
uses `claude-ds --config <absolute-private-config-path> ...`; there is no ambient
HOME/provider-config discovery. The path is not credential content. Do not provide
`--token`, inline auth flags, an interactive key prompt or a command that echoes
resolved config. An explicit `--check-config` with that path validates bounded
configuration structure, no-follow metadata and required enforcement prerequisites
without reading credential content, starting the agent or contacting the endpoint.
No configure command writes credentials; documentation instructs the operator to
create private configuration and credential files outside the installed payload.

Missing config refuses with an actionable explanation of the external config path
and credential-file reference, before auth content is read or a process/network
connection starts. Invalid/unknown config fields refuse; machine defaults are not
substituted. Runtime arguments are structured argv, never shell evaluation. Reject
caller options that override protected endpoint/auth/mount/config settings or
activate an unsandboxed mode. Arbitrary inherited environment and ordinary harness
auth stores are not admitted by forwarding the CLI's remaining arguments.

Normal uninstall removes only application-owned payload/proxy state through the
existing lifecycle; it leaves external configuration, credential files, runtime
state and retained dialogues untouched. Update likewise cannot overwrite private
config or run state. A conflicting existing `claude-ds` command must follow normal
owned-proxy conflict policy, never be replaced by bespoke shell installation.

## 4. Source, runtime state and observations

Source stays in the versioned package. Per-run writable state lives in a private,
explicit external runtime root: isolated HOME, transport sockets, progress/logs,
transcripts, resume state, source snapshots and outcomes. Do not write those beside
installed code or include them in package/source exports. Private credential input
is not a retained run resource and must not be swept into archives.

No migration, rename, deletion or replay of existing live/retained runs is part of
source organization. Existing dialogues retain their original dependencies and
protection. A new launcher can adopt a future run only through explicit admission,
not a directory scan of old scratch state.

The current WORKING-RULES.runs.md observability contract governs local acceptance:
heartbeat is observer liveness, milestone is a correlated worker claim, actual wait
is process outcome. None grants acceptance or cleanup. Package contracts may later
state these verified mechanisms without copying this pair's owner grants, paths or
local execution policy. Do not package the collaboration rules through this work;
the earlier no-rules-package trial decision remains binding.

## 5. Publication-input admission, not just secret scanning

Build source/package candidates from an exact reviewed file allowlist and retained
Git identity, never recursive copies of launcher directories, HOME or run roots.
The proposed package-local publication-input gate checks source Git diffs, its
explicit allowlisted staging, synthetic archives and Git-tree fixtures, and the
resulting payload inventories. This is an initial promotion gate, not an assertion
that generic host publishers enforce it. A path or content finding refuses before
commit/export; do not silently delete an offending file and claim the original
artifact identity. Fail closed when inventory, file type or inspection is unknown.
Admit regular files only initially; refuse symlinks, hard-linked external inputs,
unsupported entries, nested repositories, unbounded/opaque blobs and generated run
state. Bound file count/size and inspection resource use.

Check the entire candidate, not only filename suffixes: tracked or force-added
private files are just as prohibited as untracked ones. Test suspicious added,
modified and renamed paths, archives and final payloads. Public examples are
explicitly allowlisted as synthetic; do not blanket-ignore tests/examples because
they may contain an accidentally real secret. Credential-looking content and
private endpoint/path markers require investigation or refusal, not automatic
redaction into a published replacement.

Scanning is defense in depth, not proof that arbitrary text contains no secret.
An unknown credential or encoded payload can evade a pattern. The primary boundary
is clean provenance, narrowly selected source, no private source import, bounded
input admission and independent review. Do not describe a regex gate as an absolute
credential-leak guarantee.

Synthetic gates must demonstrate both boundaries:

- Inject dummy credential/endpoint/private-state sentinels into a simulated private
  runtime/config root; they never enter staged source, archive or Git payload.
- Deliberately insert those sentinels into an otherwise allowlisted source/example
  or force-added candidate file; admission fails before commit/export.
- Symlink/hard-link/path traversal and nested Git fixtures refuse; no external bytes
  are read or copied. Oversized/opaque/uninspectable input refuses explicitly.
- Final exported inventory/bytes match the admitted inputs and contain no synthetic
  canaries. Test each actual supported exporter; unsupported exporters remain unrun,
  not covered by a single filesystem-copy test.
- Errors/diagnostics never echo a credential value or dump configuration/environment.
  Useful ordinary service/tool errors remain private and bounded as policy permits.

No real secret, private endpoint or actual transcript is a test fixture. No live
provider call, external push, install or cleanup is required for these gates.

### Existing host exporters are not this gate

Builder's read-only inventory identifies `vibe-workspace/src/publish/staging.rs`
(`stage_node`, `copy_tree_excluding`), `vibe-publish/src/git_publish/payload.rs`
(`copy_dir_controlled`, `walk_controlled`) and the Git-tree branch in
`vibe-publish/src/layout_publish.rs::publish_batch`. The filesystem paths exclude
Git metadata and some generated artifacts; they do not implement credential or
private-state admission. Git-tree publication can bypass the initial copy before
hashing. CLI registry/layout publishing frontdoors do not make a package-local
scanner universal enforcement.

Initial work may exercise an isolated allowlisted source gate and synthetic
archive/Git-tree inputs only. It must not modify generic publisher APIs or claim
that all ordinary publish commands refuse secrets. Any later use of a real host
exporter requires separately demonstrated admission for that exact path before
release. Broader exporter hardening is separate explicit scope; USER-PROJECT's
UP-PRIVACY already tracks its own personal-artifact boundary, not an automatically
authorized solution for every credential or this package. No exporter, mirror,
registry or host-publication changes are part of this source-home proposal.

## 6. Promotion and acceptance

Builder supplies a read-only sanitized inventory of candidate module names,
provenance, review status, dependencies, licenses, platform assumptions and actual
exporter paths, without private source/config dumps. Architect then binds a bounded
implementation brief to exact reviewed inputs and allowed package writes. Do not
copy candidates into the canonical slot merely because they exist or ran once.

Acceptance of each promotion unit requires independently reviewed source and
synthetic checks, unchanged credential/isolation floor, reproducible inventories,
explicit unsupported/unrun cases and integrated milestone review. Source-home
acceptance does not imply launcher trust, provider availability, package release,
remote-repository creation or permission to move/delete runtime resources.
