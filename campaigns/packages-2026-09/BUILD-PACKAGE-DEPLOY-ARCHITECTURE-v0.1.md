# Build, package, deploy and the VibeVM OS horizon

_Central design packet, draft v0.1, 2026-08-26. This document is not an
authoritative specification, changes no PROP status and authorises no publish or
system mutation. It records owner requirements and the implementation shape to
be proposed through `SPEC-DEBT-LIFECYCLE.md`._

## 1. Owner requirements captured here

The lifecycle must support all of these without turning one into a special-case
command hidden beside the lifecycle:

1. Build a fully static skill whose distributable is one `SKILL.md` file.
2. Build an Agent Plugins 1.0 directory and install its supported components for
   at least Claude Code, Codex and OpenCode.
3. Build an ordinary application from source. Rust/Cargo is the commissioning
   backend; Maven, Gradle, CMake, npm/TypeScript, Go and others must fit later.
4. Install VibeVM-managed tools through launchers in `~/.vibe/bin` while still
   allowing an ordinary application to choose another explicit installer.
5. Deploy the same packaged application locally or to a server through named
   profiles.
6. Let packages provide new build, package, acquire and deploy mechanisms, and
   let a host deliberately route a logical mechanism away from a VibeVM builtin
   to a plugin provider.
7. Preserve an algorithmic, fully usable baseline. LLM assistance is an
   explicit paid enhancement (`off | assist | required`), never ambient
   behavior caused by the presence of credentials.
8. Keep the model extensible toward VibeVM OS: packages may eventually describe
   verified prebuilt downloads, system-package-manager operations, services and
   configuration resources. That future is not implemented in this campaign,
   but today's identities, plans and receipts must not block it.

## 2. The phase boundary is the primary law

The nine-phase line remains:

```text
validate -> install -> generate -> build -> test -> create -> verify -> package -> deploy
```

The words have deliberately separate meanings:

| Phase | Owns | Must not own |
|---|---|---|
| `install` | Resolve and materialise VibeVM dependencies into the workspace | User PATH, agent homes, servers, registries or OS packages |
| `generate` | Deterministic derived source | Compiled binaries or external placement |
| `build` | Produce code artifacts from source, or later acquire a digest-pinned equivalent | Local installation, publication or server mutation |
| `test` | Deterministic tests over source/build outputs | Packaging or placement |
| `create` | Explicit agent/LLM-enhanced outputs | An algorithmic prerequisite for ordinary build/package/deploy |
| `verify` | Gate build/create artifacts and provenance | Mutating a destination |
| `package` | Produce portable distributables and client-native projections | Editing user/client/server state |
| `deploy` | Reconcile a selected distributable or declared resource into a destination | Compiling source as an undocumented side effect |

Therefore a local installation is a **deploy target**. It may be presented to a
human as “install locally”, but it does not become the dependency `install`
phase. `vibe clean` removes owned build/package outputs; it never silently
undeploys an external installation. Removal is an explicit inverse deployment.

`vibe deploy --profile local` runs the normal inclusive chain through `deploy`.
`vibe package` stops before any user-home, server, registry or OS mutation.

## 3. One mechanism plane; builtins are ordinary providers

Lifecycle contributions answer **when** work runs. Mechanism providers answer
**how** one declared target is planned, applied and verified. They share one
provider/handler machine but are different nouns:

- an ordinary `phase:build` contribution adds a task to the ritual;
- a build-role mechanism can service one or more declarative build targets;
- package and deploy use the same provider protocol at their own mechanism
  points;
- a future acquire-role mechanism produces a verified prebuilt artifact without
  pretending it was compiled locally.

The proposed successor to PROP-054 does **not** add a fourth time/point family.
`ExtensionPoint` remains the closed vocabulary of scheduled moments. Providers
use a sibling `[[mechanism]]` declaration with role
`build | package | deploy | acquire`. It reuses the exact provider key, handler
kinds, envelope, world collection, disable controls, static scan surface,
narration and `vibe extensions` registry. The only new operation is lookup: a
mechanism is inert until a target selects it, then its invocation is an ordinary
execution in that target's real lifecycle phase.

Example provider shipped by a package:

```toml
[[mechanism]]
id = "cargo-v2"
role = "build"
name = "cargo"
handler = { kind = "native", crate_dir = "crates/cargo-provider" }
protocol = 1
config_schema = "schemas/cargo-build-v1.jtd.json"
```

The built-in Cargo adapter is represented by the reserved provider key
`org.vibevm/vibe#cargo`, not by a privileged branch outside the registry.
Claude/Codex/OpenCode adapters and `vibe-bin` follow the same rule.

### 3.0 Mechanism carriage and registry shape — decision record (central, 2026-08-30)

**Decision.** The mechanism plane extends the ONE existing kernel
(`vibe-extension-registry`), never a second crate or a Lane-C registry:

1. **Carriage.** `ExtensionWorld`'s two source kinds each gain
   `mechanisms: Vec<MechanismDecl>` beside `declarations` — the same
   provider identity, the same collection walk, one world snapshot. The
   durable adapter (`DurableExtensionWorld`) reads `[[mechanism]]` out of
   the same manifests it already parses; no second parse, no second epoch
   rule.
2. **Builtins are a third, engine-owned source.** The reserved keys
   (`org.vibevm/vibe#cargo`, `#static-skill`, `#agent-plugin`,
   `#vibe-bin`, …) enter collection as rows from one
   `builtin_mechanism_source()` the collector ALWAYS appends — ordinary
   rows under the reserved provider identity, never a privileged branch.
   The reserved `org.vibevm/vibe` owner is refused to any collected
   manifest (impersonation is a collection error, same genre as the
   reserved-id-prefix law).
3. **Registry.** Collection yields a `MechanismRegistry` beside the
   extension rows: one row per collected `[[mechanism]]`, keyed by the
   provider-qualified mechanism key, carrying role, logical name,
   handler, protocol, config-schema path and its provider identity.
   Disable controls apply exactly as extension disables do.
4. **Selection is a pure kernel function.** `resolve_mechanism(role,
   logical_name, target_pin, host_routes)` applies §3.1's four steps
   verbatim and returns the selected row or a typed refusal listing the
   installed candidates. No filesystem, no version ordering, no
   short-name discovery — an unpinned foreign row is INERT, which is the
   replacement fixture's whole proof: a plugin that overrides a logical
   key displaces the builtin in the ROUTING result, and the builtin row
   is still present, still queryable, and demonstrably NOT selected.
5. **Execution is out of scope for this atom.** The provider protocol
   (§3.2 plan/fingerprint/apply/verify/remove/recover) is R8-CARGO's and
   later; R8-MECHANISM lands the plane, the routes and the selection law
   only — a mechanism is inert until a target selects it, and no target
   selects one yet.

**Considered and rejected.** A second registry crate — two homes for one
provider machine (the Lane C plan note already forbids it). Widening
`ExtensionPoint` with mechanism roles — §3 keeps the closed time
vocabulary and mechanisms are not scheduled moments. Letting the builtin
defaults live as a match in the resolver — a privileged branch outside
the registry, exactly what §3 rejects by name.

### 3.1 Resolution and override law

A target names a logical vocabulary key such as `build:cargo`,
`package:agent-plugin` or
`deploy:vibe-bin`. Provider selection is deterministic:

1. an exact `provider` pin on the target wins;
2. otherwise the host-owned `[mechanisms]` route wins;
3. otherwise the shipped builtin default wins;
4. otherwise resolution fails and lists the installed candidates.

Logical role keys are capabilities, not package/artifact identities. Exact
provider pins and every generated lock/state row remain group-qualified.
Installing a dependency never lets it seize a logical key: there is no implicit
short-name discovery, “highest version wins” or filesystem-order fallback. An
exact provider pin is also the recovery path when a host override is broken.

```toml
[mechanisms]
"build:cargo" = "org.example/build-tools#cargo-v2"
"deploy:vibe-bin" = "org.example/installers#my-bin-layout"
```

`vibe extensions`, `vibe extensions --json` and the lifecycle narration show
the logical key, selected provider, displaced default, provider version/hash,
protocol, config schema and handler kind. A dry plan reports the same routing.

### 3.2 Provider protocol

All provider wire types are JTD-first and versioned. The lifecycle envelope is
retained; mechanism requests add a tagged operation:

- `plan`: validate provider config; resolve declared inputs and outputs; report
  commands and external effects without applying them;
- `fingerprint`: return the provider/toolchain portion of the freshness digest;
- `apply`: perform the declared transformation or reconciliation;
- `verify`: independently prove the output artifact or destination state;
- `remove`: deploy providers only, and only for state named by a receipt;
- `recover`: deploy providers only, for an interrupted operation named by an
  intent journal.

The engine, not the provider, owns ordering, scratch paths, artifact identities,
state persistence, locks, narration, timing and redaction. A provider may add
typed evidence but cannot mint an unscoped output path or silently invent a
second lifecycle.

Provider descriptors declare supported artifact kinds/platforms, effect class
(`workspace | user | remote | system`), network use, privilege need,
reversibility and operation support. `plan` is mandatory for deploy providers.
An installer that cannot roll back says so before apply; VibeVM never implies a
transaction it cannot provide.

Inline shell remains forbidden. Script, binary and native handlers receive
structured requests. Agent/LLM assistance may advise `plan` or `verify` under
the paid-feature policy, but a deploy side effect is applied by a deterministic
provider that can emit a receipt.

An arbitrary third-party provider receives no secret bytes from VibeVM and
authenticates through its own sanctioned client/config. A VibeVM builtin may
read a VibeVM-sanctioned source and attach the secret only at VibeVM's owned TLS
transport boundary. Naming a secret source is never permission to place its
contents in a provider argv, environment, request envelope, log or receipt.

## 4. Artifact graph and registry

The manifest declares desired producers; the run carries actual artifacts.
Build and package targets form a DAG over stable artifact IDs.

Proposed surface (exact serde spelling remains an authoritative-spec decision):

```toml
[[artifacts.build]]
id = "vibe-helper"
mechanism = "build:cargo"
workdir = "."
inputs = ["Cargo.toml", "Cargo.lock", "crates/vibe-helper/**"]
outputs = [
  { id = "vibe-helper.exe", kind = "executable",
    select = { package = "vibe-helper", bin = "vibe-helper" } }
]
config = { profile = "release", locked = true }

[[artifacts.package]]
id = "vibe-helper-windows"
mechanism = "package:windows-zip"
inputs = ["vibe-helper.exe"]
outputs = [{ id = "vibe-helper.zip", kind = "archive" }]
config = { layout = "distribution/windows" }
```

An `ArtifactRecord` contains at minimum:

- stable artifact ID and kind;
- file or directory shape, media type and platform triple;
- absolute runtime path plus a safe project/slot/store-relative identity;
- SHA-256 for a file or canonical tree digest for a directory;
- producing target, mechanism key and exact provider key/version/hash;
- input/config/toolchain fingerprint and creation time;
- verification status and evidence summary.

Artifacts carry no credentials. Later phases consume records, never guess a
Cargo filename, rescan a random directory or communicate through an undocumented
path. A directory is a first-class artifact; it is not implicitly zipped.

### 4.1 Freshness

The engine hashes target config, exact provider identity, declared inputs,
dependency artifact digests and the provider fingerprint. A fresh result is
accepted only if the recorded outputs still exist and independently hash to the
recorded digests.

Each mechanism declares `freshness = "engine" | "provider"`. Engine freshness
is legal only when the complete input set is closed and hashable. A
provider-fresh target is always invoked far enough to probe its own state; the
engine fingerprint may force work but may not suppress the provider. Cargo is
provider-fresh because `build.rs`, path dependencies and toolchain inputs make a
second, allegedly complete Vibe-side source model unsound. Provider changes
invalidate the target even when its logical mechanism name did not change.

`[[binary]]` remains compatible and lowers into a Cargo build target. Existing
`vibe bin` direct verbs remain usable. The lowering stops being the architecture:
Cargo is the first provider of the general graph.

## 5. Cargo commissioning backend

The Cargo provider:

1. uses `cargo metadata` to resolve the selected workspace/package/target;
2. executes an argv, never a shell string;
3. uses `cargo build --message-format=json-render-diagnostics` and selects the
   artifact from structured compiler-artifact messages, never from a guessed
   `target/release/<name>` path;
4. supports structured config for manifest path, package, target kind/name,
   profile, target triple, features, `locked`, `offline` and `frozen`;
5. includes Cargo/Rust toolchain identity and target config in its evidence,
   then delegates the authoritative freshness probe to Cargo rather than
   skipping Cargo from a necessarily incomplete Vibe-side source census;
6. verifies the chosen output and records its digest;
7. lets Cargo own its internal incremental compilation while VibeVM owns graph
   ordering, visibility, freshness evidence and artifact hand-off.

No language auto-detection is introduced. A stack preset or explicit target
selects Cargo. Maven/CMake/npm providers implement the same protocol instead of
adding new lifecycle semantics. A host that dislikes the built-in Cargo
provider routes `build:cargo` to another installed provider.

## 6. Packaging targets

### 6.1 Fully static skill

`package:static-skill` produces exactly one UTF-8 `SKILL.md` file. It validates
Agent Skills frontmatter and aligns directory/name identity. A multi-file source
is static-buildable only through explicit `vibe:include` directives in
`SKILL.md`; every directive names one declared textual resource and is replaced
deterministically with visible origin/hash framing. Every declared extra
resource must be consumed exactly once or the build refuses.

Static mode rejects executable scripts, shebang-bearing program files, binary
assets, unsafe traversal and unresolved sibling references. It never claims
that a multi-file skill became static while silently dropping resources. Exact
input/output digests and origin framing are required; a decompiler is not. A
normal directory skill remains a separate package kind.

At deployment the file is placed as `<client-skill-root>/<name>/SKILL.md`; the
single-file distributable and the installed directory shape are not confused.

### 6.2 Agent Plugins 1.0

`package:agent-plugin` produces a directory, because Agent Plugins 1.0 defines a
directory—not zip/tar—as the package unit. It contains root `plugin.json`, fixed
`skills/<name>/SKILL.md`, optional `mcp.json`, and only valid reverse-domain
client-extension directories. It enforces containment across symlinks,
junctions and reparse points, validates the published 1.0.0 schemas locally and
records a canonical directory digest.

Portable v1 components are Agent Skills and MCP servers only. Commands, hooks,
agents and LSPs are client projections, not invented portable fields.
`${PLUGIN_ROOT}` and `${PLUGIN_DATA}` keep their specified single-pass meaning;
visible `env`/headers are never a credential mechanism.

The canonical Agent Plugin and a client-native projection are distinct package
artifacts. Client adaptation happens in `package`, where it is reproducible and
verifiable; `deploy` only installs the selected projection. No adapter silently
drops an unsupported component: it either emits an explicit supported subset
requested by the manifest or fails with a capability report.

### 6.3 Client projection and local installation matrix

The commissioning clients are versioned adapters, not hard-coded paths spread
through the CLI:

| Client | Skill projection | Plugin projection/install posture |
|---|---|---|
| Claude Code | `~/.claude/skills/<name>/SKILL.md` for a user skill | Produce Claude-native `.claude-plugin/plugin.json`; install through a VibeVM-managed local marketplace and the Claude CLI |
| Codex | preferred shared root `~/.agents/skills/<name>/SKILL.md` (legacy `$CODEX_HOME/skills` remains readable but is not the new default) | Produce `.codex-plugin/plugin.json` plus a VibeVM-managed marketplace entry; install through `codex plugin add` |
| OpenCode | `~/.config/opencode/skills/<name>/SKILL.md`; it also discovers `~/.agents/skills` and `~/.claude/skills` | Produce explicit skill and MCP/config fragments; merge through the OpenCode adapter, because OpenCode's in-process plugin format is a different TypeScript/npm API |

Client CLIs are preferred for their private installation state. Filesystem
projection is used only where the client documents it as the public interface.
Config updates are parse/merge/atomic-write operations with before/after hashes,
never string append. All physical paths are de-duplicated before apply; selecting
Codex and OpenCode must not create competing owners of the same shared file.

Adapters expose `probe`, `plan`, `apply`, `verify`, `remove` and a supported
component matrix. A client version outside the tested range fails with a useful
plan unless the adapter declares forward compatibility. Current commissioning
evidence was measured on Claude Code 2.1.220, Codex CLI 0.148.0 and OpenCode
1.17.14; paths and commands are adapter data and must be re-probed in tests.

Existing same-name skill directories are not implicit authority. Before the
general deploy receipt lands, automatic project projection uses a strict
JTD-first receipt under `.vibe/`: provider-qualified binding, target agent/path,
and exact owned file hashes. An unowned target refuses; updates diff only
recorded files; removal leaves every unrecorded neighbor. Manifest names/paths
are single-component/relative and every source/target ancestor is checked for
containment and links immediately before mutation. Desired bindings are
reconciled against the prior owned set, so agent shrink, rename or declaration
removal cannot leave an orphan.

## 7. Deploy targets and profiles

Profiles are named destination selections, not Maven-style arbitrary overlays
of the whole manifest. This keeps “local versus production” powerful without
making the effective build unknowable.

```toml
[[deploy.target]]
id = "local-helper"
artifact = "vibe-helper.exe"
mechanism = "deploy:vibe-bin"
config = { command = "vibe-helper" }

[[deploy.target]]
id = "team-plugin"
artifact = "review-plugin.codex"
mechanism = "deploy:codex-plugin"
config = { scope = "user" }

[[deploy.target]]
id = "production"
artifact = "vibe-helper.zip"
mechanism = "deploy:acme-server"
provider = "org.acme/deploy#server"
config = { service = "helper", environment = "production" }

[deploy.profiles.local]
targets = ["local-helper", "team-plugin"]

[deploy.profiles.production]
targets = ["production"]
```

Commands:

```text
vibe deploy --profile local
vibe deploy --profile production
vibe deploy --profile local --plan
vibe undeploy --profile local
vibe deployments [--json]
```

`vibe deploy` without `--profile` is legal only when the manifest names an
explicit default or defines exactly one profile. Environment variables and the
presence of secrets never choose a profile. A plan does not read tokens, call
an LLM, download, build or mutate any destination; it reports preceding stale
targets as planned work.

Profile targets are ordered as authored and may declare dependencies. A cycle,
unknown target, duplicate physical destination or incompatible artifact kind is
a validate error. Two profiles can reuse a target without duplicating its
definition.

### 7.1 `deploy:vibe-bin`

The VibeVM-specific local-tool provider stores immutable payloads under a
content-addressed `~/.vibe` store and writes a version-free launcher in
`~/.vibe/bin`. The launcher resolves only its active deployment receipt; it does
not embed a package version or copy a mutable binary into PATH.

This is a different launcher genre from PROP-025's project-pinned `vibe bin`
shim. Their bodies carry an exact VibeVM marker naming the genre and owner.
They never fall through into one another. A name already owned by the other
genre—or by an unmarked user file—is a hard collision that names both origins
and asks the target to choose another command alias. Each GC removes only its
own marked genre.

Only an explicit executable artifact and target may use this provider. An
ordinary application that wants MSI, dpkg, Homebrew, a custom prefix or another
installer chooses a different deploy mechanism. Merely producing an executable
does not grant installation into `~/.vibe/bin`.

### 7.2 Receipts, ownership and inverse operations

Every applied target writes a schema-versioned receipt in the user VibeVM state
home. A receipt records:

- project/package identity, profile, target and generation;
- artifact digest and exact mechanism provider identity;
- desired-config digest and destination scope;
- every owned path/resource and its post-apply digest;
- provider evidence, reversibility and any prior-state handle;
- timestamps and final status, never secrets.

Receipt finalisation is last, but receipt-last alone cannot cover a crash after
the first external write. Before apply, VibeVM atomically writes a durable
intent journal containing the plan hash, prior receipt generation, every
planned resource and its desired digest. Apply checkpoints completed operations
without storing secrets. After independent verify, the finalized receipt is
written and the intent is retired. Apply uses a per-destination lock and staging
where the destination supports atomic replacement.

On restart, an intent without a matching final receipt enters `recover`: if all
observed resources match either the prior or desired digest, the idempotent
provider rolls forward and finalizes; a third digest means concurrent/user
mutation, so recovery refuses and names the exact resources. A receipt plus its
still-present matching intent is a benign crash after finalization: retire the
intent. A collision with state owned by another deployment is an error unless
both intentionally share an identical content-addressed payload and the
provider supports reference ownership.

`undeploy` removes only receipt-owned state and refuses to erase a path changed
after deployment without an explicit force/recovery decision. A failed multi-
target deploy is a recorded saga: already-applied reversible targets are rolled
back in reverse order; irreversible results remain visible as partial, never
reported as success.

## 8. LLM policy at this boundary

All commissioning build, package and deploy mechanisms have complete
algorithmic implementations. Credentials alone activate nothing. An enhancing
feature declares:

- `off`: algorithmic baseline only;
- `assist`: use the configured provider within a declared budget, fall back to
  the algorithmic result on unavailability/failure;
- `required`: refuse when the explicitly requested paid enhancement cannot run.

The plan names feature, provider/model, reason, cache posture and input/output
token budgets. Dry-run never reads a token. Receipts record usage counters but
not prompts, responses or secrets. An LLM may advise package metadata or verify
quality; it never becomes the only way to build Cargo, validate Agent Plugins,
install a skill or undo a deployment.

## 9. VibeVM OS compatibility horizon

VibeVM OS is a future desired-state system manager built on this same plane,
not a reason to mutate the host during ordinary package install today.

Future artifact/resource sources can include:

- locally built output;
- HTTPS/registry/OCI prebuilt payload pinned by digest and optionally signature;
- a system package coordinate serviced by apt/dnf/pacman/winget/Homebrew or a
  user provider;
- configuration, service, account and filesystem resources.

The package need not contain the binary. It contains a declarative source or
resource request. Download/cache/signature verification belongs to an acquire
provider; OS package-manager invocation belongs to a system-scope deploy
provider. Both return the same provenance/receipt evidence as today's adapters.

The following invariants are imposed now so that evolution remains possible:

1. IDs are stable and provider-qualified; paths are not identity.
2. Desired state, produced artifact and applied deployment are three records.
3. Every external mutation has an effect class, plan and receipt.
4. User/system/remote scopes are explicit; privilege is never inferred.
5. Provider routing is host-owned and builtins are replaceable.
6. Content hashes and signatures can be added without changing phase semantics.
7. Dependency materialisation stays separate from host-system reconciliation.
8. Partial failure is durable state, enabling later repair/reconcile.

A future privileged broker, dependency solver, capability/conflict model,
service manager and whole-system transaction coordinator are intentionally
deferred. The present provider protocol and receipts are their compatibility
seam.

## 10. Implementation sequence and proof obligations

The campaign should land this design in small atoms:

1. JTD artifact/deployment records and pure target/profile validation.
2. `[[mechanism]]` registry, builtin rows, host routes and exact provider pins.
3. Repair existing skill projection containment/ownership with a strict
   JTD-first project receipt before binding it automatically to `package`.
4. Cargo build provider plus `[[binary]]` compatibility lowering.
5. Static-skill and Agent Plugins 1.0 package providers.
6. Client projection providers for Claude Code, Codex and OpenCode.
7. Deploy planner/intents/receipts/locks, then `vibe-bin` and client install
   providers.
8. A plugin-supplied deploy provider fixture that replaces a builtin route.
9. Windows zip packaging through the same registry, not a bespoke R8 branch.

Each atom needs a mutation that turns its green test red. End-to-end gates:

- build a Rust fixture and obtain the executable only from Cargo JSON messages;
- package one static skill file and one schema-valid Agent Plugin directory;
- plan and locally deploy the skill/plugin to isolated fake homes for all three
  clients, verify intent/receipts, crash-recover, then undeploy without touching
  an unowned file;
- deploy a VibeVM tool through an isolated `~/.vibe/bin`, run its version-free
  launcher, update it and roll it back;
- route `build:cargo` or `deploy:vibe-bin` to a fixture provider and prove the
  builtin did not run;
- run `--plan` with sentinel token files and prove no credential read/network
  call/write;
- preserve the two PROP-054 owner scenarios and the full repository panel.

## 11. Research evidence and deliberate non-assumptions

The portable plugin contract was read centrally from
`agentplugins/agent-plugins-spec` commit
`ff8ab5e392cc87bd88d87c060815a87490e51003`: published 1.0.0 spec, schemas,
licenses and the 1.1 draft diff. Agent Plugins installation is explicitly
client-owned; the portable unit is a directory and only skills/MCP are portable.

Client installation evidence was taken from current primary documentation and
local CLI help on 2026-08-26. No private cache path is promoted to a contract.
Where a client exposes a CLI, the adapter drives that CLI. Where it documents a
filesystem/config surface, the adapter owns an atomic projection with a receipt.

The following remain post-campaign design decisions, not blockers for the
commissioning cut: signature policy and trust roots, remote server protocol,
system privilege broker, system-package-manager mapping, SBOM format, global
dependency solving and cross-machine receipt synchronization.

## 12. Atom-1 spelling freeze — decision record (central, 2026-08-29)

§4 says "exact serde spelling remains an authoritative-spec decision"; this
section IS that decision for sequence item 1, split into two disjoint slices
(A1 manifest grammar + pure validation; A2 record wire shapes). Everything
here is the frozen minimum; widening is a later recorded decision.

**Amended at A1 acceptance (central, 2026-08-29).** The first spelling of
this section was authored without re-reading the LANDED `2a3f3b44` grammar
(R8.2A) and contradicted three of its recorded decisions. Wrong
current-state facts are the most expensive class of plan bug, and this was
one; the section below is the repaired freeze, each delta named against the
incumbent it amends.

**Decision — identifiers and vocabulary.** The mechanism plane keeps its ONE
incumbent grammar: target ids, artifact ids, profile names, mechanism-key
tails, mechanism declaration names and provider-pin ids all obey R8.2A's
`is_portable_token` (nonempty lowercase alphanumerics, `-`, `.`). The first
freeze's backend-id grammar is WITHDRAWN here: it was the compiler plane's
authority, and importing it would have been the second grammar in this plane
— the exact drift the argument claimed to prevent — while splitting one
mechanism family across two grammars (a key tail its own declaration name
could not spell). `ArtifactKind` becomes the closed lowercase set
`executable | archive | file | directory | skill | agent-plugin`. This
SUPERSEDES R8.2A's recorded open-kind decision, and the trigger is named:
its why («future ecosystems need no phase-law change») is answered by growth
being a one-line spec amendment, while §4's registry law — records validated
against a vocabulary — became implementable only if the vocabulary is
closed; an acquire-role provider reopens the set as a recorded decision.
Mechanism-key prefix/table-family agreement stands (`build:` / `package:` /
`deploy:`). An exact `provider` pin keeps R8.2A's `ProviderPin` spelling.

**Decision — A1 manifest grammar (vibe-core::manifest, serde over TOML,
`deny_unknown_fields` on every structural table).**
Inputs keep R8.2A's RECORDED shape — the strict tagged one-of
`{ path = "…" }` | `{ artifact = "…" }`, in BOTH families: path-versus-id is
never guessed from text, a package may carry raw files beside consumed
artifacts, and build→build chaining stays expressible under the incumbent
phase-forward law (package may consume build outputs; build never consumes
package). The first freeze's bare strings are WITHDRAWN — they re-introduced
the exact ambiguity `2a3f3b44` resolved structurally, and its split
paths-for-build/ids-for-package semantics lost both mixed cases.
`provider: Option<ProviderPin>` stays on BOTH artifact families and on
`[[deploy.target]]` — §3.1's resolution law rule 1 names target pins for
every mechanism target, and the first freeze's omission was an error, not a
decision. What A1 ADDS to the incumbent: optional `workdir` on build targets
only (default `"."`, forward-slashed, no `..` escape, the declarant-path
law); `select` (an opaque provider table, the config newtype) on output
rows; the closed `ArtifactKind` above; typed refusal enums (thiserror)
replacing String errors, each message naming table, field and bounded value;
`[deploy] default_profile`; and the named-cycle refusals. Pure validation
(validate-phase, no filesystem, no provider): unique target ids and globally
unique output ids (the incumbent's stronger cross-family law stands); every
`{ artifact }` input and deploy `artifact` resolves to exactly one declared
output id under phase-forward; mechanism-prefix/family agreement; profile
members exist; `default_profile` names a declared profile; the reference
graph and deploy `depends_on` are acyclic with the refusal naming the closed
id sequence; bare-`deploy` legality stays a law on the value, decided by the
CLI later. "Duplicate physical destination" and kind/mechanism compatibility
need provider knowledge and belong to the mechanism atom, not A1.

**Decision — A2 record wire shapes (schemas/ + vibe-wire, JTD-first,
mirrored end to end on the existing `lifecycle_state` format's registration,
codegen, behaviour-cell and corpus pattern).** Three new formats:
`artifact_record` (per §4: schema, id, kind, shape `file|directory`, optional
media_type and platform triple, absolute path plus `{root: project|slot|store,
path}` relative identity, digest `{algorithm: sha256|sha256-tree/1, value}`,
producer `{target, mechanism, provider {key, version?, content_hash?}}`,
three named freshness digests `{inputs?, config?, toolchain?}` — each
optional because a provider-fresh target may honestly lack one — created_at,
verification `{status: unverified|verified|failed, evidence?}`);
`deploy_intent` (§7.2: plan hash, target identity, prior generation?,
planned resources `[{resource, desired_digest, prior_digest?}]`, started_at);
`deploy_receipt` (§7.2's exact list: identity, profile, target, generation,
artifact digest, exact provider identity, desired-config digest, destination
scope `workspace|user|remote|system`, owned resources `[{resource,
post_digest}]`, evidence?, reversibility, prior-state handle?, timestamps,
final status — and NEVER a secret-bearing member). Scalar laws ride the
conversion exactly as compiler-IR members do (non-blank, digest hex forms,
forward-slashed relative paths); every record is strict
(`deny_unknown_fields`) and corpus-pinned valid+invalid.

**Considered and rejected.** One combined "artifacts" JTD format — three
lifecycles, three writers, one schema epoch would couple them for zero
sharing. Free-form artifact kinds — the §4 registry law needs a closed
vocabulary to validate against. Putting the grammar beside the records in
vibe-wire — the manifest is vibe-core's one grammar home, and `[[extension]]`
is the precedent. Validating destination collisions in A1 — needs provider
descriptors that do not exist yet.

**When to revisit.** When the mechanism atom lands provider descriptors, the
deferred compatibility checks join validation; when acquire-role providers
land, `ArtifactKind` and the digest algorithms are re-opened as recorded
decisions.
