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

**Ratified at R8-MECHANISM acceptance (central, 2026-08-30).** Five rulings
the landing surfaced, each pinned:

1. **One shared disable list governs both planes.** The host's single
   `[extensions].disable` is the only disable surface, so a key naming a
   mechanism row of the same world counts as known to the extension
   collector — the narrowest widening that makes §3.0.3's sentence true in
   practice; `UnknownDisable` still fires for a key in neither plane, and a
   builtin disable refuses as the reserved-control twin. A provider sharing
   one id across both planes silences both facets with one line, which is
   the coherent reading of "one control list".
2. **Constructor-site updates outside the atom's perimeter are
   gate-forced, not scope creep**: a pub struct's new field breaks every
   literal, and the sixteen one-line sites (four production, twelve
   test/doctest) are the whole cost — an empty vector is the historical
   world exactly.
3. **The builtin descriptors' freshness values are read out of this
   document, not invented**: cargo → provider (§4.1's own sentence),
   static-skill and agent-plugin → engine (closed hashable input sets,
   §§6.1–6.2), vibe-bin → provider (deploy reconciles state no engine
   census can hash). The four `config_schema` spellings are engine-owned
   identities under `schemas/mechanism/`; R8-CARGO materialises the files
   when something first reads them (REVIEW marker at the table).
4. **`resolve_mechanism` takes `&MechanismKey`**, not a `(role, name)`
   pair — an unvalidated pair would be a second spelling of the one key
   grammar, which §3.0's own ruling forbids.
5. **A selection cannot displace itself**: the displaced default is
   carried exactly when a replacement replaced something — a builtin
   selected by its own pin or route displaces nothing (reviewer mutation
   proved the filter unpinned; the pin now holds it as law).

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

### 5.0 Execution seam and the Cargo atom's staging — decision record (central, 2026-08-30)

**Decision.** R8-CARGO brings the FIRST executing mechanism, staged
honestly:

1. **Execution home.** The mechanism execution seam lives in
   `vibe-lifecycle`, beside the phase machine that owns ordering,
   narration and state (§3.2's engine-owns list). Builtin providers are
   IN-PROCESS implementations of one crate-internal trait mirroring the
   §3.2 operations a build provider needs (`plan`, `fingerprint`,
   `apply`, `verify`); the out-of-process provider protocol transport
   (script/binary/native envelopes) is a LATER atom — at this atom a
   resolved non-builtin mechanism at the build phase refuses typed,
   naming the transport as not-yet-landed rather than pretending.
2. **Wiring.** The build phase walks the landed `[[artifacts.build]]`
   targets in the A1 DAG order; per target the logical key comes from
   `target.mechanism`, the exact pin from the target's own `provider`
   member, the routes from the host manifest, and selection is
   R8-MECHANISM's `resolve_mechanism` — one law, no second resolver.
3. **Cargo's message reader is a FOREIGN-format reader.** The
   `--message-format=json-render-diagnostics` stream is Cargo's wire,
   not ours: it is parsed with a minimal lenient serde shape carrying
   exactly the members the laws read (`reason`, `package_id`,
   `target.{name,kind}`, `executable`, `fresh`) — a lawful handwritten
   derive under the wire-derive ratchet, named in the landing commit
   per that gate's own recipe. No schema is authored for another tool's
   format.
4. **Records.** One `artifact_record` (the A2 exchange) per produced
   output, written by the ENGINE to the engine-owned state home
   `.vibe/state/artifacts/<output-id>.json` — a provider cannot mint an
   output path (§3.2), and the record's byte-count/digest members follow
   the A2 laws exactly.
5. **Freshness at this atom is provider-fresh, as §4.1 rules for
   Cargo**: the adapter always invokes Cargo and lets Cargo's own
   incremental machinery answer; the `fingerprint` operation contributes
   toolchain identity (`cargo -Vv`, `rustc -V`) into the evidence, and
   the record notes Cargo's own `fresh` verdict. No Vibe-side source
   census for Cargo targets.
6. **`[[binary]]` lowers by projection, not by branch**: one pure
   function projects a legacy `[[binary]]` entry into the equivalent
   build target (mechanism `build:cargo`, one executable output selected
   by package/bin), so the graph executor sees ONE target shape;
   `vibe bin` direct verbs stay untouched.
7. **Tests.** The selection/refusal/message laws are unit-proven over
   recorded Cargo JSON fixtures; the acceptance's "Rust fixture built"
   is ONE real end-to-end test compiling a dependency-free fixture crate
   (offline-safe, temp target dir) and asserting the executable is taken
   only from the compiler-artifact message — the real-build cost is
   accepted for exactly one test per suite.

**Considered and rejected.** Executing builtins through a synthetic
in-process copy of the out-of-process envelope — serialization theater
with no second process to justify it; the trait mirrors the operations,
the envelope arrives with the transport atom. A public schema for
Cargo's message stream — freezing another tool's wire as ours. Records
under the provider's chosen paths — §3.2 forbids it by name.

**Ratified at R8-CARGO acceptance (central, 2026-08-30).** Six rulings the
landing surfaced, each pinned:

1. **The Cargo `config` member names are snake_case and strict**, with four
   engine-owned members refusing BY NAME (`manifest-path` spellings that
   would collide with the engine's own argv authority); the engine appends
   `--target-dir <project_root>/target` itself and scrubs
   `CARGO_TARGET_DIR` from the child environment — the flag outranks the
   variable in Cargo's own precedence, and both defenses ride together.
2. **`vibe_safefs`'s multi-name refusal is right for its domain and wrong
   for Cargo artifacts** (release binaries carry a hard-linked second name
   under `deps/`), so `verify` streams the bytes itself after containment
   — filed as B-120 for the safefs owner to decide whether a
   build-artifact read primitive belongs beside the publication one.
3. **The verify refusals are laws, not incidents**: review proved the
   whole containment check deletable with every suite green — the
   in-project-but-outside-target fixture is what isolates the build-root
   law from the project-relative step, and the three refusal pins
   (containment, link, absence) now hold the seam.
4. **The projection's engine literal enters through a crate-internal
   validated-parts constructor** (`MechanismKey::from_validated_parts`,
   debug-asserted against the one token grammar) — no excused `expect`,
   no Result on an impossible parse, conform back at exactly its 27.
5. **The orchestrator call site is the NEXT atom's wiring**: the complete
   executor (`vibe_lifecycle::execute_build_targets`) landed with no stub,
   and `run_phases` learns to call it when R8-PACKAGE wires build and
   package together — one wiring, two consumers.
6. **`vibe-lifecycle` enters the wire-derive baseline at 1** for exactly
   the lenient Cargo message reader — a foreign-format file named per the
   ratchet's own recipe.

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

### 6.0 Package staging and the phase wiring — decision record (central, 2026-08-30)

**Decision.** R8-PACKAGE lands the two builtin package providers and the
one phase wiring:

1. **The provider family extends the R8-CARGO seam**: a crate-internal
   `PackageProvider` trait beside `BuildProvider` in `vibe-lifecycle`,
   same operations (`plan`/`fingerprint`/`apply`/`verify`), same
   engine-owns list, same in-process staging (out-of-process transport
   stays a later atom; a non-builtin selection refuses by its name).
   Selection is the same `resolve_mechanism`, role `package`.
2. **`run_phases` learns BOTH executors in one wiring** (the R8-CARGO
   deferral discharged): the build phase calls
   `execute_build_targets`, the package phase calls the new
   `execute_package_targets`, both inside `vibe-orchestrator`'s
   existing phase walk — no phase reordering, no new commands. A
   `[[artifacts.package]]` input names a build output's id; the package
   executor reads the A2 record the build executor wrote (engine-owned
   state, never a guessed path) and refuses a missing or stale-digest
   input by name.
3. **`package:static-skill` is engine-fresh per §4.1** — the complete
   input set is closed and hashable (the declared `SKILL.md` plus every
   `vibe:include`-named textual resource); §6.1's laws verbatim: one
   UTF-8 file out, frontmatter validated, every declared resource
   consumed exactly once, executable/binary/traversal/sibling refusals,
   origin/hash framing on every inclusion, exact input/output digests
   recorded.
4. **`package:agent-plugin` is engine-fresh likewise** — §6.2 verbatim:
   a DIRECTORY distributable (plugin.json, `skills/<name>/SKILL.md`,
   optional mcp.json, reverse-domain client-extension dirs only),
   containment across links/junctions/reparse points, local schema
   validation against the published 1.0.0 shapes, one canonical
   directory digest recorded in the A2 record (`kind = directory`).
5. **Client projections (§6.3) are OUT of this atom** — they are
   `package`-phase adapters over the canonical plugin and land with the
   deploy lane, where their install postures live; nothing here touches
   a user home.
6. **Distributables land under the engine-owned package root**
   `target/vibe-package/<target-id>/…` — same containment law as the
   build root, same verify pins, records beside the build records in
   `.vibe/state/artifacts/`.

**Considered and rejected.** A second provider trait shape for package —
one seam, two roles. Wiring package before build in the same atom as a
"while we're here" reorder — the phase line is frozen (§2). Client
projection inside `package:agent-plugin` — §6.2 keeps the canonical
plugin and the projections distinct artifacts.

**Ratified at R8-PACKAGE acceptance (central, 2026-08-30).** Six rulings
the landing surfaced, each pinned:

1. **The §6.0.2 call site as first frozen was unimplementable**: the
   pre-dispatch phase walk runs before the prepared world exists, and
   placing the executors there inverted the generate edge of §2's primary
   law. REJECTED in review; the landed shape is the correct one — the two
   mechanism fences fire inside dispatch's own phase-ordered contribution
   walk, BEFORE their own phase's contributions (the verify boundary's
   position and reason), straddling the verify gate as the phase line
   orders; a partial epoch (a prerequisite install) arms nothing by
   parameter. Hermetic ordering pins hold both edges, red under the
   executor-before-the-walk mutation.
2. **`vibe:include` is a WHOLE-LINE directive** — `<!-- vibe:include
   NAME -->` alone on its line; any other spelling that mentions the
   token refuses rather than surviving as text.
3. **A plugin target carries an obligatory `place` map**: every declared
   input placed exactly once, destination inside a reverse-domain
   client-extension directory — the member §6.2 needed for a consumed
   artifact to enter the directory at all; grammar promotion into
   vibe-core is the deploy lane's candidate.
4. **The plane reaches the orchestrator through vibe-lifecycle's R4.0
   compatibility door** (its dependency-exactness fence forbids a direct
   kernel edge), and `RitualPlan`'s structural fence moved 8 → 9 fields
   with the reason inline.
5. **Reviewer pins at the platform edges**: the junction containment law
   proven with a REAL Windows junction (the one platform §6.2's reparse
   sentence exists for), and the binary-asset refusal pinned at its
   sharpest edge — non-UTF-8 bytes with no NUL and no shebang, invisible
   to every sibling law under a lossy read.
6. **The `[[binary]]` runtime projection call site remains unwired** —
   the projection function is landed and tested; the call site that turns
   a legacy entry into a live build target belongs to the deploy-lane
   wiring atom, named here so it cannot be forgotten.

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

### 6.3.0 R8-CLIENTS staging — decision record (central, 2026-08-30)

**Decision.** R8-CLIENTS is five serial atoms — freeze, foundation,
package projections, deploy adapters, three-client gate — and lands the
commissioning matrix without turning any client into a special CLI branch:

1. **Scope.** In: user-scope skill placement for Claude Code, Codex and
   OpenCode; three client-native projections of one canonical Agent Plugin;
   user-scope plugin installation through the general deploy transaction; and
   the isolated §10 three-client gate. Out: project-scope automatic skill
   projection (R8.1 already owns it), live operator-home mutation, public
   marketplace publication, OpenCode's unrelated npm/TypeScript plugin API,
   remote/system deployment and the still-unlanded foreign-provider transport.
2. **Nine ordinary builtin rows extend the ONE mechanism registry.** Package:
   `package:claude-plugin` → `org.vibevm/vibe#claude-plugin-projection`,
   `package:codex-plugin` → `#codex-plugin-projection`, and
   `package:opencode-plugin` → `#opencode-plugin-projection`. Deploy:
   `deploy:{claude,codex,opencode}-skill` → the same-named `#*-skill` rows and
   `deploy:{claude,codex,opencode}-plugin` → `#claude-plugin`,
   `#codex-plugin`, `#opencode-plugin`. Projection rows are engine-fresh;
   destination rows are provider-fresh. The first three rows deliberately
   prove that provider id and logical name are separate fields.
3. **Projection is package work.** Each client-plugin package target consumes
   exactly one recorded `agent-plugin` directory artifact and emits one
   recorded directory projection. Its strict config requires a nonempty unique
   `components` subset of `skills|mcp`; only those explicitly requested
   portable-v1 components may be emitted. A source component the selected
   adapter cannot represent refuses with a capability report; no file is
   silently dropped. Reverse-domain client-extension directories remain
   unrequested unless a later client-specific ruling admits one.
4. **The three projection shapes are exact.** Claude moves the canonical
   manifest to `.claude-plugin/plugin.json`, Codex to
   `.codex-plugin/plugin.json`; both retain selected `skills/` and map selected
   `mcp.json` to `.mcp.json`. The Codex shape is the current OpenAI Docs plugin
   contract: `.codex-plugin/plugin.json`, optional `skills/`, optional
   `.mcp.json`, installed from a local marketplace. OpenCode emits selected
   `skills/` plus one strict `opencode.json` fragment containing only the named
   `mcp` entries its adapter will merge. Every projection preserves plugin
   name/version and records adapter epoch 1 in its fingerprint/evidence.
5. **Standalone skills remain artifacts, not plugins.** The three skill deploy
   providers accept only a file-shaped `skill` artifact plus strict
   `config={name="portable-token"}` and own exactly one entry file under
   `.claude/skills/<name>/SKILL.md`, `.agents/skills/<name>/SKILL.md`, or
   `.config/opencode/skills/<name>/SKILL.md`. They never own or remove an
   unrecorded neighbour and never route a Codex/OpenCode selection through the
   same shared physical path by convenience.
6. **Home and executable authority are injected.** `DeployExecution` carries
   the exact user home beside `settings_root`, plus explicit Claude/Codex/
   OpenCode executable paths. The CLI surface resolves them once; every lower
   cell and provider is forbidden from calling `dirs::home_dir`, reading
   `HOME`/`USERPROFILE`/`CODEX_HOME`/`CLAUDE_CONFIG_DIR`, searching `PATH`, or
   finding a real client. Tests pass temp homes and fake executables, making
   the operator's home unreachable by construction.
7. **VibeVM owns local marketplace bytes; clients own private install state.**
   Claude and Codex deploy providers materialise an immutable local marketplace
   below `settings_root/client-marketplaces/<client>/<target>/<artifact-digest>/`
   with the client's native marketplace manifest, then use the injected CLI.
   Claude's exact epoch-1 argv is `plugin marketplace add --scope user <root>`,
   `plugin install --scope user <plugin>@<marketplace>`, `plugin list --json`,
   and `plugin uninstall --scope user <plugin>@<marketplace>`; Codex uses
   `plugin marketplace add --json <root>`, `plugin add --json
   <plugin>@<marketplace>`, `plugin list --json`, and `plugin remove --json
   <plugin>@<marketplace>`. Live probes reconfirmed Claude Code 2.1.220 and
   Codex CLI 0.148.0. The provider fingerprints the parsed client version and
   refuses outside its tested minor line with remediation rather than guessing
   private state.
8. **OpenCode is the documented different adapter.** It does not install the
   Agent Plugin through `opencode plugin` (that command installs a different
   npm/TypeScript plugin genre). It publishes selected skill files and merges
   each projected MCP entry into
   `.config/opencode/opencode.json` under `mcp`, preserving every foreign key,
   through parse → merge → canonical encode → atomic replacement. Epoch 1 is
   tested against OpenCode 1.17.x; an incompatible version refuses before the
   first write.
9. **Owned identity and physical locking are separate.** `DeployPlan` gains
   engine-internal `lock_resources`; `DeployDescriptor` gains an explicit
   reference-ownership capability. A normal provider's lock resources equal
   its owned resources. An OpenCode config entry owns
   `home:.config/opencode/opencode.json#mcp/<name>` while locking the physical
   document `home:.config/opencode/opencode.json`; CLI-owned plugin state uses
   a logical plugin resource while locking that client's private plugin state.
   The exception is admitted only for a provider declaring reference ownership
   and is the deferred §7.0 ratification-3 capability arriving with its first
   honest user.
10. **Every selected plan is prepared before the first apply.** The engine
    compares all owned and lock resources through the shared Unicode-9
    physical path identity. Duplicate owned identity always refuses. Duplicate
    physical lock identity refuses unless every participant explicitly uses
    reference ownership and owns a distinct logical member of that shared
    document/state. Thus a Codex/OpenCode combination cannot reach apply while
    competing for one skill or config member; the per-destination locks also
    serialize separate deployments of one shared document.
11. **Plan and verify use read-only probes only.** Skill/OpenCode providers
    inspect the exact target/config bytes. Claude/Codex use only their local
    marketplace/list JSON probes; no provider plan reads a token, reaches the
    network, mutates a marketplace, or runs install. Apply is idempotent by
    probing each completed CLI step before repeating it, so the existing intent
    journal can recover. Epoch 1 refuses an in-place plugin update to different
    artifact bytes with `undeploy, then deploy` remediation rather than claim a
    rollback for private client state it cannot restore.
12. **R8.1 is reused as a lower vocabulary, never as a second receipt.**
    `vibe-agent-projection` owns client ids, pure path/config transformations
    and the shared safe-filesystem identity helpers; `vibe-lifecycle` owns
    mechanism protocols, intent/checkpoints/receipt/locks/saga; the
    orchestrator and CLI only inject authority and render. A user deployment
    writes no project package-skill receipt and automatic project projection
    writes no user deployment receipt.
13. **The commissioning proof touches no real client.** Fake Claude/Codex
    executables implement the exact argv/list contract inside temp homes;
    OpenCode uses a temp config with foreign neighbours. The gate packages one
    static skill and one canonical Agent Plugin, produces all three native
    projections, plans every target with write/network/token sentinels, deploys,
    independently verifies, interrupts one apply and recovers it, then
    undeploys. Every foreign neighbour remains byte-identical; target/physical
    alias and config-entry mutations turn their named tests red.

**Considered and rejected.** Adapting a canonical Agent Plugin during deploy —
that makes installation non-reproducible and contradicts §6.2. Calling
`opencode plugin` — a different extension format, not an Agent Plugin adapter.
Letting each provider resolve the operator home or client binary — untestable
ambient authority. Treating a shared JSON file as one deployment's owned file —
it prevents unrelated plugins from coexisting; treating its entries as
independent without one physical lock loses updates. Claiming private-CLI
updates are reversible — the client, not VibeVM, owns their prior state.

**Ratified at R8-CLIENTS-FOUNDATION acceptance (central, 2026-08-30).**
Seven rulings the landing surfaced, each pinned:

1. **Executable authority is a total value, not `Option<PathBuf>` and not a
   bare command word.** `ClientExecutable` is `Resolved { command,
   absolute_path } | Missing { command }`; the CLI surface alone reads
   PATH/PATHEXT and passes one total three-client value down. Missing clients do
   not veto an unrelated `deploy:vibe-bin` run. The sanctioned ambient read is
   one explicit `conform.toml` `env_roots` entry; removing it is red.
2. **The physical lock filename uses the same Unicode-9 identity as
   preplanning.** Case/NFC aliases produce one lock file across runs, while
   exact provider/receipt spellings remain unchanged. The read-only planner and
   prior-receipt collision gate call the same selected-set/identity laws as
   apply, so `--plan` cannot promise a deployment apply later refuses.
3. **Preplanning strengthens the old live saga scenario honestly.** Every
   artifact and every provider plan now succeeds before apply 0, so a
   plan-detectable second-target refusal leaves the first destination absent
   rather than exercising rollback. Saga rollback remains proved through
   apply-time faults at the hermetic seam; the owner scenario did not lose a
   failure class, it moved one refusal to the earlier transaction boundary.
4. **Reference ownership is admitted for plan/apply but not yet for inverse
   operations.** A receipt stores the logical owned member, not the physical
   lock resource, so undeploy and saga reversal refuse/retain a reference owner
   instead of locking the wrong identity. R8-CLIENTS-DEPLOY must land an
   engine-owned strict-serde durable lock sidecar before its first shipped
   reference-owning provider: persisted before the first external write,
   retained beside the receipt, read by recovery/undeploy, never added to the
   JTD owned-resource wire. No resource-string parser is permitted.
5. **The all-participants rule is genuinely n-ary.** An already-admitted pair
   of reference owners does not erase the shared-lock claim: a third
   participant still must declare the capability, and duplicate logical
   ownership remains unconditionally refused.
6. **PATH resolution follows executability, not mere file existence.** On Unix
   a non-executable earlier file cannot shadow a later real client; the pure
   resolver takes its executability predicate as test input, while the shipped
   predicate enforces file + execute bits. Windows keeps PATHEXT order and
   npm-style `.cmd` discovery.
7. **The 600-line floor produced responsibility seams, not cosmetic shards.**
   Builtin-table assertions, injected-authority assertions, the hermetic
   provider, preplan laws and saga failure handling each live in their own
   cell. Existing public values and vibe-bin outputs remain compatible; only
   the new explicit authority/preplan refusal surface was added.

**Ratified at R8-CLIENTS-PACKAGE acceptance (central, 2026-08-30).**
Seven implementation rulings the landing made executable:

1. **Artifact kind and physical shape answer different questions.** The
   canonical provider now records `kind=agent-plugin, shape=directory`, matching
   the frozen A2 corpus and its own contract. Package input provenance carries
   the record's typed kind; a workspace path or recorded plain directory cannot
   become a canonical plugin by resemblance, and a projection cannot feed a
   second projection.
2. **Three provider identities share one closed implementation.** The Claude,
   Codex and OpenCode rows dispatch to one provider parameterised by a closed
   client enum. Client choice never enters config, so provider id and logical
   mechanism name remain visibly separate without three copied adapters.
3. **`components` is a canonical set and capability checking is plan work.**
   The only config member is a nonempty unique subset of `skills|mcp`, stored in
   canonical order. A missing selected component or unrepresentable OpenCode
   member refuses during the read-only plan, before the engine prepares output.
4. **Projection reuses the canonical validator.** The recorded tree is
   revalidated through the Agent Plugin shape/manifest cells; that one parse
   returns name, version and the validated MCP map. Legal reverse-domain client
   extensions remain outside the component vocabulary, are not emitted, and
   are counted as withheld-by-contract in evidence rather than silently lost.
5. **The client shapes are byte-exact and OpenCode remains a different genre.**
   Claude/Codex move the full canonical manifest, retain selected skills and
   copy selected `mcp.json` to `.mcp.json`. OpenCode emits selected skills plus
   a deterministic `mcp` fragment: local argv/environment and remote
   URL/headers, with unsupported cross-transport members refusing. The two
   plugin placeholders remain unexpanded until deploy; no npm/TypeScript plugin
   field or command is invented.
6. **Adapter epoch 1 is durable freshness evidence.** The projection
   fingerprint binds client, adapter epoch, canonical tree digest, parsed
   name/version and canonical component set. All three outputs remain recorded
   plain directory artifacts, so the record kind keeps canonical input and
   client-native output distinct.
7. **Acceptance is mutation-backed on the integrated tree.** Nine worker REDs
   and three independent central REDs all failed one nonempty selected test and
   restored byte-exact. Main gates passed 506 lifecycle tests plus 38 doctests
   (three privilege cases ignored), package/projection filters, strict clippy,
   check, fmt and conform with zero new findings.

### 6.3.1 R8-CLIENTS-DEPLOY staging — decision record (central, 2026-08-30)

**Decision.** The destination half is four serial children — engine state,
standalone skills, client plugins, focused deploy gate — under these frozen
contracts:

1. **Prior ownership is injected engine evidence.** `DeployTargetRequest`
   carries the prior receipt the engine read without creating the state home.
   A provider may update a present destination only when that receipt owns the
   exact physical/logical resource and the observed digest still matches it;
   an absent receipt never authorises an identical foreign occupant. Apply
   rechecks the same receipt under the deployment-state lock before writing.
2. **The lock sidecar has committed and pending generations.** The strict-serde
   epoch-1 `lock-resources.json` is engine-owned and outside the JTD intent and
   receipt wires. Each binding carries generation, plan hash and exact physical
   lock resources. A pending binding is durable before its matching intent and
   therefore before the first external write; finalisation promotes it to
   committed only after the receipt is durable. The old committed binding is
   retained throughout an update, so no crash window loses the inverse lock.
3. **One stable deployment lock serialises sidecar/state transitions.** Apply,
   recovery, saga rollback and undeploy take the deployment-id lock, then the
   union of current, committed and pending destination locks in canonical order.
   The deploy plan hash binds `lock_resources` as well as owned resources.
   Recovery requires the pending binding matching its intent; stale retirement
   clears only that pending generation. Receipt finalisation and benign-intent
   retirement promote the matching pending binding. Successful inverse clears
   committed ownership after the rolled-back receipt is durable.
4. **Legacy compatibility is one-way and safe.** An ordinary non-reference
   receipt created before the sidecar may fall back to its owned resources
   because its descriptor proves lock set equals owned set. A reference owner
   never has that fallback and never reconstructs a physical lock by parsing a
   logical resource string. The first reference-owning provider cannot reach an
   external write until its pending sidecar reads back valid.
5. **Read-only planning truly creates nothing.** Receipt/sidecar inspection uses
   a no-create state view; `DeployState::open` remains apply-only. The same prior
   receipt value reaches provider plan in both `--plan` and preapply.
6. **Three standalone skill providers share one filesystem implementation.**
   They accept only a file-shaped `skill` artifact and strict single-component
   `config.name`, require frontmatter identity to match, then own exactly one
   entry file at Claude `.claude/skills/<name>/SKILL.md`, Codex
   `.agents/skills/<name>/SKILL.md`, or OpenCode
   `.config/opencode/skills/<name>/SKILL.md`. Existing unowned occupants and
   receipt-owned drift refuse; removal leaves every unrecorded neighbour and
   prunes only proven-empty directories.
7. **Pure client paths live in `vibe-agent-projection`.** New helpers accept the
   injected home and never call ambient directory resolvers. The older public
   ambient agent APIs remain compatibility surfaces; lifecycle providers call
   only the pure helpers.
8. **Claude/Codex plugin state is logical and marketplace bytes are immutable.**
   Each provider validates its exact directory projection, rejects a client
   plugin name outside the documented kebab-case grammar, fingerprints the
   tested client minor and materialises a native local marketplace below
   `settings_root/client-marketplaces/<client>/<target>/<artifact-digest>/`.
   Marketplace bytes and registration are checkpointed CAS-like support, not
   receipt-owned; the receipt owns one `plugin@marketplace` logical member and
   locks that client's private plugin state. Artifact changes require
   `undeploy, then deploy` in epoch 1.
9. **Client processes receive a clean, injected environment.** The absolute
   executable is spawned with no inherited token/PATH environment; HOME and
   USERPROFILE come from `user_home`, Claude receives
   `CLAUDE_CONFIG_DIR=<home>/.claude`, Codex receives
   `CODEX_HOME=<home>/.codex`, and stdout/stderr are bounded. Missing or
   unsupported versions refuse before writes: Claude `2.1.x`, Codex `0.148.x`,
   OpenCode `1.17.x`.
10. **Private CLI JSON and idempotence are measured, not guessed.** Isolated
    live probes on 2026-08-30 pinned Claude's installed array
    (`id/version/scope/enabled`) and Codex's `{installed,available}` object
    (`pluginId/name/marketplaceName/version/installed/enabled`). Repeating local
    marketplace add and install is a successful no-op in both clients. The
    epoch-1 argv remains §6.3.0.7's exact add/install/list/uninstall or remove
    sequence; post-command list is the independent state witness.
11. **OpenCode owns entries, never the whole document.** Its plugin provider
    validates the strict projection, publishes every selected skill file and
    owns `home:.config/opencode/opencode.json#mcp/<name>` while locking the
    physical JSON document. MCP names and skill components are validated before
    becoming resource identities. Parse/merge/canonical-encode/atomic-replace
    preserves every foreign value; remove drops only receipt-owned members and
    exact skill files. No `opencode plugin` command exists in the adapter.
12. **The focused proof uses real fake processes.** A small compiled fake client
    executable implements the measured version/list/add/install/remove contracts
    inside an injected temp home and records exact argv. Provider tests cover
    plan/apply/verify/recover/remove, sidecar crash windows, unowned and drifted
    destinations, reference sharing, marketplace idempotence and foreign JSON/
    skill neighbours. No real home, token, network or installed client is used.

**Ratified at R8-CLIENTS-DEPLOY-FOUNDATION acceptance (central, 2026-08-30).**
Six implementation rulings make decision items 1–5 executable before any client
provider is admitted:

1. **Prior ownership is one borrowed engine read on both planning surfaces.**
   `DeployTargetRequest` receives the exact prior receipt from a no-create view;
   preplan owns that value beside the plan, and apply compares the whole receipt
   again as its first state judgement under the deployment lock. A changed
   receipt refuses before intent, external apply or even a legacy sidecar repair.
2. **The sidecar is a strict two-slot engine record, not wire growth.** Every new
   plan stages an epoch-1 `{generation,plan_hash,resources}` pending binding and
   reads it back before intent. Finalisation requires the matching binding after
   the receipt is durable; update leaves old committed beside new pending until
   that promotion. Intent and receipt JTD shapes remain unchanged.
3. **Lock order is total across forward and inverse paths.** Apply/recovery take
   the stable deployment lock and the current+committed+pending destination
   union. Undeploy and saga take the same deployment lock and the committed+
   pending union, so an interrupted update's physical destination is never left
   open while the older receipt is reversed. Lock resources use their own plan-
   hash frame and preserve declared spelling while lock files use shared physical
   identity.
4. **Legacy compatibility is absence-only.** An ordinary record with no sidecar
   may derive its physical set from typed owned resources and materialise a
   pending binding for interrupted or benign settlement. Once a sidecar exists,
   an absent or mismatched committed generation is corruption, never permission
   to fall back. Reference ownership has no missing-sidecar fallback at all.
5. **Each crash window moves one attributable slot.** Matching recovery and a
   benign receipt+intent promote the exact pending generation; stale retirement
   clears only its own pending binding; failed verification still commits the
   lock set of the failed receipt; a successful inverse clears committed only
   after the rolled-back receipt is durable. Malformed state refuses before any
   provider verb that could mutate a destination.
6. **Acceptance is behavioural and mutation-backed.** Ten worker REDs plus three
   independent central REDs each failed one selected test and restored exactly.
   Main gates passed 525 lifecycle tests plus 38 doctests (three privilege cases
   ignored), the 66-test deploy and 19-test vibe-bin filters, strict clippy,
   check, fmt, conform with zero new findings, and a clean specmap gate.

**Ratified at R8-CLIENTS-DEPLOY-SKILLS acceptance (central, 2026-08-30).**
Five implementation rulings make decision items 6–7 executable without opening
the plugin surface:

1. **The producer and destination share one typed artifact.**
   `package:static-skill` now records `kind=skill, shape=file` without changing
   its exact bytes, freshness, frontmatter/include behavior or one-output shape.
   The lifecycle admits that proven pair only and reuses the producer's one
   frontmatter parser.
2. **Three destinations are data over one provider.** Claude, Codex and OpenCode
   select one closed `SkillClient` implementation, strict `{name}` config and
   the shared portable skill-name grammar. Public pure Agent helpers derive the
   three user roots from injected home; the lifecycle proves the helper-relative
   path equals the resource identity before it may plan or publish exact bytes.
3. **Intent evidence makes recovery reachable but never authorises apply.** A
   validated unretired intent reaches provider plan through the no-create state
   view only. Exact resource, independently observed desired digest and
   `prior_generation` agreement distinguish an interrupted first deployment or
   update from an unowned occupant/drift. Apply receives no intent and rechecks
   receipt ownership under locks; only matching plan hash plus the engine's
   three-digest law may call idempotent `recover`.
4. **Inverse containment is exact.** Remove accepts exactly the configured one
   `home:.../<name>/SKILL.md`, requires the receipt to own that member, mutates
   only the precomputed contained relative path and prunes no farther than the
   proven-empty named skill directory. Receipt/caller agreement on any foreign
   string cannot widen the provider perimeter.
5. **Acceptance is engine-driven and mutation-backed.** Ten worker REDs plus
   three independent central REDs restored byte-exact. Engine tests cover all
   three clients, crash-after-publication recovery for generation 0 and an
   update generation, stale intent, unowned/drifted occupants and foreign
   inverse requests. Main gates passed 553 lifecycle tests plus 38 doctests
   (three privilege cases ignored), 67 projection tests plus 12 doctests,
   strict check/clippy/fmt and conform with zero new findings. Specmap is 6,831
   units / 2,987 tagged items / 2,728 edges, with 0 suspects, gated orphans or
   unresolved host edges and 25 standing warnings.

**Ratified at R8-CLIENTS-DEPLOY-PLUGINS acceptance (central, 2026-08-30).**
Five implementation rulings make decision items 8–12 executable:

1. **A deploy consumes the recorded client projection, not a resemblance.**
   Directory artifacts are re-proven by canonical tree digest. Each adapter
   admits only its exact no-link closed shape, reuses the canonical manifest/MCP
   parsers at fixed projected paths and applies the shared portable-name grammar
   before any resource identity or destination write.
2. **Client execution is absolute, clean and measured.** The selected injected
   executable alone may run. A dedicated env-cleared process map excludes PATH,
   tokens, proxies and ambient homes, installs injected HOME/USERPROFILE plus
   the one client root, and keeps bounded output. Compiled fakes in a fourth
   temp root prove Claude 2.1.x, Codex 0.148.x and OpenCode 1.17.x, exact argv,
   both list JSON shapes and read-only plan home identity.
3. **Claude/Codex own one logical coordinate over immutable support.** A
   deterministic marketplace name and resource digest bind client, target,
   plugin, artifact and version. VibeVM atomically publishes the exact
   projection plus native marketplace below the settings-root CAS path, using a
   pinned safefs parent; any present census/digest drift refuses. The client
   list is the independent desired-state witness, private state has one shared
   physical lock, artifact changes require `undeploy, then deploy`, and inverse
   never removes marketplace support.
4. **OpenCode owns members, never the shared document.** Exact projected skill
   files are whole-file resources; canonical MCP entry digests are logical
   resources sharing the one physical config lock. Plan/apply repeat strict
   receipt ownership, recovery uses intent/three-digest settlement, equal
   documents are not rewritten, and inverse removes only receipt-owned portable
   members while preserving every foreign JSON value and file.
5. **Acceptance is real-process, engine and mutation backed.** Twelve worker
   REDs, three correction REDs and three independent central REDs all failed
   decisively and restored exact hashes. Main gates passed 576 lifecycle tests
   plus 38 doctests (three privilege cases ignored), 67 projection tests plus 12
   doctests, plugin 22, deploy 116, strict check/clippy/fmt and conform with zero
   new findings. Specmap is 6,833 units / 3,011 tagged items / 2,752 edges, with
   0 suspects, gated orphans or unresolved host edges and 25 standing warnings.

**Ratified at R8-CLIENTS-DEPLOY-GATE acceptance (central, 2026-08-30).**
The focused gate adds no second test harness or product path. The integrated
`mechanism::deploy` filter is the executable matrix: all three standalone-skill
and all three client-plugin rows, compiled Claude/Codex fakes, OpenCode
file/member merge, preplan reference sharing, committed/pending sidecar crash
windows, stable deployment/destination locks, recovery, saga and inverse. It
passes 116/116 on the exact accepted tree. The provider atoms' accepted
mutations already name the load-bearing branches; rerunning a duplicate
mutation ceremony at the gate would add no independent oracle. The whole
R8-CLIENTS-DEPLOY child is therefore accepted and the remaining three-client
commissioning gate owns the canonical-package-to-destination e2e.

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

### 7.0 Deploy engine staging — decision record (central, 2026-08-30)

**Decision.** R8-DEPLOY lands the deploy ENGINE — protocol, transaction,
selection, commands, fence — and deliberately no real destination provider:

1. **Scope.** In: the deploy member of §3.2's provider protocol (all six
   verbs; descriptors carrying effect class, reversibility and
   plan-support, with `plan` mandatory), profile selection, the §7.2
   intent/receipt/recover/lock transaction, `vibe deploy [--plan]`,
   `vibe undeploy`, `vibe deployments [--json]`, the third dispatch
   fence, the `[[binary]]` lowering call site (R8-CARGO's named
   follow-up), and the `package:windows-zip` builtin (rescoped into this
   atom at R8-PACKAGE acceptance, plan revision 23). Out: `deploy:vibe-bin`
   (R8-VIBE-BIN's), the three client adapters (R8-CLIENTS'), remote/server
   providers, signature policy and the acquire role.
2. **Protocol home.** The deploy provider is the third sibling in the ONE
   mechanism home — an in-process trait beside Build and Package, its
   executor's single selection path `resolve_mechanism`. The only
   executing implementations this atom are hermetic fixtures at the unit
   seam: a non-builtin selection refuses by the unlanded transport's
   name (the R8-CARGO law), and the one deploy builtin row (`#vibe-bin`)
   refuses as provider-not-landed — a typed refusal, never a stub.
3. **State home.** Deployment intents and receipts are USER state:
   `state/deployments/` under the `vibe_core::settings` directory (the
   canonical `<home>/.vibe`; `$VIBE_SETTINGS` isolates every test) —
   never the project `.vibe/` (R8.1's project-skill receipt is
   project-scoped by design; a deployment's destination scope is
   user/remote/system). Schema-versioned JTD-first wire; the layout
   inside that home is engine-owned (§3.2: the engine owns state
   persistence), disclosed by the implementation, never minted by a
   provider.
4. **Transaction law.** §7.2 binds verbatim: durable intent atomically
   before the first external write; checkpoints; independent verify;
   finalized receipt then retired intent; per-destination lock; staging
   where the destination supports atomic replacement. Recover's
   three-digest law, the benign receipt-plus-intent case, reverse-order
   saga rollback and the undeploy refusal for post-deploy drift are each
   a test, not prose.
5. **Selection.** Profile resolution happens ONCE, in the command layer
   that owns flags, and travels as data: explicit `--profile`, else the
   manifest's `default_profile`, else the exactly-one rule, else a typed
   refusal naming the defined profiles. Environment and secrets never
   choose. The fence arms only when the dispatch carries a resolved
   selection AND that epoch's plan reaches `deploy` — a partial epoch
   arms nothing (§6.0's parameter law).
6. **Plan mode.** `--plan` is a read-only planner, not a chain run:
   resolve the profile, read records and receipts, compute staleness,
   call provider `plan` verbs only, report — no token read, no network,
   no build, no destination mutation (§10's sentinel gate is the proof).
7. **Fence and lowering.** The deploy fence is the third member of
   §6.0's `Fences`, armed at the deploy phase's own-contribution
   boundary with the identical position and reason; build, verify's
   gate, package and deploy fire exactly as the phase line orders. The
   same assembly that arms the fences lowers legacy `[[binary]]` rows
   through the R8-CARGO projection into the build target set; an id
   collision between a lowered row and an authored `[[artifacts.build]]`
   row is a typed refusal (two claimants for one identity), never a
   silent merge.
8. **windows-zip.** A fifth builtin row (`org.vibevm/vibe#windows-zip`,
   role `package`, engine-fresh — its input set is closed and hashable)
   whose provider writes a byte-identical archive on re-run: entries
   sorted by archived name, forward-slash names, one fixed timestamp
   constant, fixed compression parameters, no platform extra fields; a
   directory input enters by its canonical walk. Determinism IS the
   acceptance: two runs, one digest.

**Considered and rejected.** A minimal real destination provider
(`deploy:fs-copy`) to give this atom a live end-to-end — an unfrozen
builtin surface invented for a test; the hermetic fixture seam plus
R8-VIBE-BIN's immediate real e2e cover it. Project-scoped deployment
receipts — a deployment mutates user/remote state and its record belongs
beside that scope. Engine-side profile inference from environment — §7
forbids it twice already.

**Ratified at R8-DEPLOY acceptance (central, 2026-08-30).** Twelve
rulings the landing surfaced, each recorded:

1. **The checkpoint ledger is an engine-owned strict-serde sidecar with
   its own schema epoch, not a JTD wire format.** The §12 freeze spelled
   exactly two §7.2 records, both `deny_unknown_fields`, so a checkpoint
   cannot ride the intent without destroying the planned set recovery
   compares against — and the ledger never crosses a process boundary,
   which is what JTD-first is for. Its `plan_hash` ties it to its
   intent; retirement removes both.
2. **A stale unretired intent (a plan nobody wants any more) settles
   conservatively**: the three-digest law still runs in full, the
   roll-forward does not (the engine must not invent an intent), the
   journal retires, and the fact reports as `stale-intent-retired`. An
   added semantic §7.2 leaves open, recorded in the transaction cell's
   own module doc.
3. **§7.2's reference-ownership exception is deferred** to the first
   provider that can honestly declare it; the collision refusal is
   unconditional until then.
4. **The zip is STORED for every entry.** DEFLATE output is a property
   of the compressor's version and heuristics, so "fixed compression
   parameters" in the byte-identical sense admits exactly one method.
   Larger archives are the acknowledged price; a pinned-compressor
   DEFLATE is a recorded future decision, not a default.
5. **Review closed three unpinned laws, each proven red-under-mutation
   first**: a deployment that applied and then FAILED verification still
   owns its resources (ownership skips only rolled-back receipts); the
   per-destination lock is HELD while the provider applies (a
   non-blocking probe from inside `apply`, not a file that once
   existed); and recovery refuses a resource the intent was UPDATING
   that is observed absent — deletion is the third digest's silent
   spelling.
6. **The archive is proven against an independent consumer**:
   `Expand-Archive` (System.IO.Compression) extracts the hand-rolled
   writer's bytes and verifies every CRC live — the one oracle a
   self-read cannot provide. The census past 65535 entries now refuses
   (0xFFFF is the ZIP64 sentinel in the EOCD count), and the size/offset
   ceiling refuses at exactly `0xFFFF_FFFF` for the same reason.
7. **The MCP lifecycle surface deliberately cannot deploy**: it exposes
   no profile flag, resolves no selection, and its one constructor site
   passes `None` — R8-MECHANISM ratification 2's gate-forced-site case.
8. **`--plan` at the CLI pins the read-only property around the
   provider-not-landed refusal** (nothing built, nothing recorded, no
   state written, no token read); a populated plan body arrives with the
   first landed deploy provider, and asserting one today would assert a
   fiction.
9. **`--profile` arity differs on purpose**: optional on `deploy` (the
   manifest may answer), required on `undeploy` (the destructive verb is
   named) — the architecture's own two spellings, one resolver behind
   both.
10. **`vibe clean deploy` carries the deploy arguments** so the
    clean-prefixed verb reconciles exactly what `vibe deploy` would, and
    refuses `--plan` by name (a clean prefix has already wiped derived
    state; a read-only planner cannot follow it). A small surface
    addition beyond §7's literal list, accepted for symmetry.
11. **Package inputs are shape-aware**: a `directory` artifact input
    resolves by the canonical tree digest (§7.0.8 requires it),
    `{ path }` inputs stay file-only, and the two §6 providers still
    refuse a directory where they cannot use one.
12. **The deployment state home layout is disclosed** (intent /
    checkpoints / receipt / staging per deployment id, locks under the
    home's own `.vibe/`): the odd-looking lock path is the price of
    reusing the audited `vibe_safefs` lock primitive with its post-lock
    identity recheck, and a second lock implementation would have been
    the wrong purchase. Env-reading code cannot leave `main.rs` until
    `conform.toml`'s `env_roots` is deliberately widened — a live
    constraint future CLI splits inherit.

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

### 7.1.0 `vibe-bin` staging — decision record (central, 2026-08-30)

**Decision.** R8-VIBE-BIN lands the FIRST executing deploy provider, and
only it:

1. **Scope.** In: the builtin `deploy:vibe-bin` provider (the
   provider-not-landed arm becomes the real one), its store and launcher
   layout, the update/rollback semantics below, and the §10 end-to-end
   gate (deploy into an isolated bin home, RUN the version-free
   launcher, update it, roll it back). Out: any store GC (deferred and
   named — an undeployed payload is disclosed garbage until a GC atom),
   client adapters (R8-CLIENTS), the plugin-supplied replacement fixture
   (it needs the unlanded plugin transport), remote/system scopes.
2. **Layout under the ONE settings dir** (the engine already resolves
   it): immutable content-addressed payloads in `store/<sha256>`, the
   launcher in `bin/<command>` (`.cmd` on Windows, `#!/bin/sh` else),
   and beside it the ACTIVE-PAYLOAD POINTER `bin/<command>.current` —
   one line naming the payload digest. `DeployExecution` carries the
   settings root beside the state home; a provider never resolves a
   home.
3. **The launcher is version-free by construction**: its body is a fixed
   marked template embedding ONLY the command name, the genre/owner
   marker and the pointer indirection — never a version, never a digest,
   never a copied binary. Update rewrites the POINTER (atomically), not
   the launcher; rollback rewrites it back. §7.1's sentence "resolves
   only its active deployment" is the pointer, and the pointer is an
   owned, receipted resource.
4. **Owned resources are the launcher and the pointer — NOT the
   payload.** A CAS payload is write-once, idempotent to re-write
   (which is what makes apply §7.2-recoverable for free) and may be
   shared by generations, so receipt-owning it would make undeploy
   delete what a prior generation still names. Undeploy removes the two
   owned files; the payload stays as disclosed store garbage.
5. **Collision law verbatim**: an existing `bin/<command>` that does not
   carry OUR genre marker — the PROP-025 project-pinned shim or an
   unmarked user file — is a hard refusal naming both origins and the
   fix (another `command` alias). Same-genre is an update, not a
   collision.
6. **Update and rollback are engine words, not new verbs**: update is a
   new generation of the same deployment (new payload written, pointer
   swapped, `prior_state_handle` carrying what restoration needs);
   rollback is the landed saga/remove path restoring the prior pointer
   through that handle. The e2e proves both — the rolled-back launcher
   RUNS the original payload again.
7. **Only an explicit executable artifact may use this provider**: an
   `executable`-kind file artifact named by an explicit
   `[[deploy.target]]`; every other kind refuses by name. An ordinary
   application that wants MSI/dpkg/Homebrew names a different mechanism
   (§7.1's own sentence), and nothing infers installation from the mere
   existence of an executable.

**Considered and rejected.** Launcher-reads-receipt-JSON (a shell shim
parsing JSON is a second parser nobody audits; the pointer file is the
receipt's projection, one line, atomic). Receipt-owned payloads (undeploy
would erase shared state). A new rollback verb (the saga and
`prior_state_handle` already spell it).

**Ratified at R8-VIBE-BIN acceptance (central, 2026-08-30).** Thirteen
rulings the landing surfaced, each recorded:

1. **The Windows CAS payload carries `.exe`** — a ratified deviation from
   ruling 2's literal `store/<sha256>`: `cmd.exe` cannot execute a file
   with no PATHEXT extension (verified live — exit 9009 extension-less,
   the same bytes as `.exe` run with argv and exit code preserved), so
   the literal spelling is unimplementable where §10's RUN gate must
   hold. The suffix is the platform's, exactly as the launcher's own
   `.cmd` one clause earlier; the pointer still names the bare digest.
   The content-addressed-directory alternative was declined.
2. **The PROP-025 shim marker is a forward declaration living beside
   ours**: that genre is `spec/done` and unimplemented, so no writer
   mints it yet; the `vibe bin sync` atom imports this constant rather
   than minting a second spelling. The PROP-019 VVM shims are a THIRD
   genre §7.1 never mentions, today safely refused as unmarked but not
   nameable by the refusal — B-121.
3. **The remove seam was REPAIRED at acceptance** (boss-authored): the
   worker proved the engine handed `remove` identical inputs for a saga
   rollback and an `undeploy`, leaving one residual defect — undeploy
   after a pointer-moving update restored instead of removing.
   `Transaction::remove` now takes the prior-state handle from its
   CALLER: the saga passes the receipt's handle (restore what the failed
   generation displaced), `undeploy` passes none (remove what the
   receipt owns). Proven red-first through the engine's own path
   (`undeploy_after_a_pointer_moving_update_removes_both_owned_files`).
   The provider keeps its own only-when-the-pointer-moved handle law —
   independently honest under §7.2.
4. **A rolled-back launcher SURVIVES**: its bytes are version-free and
   therefore the prior generation's too; rollback restores the pointer
   and deletes nothing the restored generation still needs.
5. **`ProviderNotLanded` and `VIBE_BIN_ATOM` are deleted**, not kept as
   reachable-looking dead surface; a future deploy builtin collected
   before its adapter refuses through `UnknownBuiltinProvider`.
6. **The deploy-role refusals moved to a `DeployProviderError` section**
   carried transparently by the ONE layer enum — the landed "no second
   enum" rationale evolved with its reason recorded (a destination, two
   extra verbs, laws no producing provider can raise), and the rendered
   error surface is unchanged.
7. **`plan` reads the destination, read-only, for the collision law**:
   the operative half of the landed purity sentence is "mutates no
   destination", and a plan that promised a write apply would refuse is
   a lying plan. §3.2's "report external effects" asks for exactly this
   consultation.
8. **Two invented `command` laws stand**: the reserved `.current` suffix
   refuses (two commands would claim one file on POSIX), and reserved
   Windows device names refuse on every platform (a portable manifest
   must not install `nul`).
9. **The payload write is checkpointed, never receipted** — a completed
   operation under §7.2's ledger, not an ownership; undeploy leaves
   payloads as disclosed store garbage until the deferred GC atom.
10. **The e2e's order is deploy → update → re-deploy → saga → undeploy**:
    the saga rolls back the generation the FAILING run applied, so the
    failing run must be the one that moved the pointer off the original;
    the re-deploy doubles as the CAS write-once proof. Every clause of
    §10's gate is asserted; only the order differs.
11. **Review added the drift composition pin**: a hand-edited pointer
    refuses `undeploy` through the REAL provider's `verify`
    (`undeploy_refuses_a_hand_edited_pointer_through_the_real_provider`),
    binding the engine's §7.2 drift law to vibe-bin's own observation.
12. **A landed CLI assertion was corrected, not weakened**: `--plan` has
    always created the empty state home directory (`DeployState::open`
    is `create_dir_all`); the old absence assertion held only because
    the run refused before planning. The realigned assertions pin "no
    deployment recorded", which is the engine's own unit law.
13. **A legacy `[[binary]]` cannot be named by a `[[deploy.target]]`**
    (validate_plane's artifact-id set predates the orchestrator's
    lowering) — found, not acted on, filed as B-122 with the migration
    spelling in the refusal.

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
