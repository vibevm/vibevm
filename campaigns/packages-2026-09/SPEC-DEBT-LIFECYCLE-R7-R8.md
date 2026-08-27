# Lifecycle/extension campaign — R7/R8 amendment continuation

_Draft only, 2026-08-26. Continuation of `SPEC-DEBT-LIFECYCLE.md`; this file is
not authoritative and changes no PROP status. Exact architecture record:
`BUILD-PACKAGE-DEPLOY-ARCHITECTURE-v0.1.md`._

## 0. Evidence and ruling boundary

Landed evidence safe to cite now:

| Evidence | What it proves |
|---|---|
| `e2392893` | JTD-first OpenAI-compatible request/response; synchronous object-safe provider; project/user config merge; credential/endpoint/redirect/proxy/body/timeout safety; provider-independent usage |
| `c0fa49be` | package-skill inventory and project projection have one library owner below the CLI |
| `f42334ff` | initial Chat Completions schema/corpus registration |
| `26929050` | R7.2 strict generated agent-result wire, selected-world prompt resolution, optional CLI provider execution and safe multi-output publication; no selected agent row keeps the complete algorithmic path |
| `9275f373`, `67886ea7` | project package-skill binding has strict JTD receipt, intent/recovery/CAS, exact ownership, lossless path handling and Unicode-9 physical-alias protection across Claude/Codex/OpenCode projections |
| `2a3f3b44` | strict artifact/mechanism/deploy grammar, target DAGs, provider pins/routes and named profiles with parse/write symmetry |

The production provider's mock, loopback, proxy, redirect, size, redaction and
timeout gates are green. The earlier ordinary-process transport failure was
superseded on 2026-08-27 by an owner-authorised production smoke: central
`vibe create` called the official OpenAI-compatible coding endpoint with
`glm-5-turbo`, read the claudez credential only through its token-file path,
exited 0 and produced the exact declared `LIVE_OK\n` output. No token or
provider response body was printed. This proves the live provider seam; mock
and security gates remain the deterministic acceptance evidence.

The remaining sections include owner rulings made after PROP-054 was written.
They authorise implementation but remain draft amendments at their owning
anchors.

## 1. PROP-000 `##CRATE-LLM` present-truth replacement

Owning anchor:
`spec://org.vibevm.core/vibevm/common/PROP-000#workspace`.

Replace the stale “stub in M0, real in M1.5” row with:

```xml
<CRATE-LLM fact="true" status="impl/work">`vibe-llm` — the synchronous,
object-safe LLM provider abstraction and its first real OpenAI-compatible Chat
Completions adapter. R7.1 ships JTD-generated request/response wires, layered
configuration, secret-safe endpoint/transport policy, bounded blocking calls
and provider-independent usage. R7.2 ships selected-world prompt resolution,
strict agent-result output and the optional CLI execution path. Hosted
delegation and MCP remain R7.3–R7.4 work; the provider's existence alone
activates nothing.</CRATE-LLM>
```

Proposed status is `impl/work`, not `impl/done`: the crate is real, but the R7
feature family is intentionally incomplete.

## 2. PROP-054 `##AGENT-CLI` — R7.1/R7.2 successors and config ruling

Append beside `##AGENT-CLI`:

```xml
<AGENT-PROVIDER-SEAM fact="true" status="impl/done">The first CLI provider is
the exact id `openai-compatible`: synchronous `LLMProvider::chat`, generated
epoch-1 Chat request/response types, and a blocking reqwest transport. User
config supplies provider, model, full endpoint and optional token-file path;
project `[llm]` supplies required `default_provider`/`default_model` and optional
`api_key_env`. Project provider/model win independently; endpoint remains
operator-owned. A nonempty project env source wins over a token file and fails
honestly when absent; it never silently falls through to another credential.
Absolute token paths are legal operator configuration; relative paths resolve
beside the selected user config; `~` is literal, never expanded.</AGENT-PROVIDER-SEAM>

<AGENT-CLI-EXECUTION fact="true" status="impl/done">R7.2 resolves each
explicitly selected agent prompt against the selected package world, invokes
the configured provider only in CLI mode, validates a strict generated
`AgentResult`, and publishes every declared output through the shared safe
filesystem cell. Multi-output apply is planned before mutation and preserves
the algorithmic baseline: with no selected agent contribution, no provider is
constructed, no credential is read and the lifecycle remains complete.
Hosted outbox/resume and MCP are separate R7.3/R7.4 adapters over this result
contract.</AGENT-CLI-EXECUTION>
```

Keep exact provider ids; no aliases route Anthropic/OpenRouter names through an
OpenAI wire. Keyed traffic requires HTTPS; keyless HTTP is literal loopback
only. Redirects are disabled, loopback bypasses ambient proxies, 2xx bodies are
capped, connect/whole-request timeouts are explicit, and keys/query/body/raw
request ids never enter diagnostics.

## 3. LLM is an advanced feature, never the algorithmic floor

Owning anchors: `##PHASE-CREATE`, `##PHASE-VERIFY`, `##AGENT-CLI`,
`##OPEN-CREATE-BUDGET`.

Append the following owner-selected laws:

```xml
<LLM-IS-AN-ENHANCEMENT fact="true" status="spec/done">Every core VibeVM
subsystem retains a complete algorithmic path. An LLM may improve quality when
the operator explicitly enables that feature; credentials, endpoint presence
or provider construction never activate a feature. VibeVM remains useful with
no provider, no API access and no agent host.</LLM-IS-AN-ENHANCEMENT>

<LLM-MODE fact="true" status="spec/done">Each LLM-enhanceable feature declares
`off | assist | required`. `off` runs only the algorithmic implementation.
`assist` may call the configured provider and falls back to the algorithmic
result on unavailable/failure. `required` is an explicit operator choice and
fails with remediation when the paid enhancement cannot run. The existence of
`required` never permits removal of the subsystem's algorithmic mode.
</LLM-MODE>

<LLM-LAZY-BOUNDARY fact="true" status="spec/done">Construct/read a provider
only after the selected feature is non-off, non-fresh and actually reaches its
LLM step. Algorithmic runs, fresh runs, dry plans, hosted/outbox delegation and
features not selected read no token and call no API.</LLM-LAZY-BOUNDARY>

<LLM-BUDGET fact="true" status="spec/done">Each feature and run declares call,
input-token and output-token ceilings. Narration names feature, mode,
provider/model, reason, cache posture and budget before spend. Completed state
records actual provider-reported usage. Exceeding a budget follows the selected
mode: `assist` falls back; `required` fails.</LLM-BUDGET>
```

These facts close `##OPEN-CREATE-BUDGET`: budget is per feature/run, not one
global create number.

## 4. Two nouns, one extension machine

Owning anchors: `##POINT-GRAMMAR`, `##CONTRIB-GRAMMAR`, `##ONE-MACHINE`,
`##OBS-REGISTRY`, `##PRESET-LAW`.

Do not add `mechanism:*` as a fourth scheduled-point family. Add a sibling
provider declaration:

```toml
[[mechanism]]
id = "cargo-v2"
role = "build"                 # build | package | deploy | acquire
name = "cargo"
handler = { kind = "native", crate_dir = "crates/cargo-provider" }
protocol = 1
config_schema = "schemas/cargo-build-v1.jtd.json"
freshness = "provider"         # engine | provider
```

`[[extension]]` remains “run this handler at this moment”. `[[mechanism]]`
means “this provider can be selected for this role”. It adds lookup, not a
second scheduler: same qualified provider key, handler taxonomy, JTD envelope,
installed-world collection, disable state, config redaction, narration and
`vibe extensions [--json]` registry.

Logical keys such as `build:cargo`, `package:agent-plugin` and
`deploy:vibe-bin` are capability vocabulary, not provider identity. Selection:

1. exact target `provider = "<group>/<package>#<id>"`;
2. host route `[mechanisms]."build:cargo" = "<qualified-key>"`;
3. VibeVM builtin default;
4. hard error.

Installed packages never seize a role by matching a short name. Exact pins and
generated state are always provider-qualified; two ambiguous authored route
values are a collision, never an interactive pick.

## 5. Artifact target grammar and registry

Owning anchors: `##PHASE-BUILD`, `##PHASE-PACKAGE`, `##ARTIFACT-REGISTRY`,
`##PHASE-FINGERPRINT`.

Proposed manifest surface:

```toml
[[artifacts.build]]
id = "helper"
mechanism = "build:cargo"
provider = "org.example/build#cargo-v2" # optional exact pin
inputs = []
kind = "executable"
config = { manifest_path = "Cargo.toml", package = "helper", bin = "helper", profile = "release", locked = true }

[[artifacts.package]]
id = "helper-windows"
mechanism = "package:windows-zip"
inputs = ["helper"]
kind = "archive"
config = { layout = "distribution/windows" }
```

The nested arrays make an invalid build/package phase unrepresentable. Ordering
comes from explicit artifact-id DAG edges plus fixed lifecycle phase order, not
source interleaving. Each actual artifact record carries id, file/directory
shape, media/kind, platform, canonical path, SHA-256/tree digest, producer
target, logical mechanism, exact provider/version/content hash, input/config/
toolchain fingerprint and verification evidence. Secrets never enter records.

Freshness stays per target. `freshness = "engine"` is legal only for a closed,
hashable input set. `provider` means the engine fingerprint may force a run but
may never suppress the provider probe. Output existence alone is never fresh;
recorded outputs must independently hash to the record.

Existing `[[binary]]` lowers compatibly into one Cargo build target. Direct
`vibe bin` verbs remain; the lowering is compatibility, not the architecture.

## 6. Cargo is the commissioning mechanism, not a hard-coded phase

Amend PROP-024 `##OOS-AUTODETECT`, PROP-025 `##BINARY-TABLE`,
`##SLOT-RESIDENT`, `##TRUST-CURRENT-SLOT`, and PROP-054 `##PHASE-BUILD`:

- no language/build-system autodetection; stack preset or target selects Cargo;
- use `cargo metadata` for package/target resolution;
- execute argv, never inline shell;
- consume `cargo build --message-format=json-render-diagnostics` compiler-
  artifact messages; never guess `target/release/<name>`;
- record manifest/package/target/profile/triple/features/locked/offline/frozen;
- Cargo reports freshness because `build.rs` and path dependencies make a
  second complete Vibe-side input model unsound;
- VibeVM owns DAG/order/provider selection/provenance/output verification;
  Cargo owns its internal incremental compiler.

Maven, Gradle, CMake, npm/TypeScript, Go and future systems implement this
mechanism protocol. A host may route `build:cargo` away from the builtin without
changing the phase model.

## 7. Package targets — static skill and Agent Plugins 1.0

Replace/extend `##PHASE-PACKAGE` with:

```xml
<PACKAGE-ARTIFACTS fact="true" status="spec/done">Package consumes verified
artifact records and emits portable distributables without editing user, client,
server, registry or OS state. A file and a directory are distinct artifact
shapes; a directory is not implicitly zipped.</PACKAGE-ARTIFACTS>

<STATIC-SKILL fact="true" status="spec/done">The static-skill mechanism emits
one UTF-8 `SKILL.md`. A one-file source passes directly after Agent Skills
validation. A multi-file source is legal only through explicit `vibe:include`
directives consuming declared textual resources exactly once, with visible
origin/hash framing. Executable, shebang-bearing or binary resources, unsafe
paths, unresolved references and silently dropped declared files refuse.
</STATIC-SKILL>

<AGENT-PLUGIN-PACKAGE fact="true" status="spec/done">Agent Plugins 1.0 is a
directory artifact: root `plugin.json`, fixed `skills/<name>/SKILL.md`, optional
`mcp.json`, and valid reverse-domain client-extension directories. Validate the
published 1.0.0 schemas locally, enforce symlink/junction/reparse containment,
and record a canonical tree digest. Portable components are skills and MCP;
client-only commands/hooks/agents/LSP never masquerade as portable fields.
</AGENT-PLUGIN-PACKAGE>
```

Canonical Agent Plugin and client-native projections are separate package
artifacts. Unsupported components are declared omissions or hard errors, never
silent drops.

## 8. Deploy profiles own local installation and remote placement

This closes PROP-054 `##OPEN-DEPLOY-TARGETS`. Selected target genres are:

1. project/user agent skill/plugin projections for Claude Code, Codex and
   OpenCode;
2. VibeVM-managed local tool launcher under `~/.vibe/bin`;
3. explicit custom installer for an ordinary application;
4. registry/marketplace/server/remote providers;
5. future system-scope package/config/service providers.

The first commissioning cut is isolated local projection/`vibe-bin`; no live
publish/server mutation is an implementation test.

```toml
[[deploy.target]]
id = "local-helper"
artifact = "helper"
mechanism = "deploy:vibe-bin"
config = { command = "helper" }

[[deploy.target]]
id = "production"
artifact = "helper-windows"
mechanism = "deploy:acme-server"
provider = "org.acme/deploy#server"
config = { service = "helper", environment = "production" }

[deploy.profiles.local]
targets = ["local-helper"]

[deploy.profiles.production]
targets = ["production"]
```

Profiles select ordered destination targets; they are not arbitrary overlays of
the entire manifest. `vibe deploy --profile <name>` runs the inclusive chain;
`--plan` performs no build/download/token/LLM/destination write. Profile choice
is explicit/defaulted in the manifest, never inferred from env or credentials.
`vibe undeploy --profile <name>` is the inverse. `clean` never undeploys.

## 9. Ownership, intent journal and receipt

Every external mutation has a scope (`project | user | machine | system |
remote`), exact provider and durable desired-state digest.

Before apply, atomically write an intent journal containing plan hash, prior
receipt generation and every planned resource/digest. Apply checkpoints
completed operations without secrets. After independent verify, finalize the
strict JTD receipt and retire the intent. Receipt rows include owned resources
and after-digests; removal touches only verified owned rows.

Restart recovery:

- intent without receipt: compare observed paths to prior/desired digests;
  unambiguous state rolls forward idempotently, third-digest mutation refuses;
- matching receipt plus intent: retire the benign post-finalization intent;
- malformed state never authorizes adoption, overwrite or deletion.

Unowned pre-existing paths and cross-provider collisions refuse. A partial
multi-target run is a durable saga; reversible targets recover/undo in reverse
order, irreversible effects remain visible as partial.

Third-party provider processes receive no VibeVM secret bytes. They authenticate
through their own sanctioned configuration. Only a VibeVM builtin may read a
VibeVM secret source and attach it at VibeVM's owned TLS transport boundary.

## 10. Agent/client installation adapters

Commissioning evidence as of 2026-08-26:

| Client | Public skill target | Full plugin posture |
|---|---|---|
| Claude Code | `~/.claude/skills/<name>/SKILL.md` | Claude-native projection + VibeVM local marketplace + Claude CLI |
| Codex | preferred `~/.agents/skills/<name>/SKILL.md` | `.codex-plugin/plugin.json` projection + marketplace + `codex plugin add` |
| OpenCode | `~/.config/opencode/skills/<name>/SKILL.md` (also reads shared roots) | explicit skill/MCP/config projections; its TS/npm plugin API is not Agent Plugins 1.0 |

Use client CLIs for private install state; direct filesystem/config merge only
where the client documents it. Config writes parse/merge/atomic-replace and
carry before/after hashes. Physical paths are de-duplicated across selected
clients.

The project package-skill writer's safety successor is landed at
`9275f373`/`67886ea7`: validated safe names/paths, authenticated selected-world
inputs, plan-time physical collision detection, provider-qualified artifact
ids, strict JTD receipt, intent/recovery/CAS, owned-file diff, output probe
before fresh and desired-versus-owned reconciliation. Physical identity uses
NFC → Unicode-9 full fold → NFC while receipts keep lossless OS path units. An
unowned same-name directory survives and causes refusal; whole-directory
`remove_dir_all` is not ownership. This closes safety for automatic **project**
binding only. User/client installation remains explicit general deploy work.

## 11. Two launcher genres under `~/.vibe/bin`

PROP-025's project-pinned `vibe bin` shim and a locally deployed global tool are
different genres. Their launcher bodies carry an exact VibeVM owner/genre
marker. They never fall through into one another. A name occupied by the other
genre or an unmarked user file is a hard collision; use an explicit alias. Each
GC removes only its own marked launchers.

The deploy launcher is version-free and resolves its one active receipt into an
immutable content-addressed payload. Existing `vibe bin` remains lockfile→slot→
artifact per current working directory.

## 12. VibeVM OS horizon — compatibility, not current scope

Future VibeVM packages may declare locally built artifacts, digest/signature-
pinned HTTPS/registry/OCI prebuilt payloads, system package coordinates, config,
services, accounts and filesystem resources. The package need not contain a
binary.

Acquisition produces a verified artifact/provenance record. Invoking apt/dnf/
pacman/winget/Homebrew is a system-scope deploy mechanism. A future reconciler
adds observed system state to plan; it does not redefine build/package/deploy.

Impose now: stable provider-qualified identities; algorithm-tagged digests;
desired state, artifact and deployment as separate records; explicit scope and
privilege; plan+intent+receipt for mutation; durable partial failure. Privileged
broker, global solver, services and whole-system transactions remain future
work.

## 13. Proposed status queue

No movement is applied by this draft.

| Anchor | Proposed when evidence lands |
|---|---|
| PROP-000 `##CRATE-LLM` | `impl/work` now (`e2392893`) |
| PROP-054 `##AGENT-CLI` provider seam | add `impl/done` provider successor (`e2392893`) |
| PROP-054 `##AGENT-CLI` CLI execution | add `impl/done` successor (`26929050`); hosted/MCP remain work |
| `##OPEN-CREATE-BUDGET` | close by §3 owner ruling after mode/budget implementation |
| `##OPEN-DEPLOY-TARGETS` | close by §8 owner ruling; grammar/profiles landed at `2a3f3b44`, runtime statuses remain open |
| project package-skill binding | record `impl/done` safety successor at `9275f373`/`67886ea7`; do not imply user deploy |
| artifact/mechanism/deploy grammar | record `impl/done` grammar successor at `2a3f3b44`; records/routing/runtime remain open |
| `##PHASE-BUILD`, `##PHASE-PACKAGE`, `##PHASE-DEPLOY` | move only per landed mechanism/target evidence |
| PROP-024/025 build wording | amend with Cargo/general mechanism compatibility when Cargo atom lands |
