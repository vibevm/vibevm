# UP-BOOT / UP-PRIVACY — ordered delivery and personal artifact boundary

Date: 2026-11-16. Architectural contract proposal, not implemented behavior.
Compatibility source baseline: `6a83be1dd35d7bec2de4911ee375c2be3cb8686a`.
Architect HEAD when authored: `aefc4302136b7d0b2b40685ac77bd5a7ee45fb4b` (the
additional commit changes only the source-owner input brief). Routing/root source
candidates are separate uncommitted inputs, not secretly part of this baseline.

Owner mandate remains all USER-PROJECT. This document refines the existing campaign;
it creates no second execution registry, coding worker, publication or new runtime
authority. New boot implementation waits for admitted lifecycle/source foundations.
Privacy classification/export admission can be developed independently against
synthetic reserved-path fixtures before any composed personal artifact is produced.

## 1. Ordered material stream is the delivery authority

For every admitted enabled layer, construct one ordered sequence with explicit
layer identity, root authority, node identity, plan ordinal, relative source path,
condition/decision and request-local byte observation. Layer order is:

1. Project entrypoint/static materials and every applicable INDEX continuation in
   original interleaved order, including live/gated rows.
2. Global user project's static and complete applicable continuation, in its order.
3. Selected per-project override's static and complete applicable continuation.

Do not interpret U and G as concatenate-all-U-then-all-G. Preserve INDEX ordinals
when evaluating/skipping conditions. A request with U1,G1,U2 must deliver U1,G1,U2
when G1 applies, before any personal material. Unknown gate is unresolved, not true;
existing target/host policy remains authoritative. Do not bake portable OS gates
from the launching host or load personal instructions before an unresolved project
continuation that must still be read. Such a transport must retain the deferred
ordered plan/router rather than claim all personal text was already delivered last.

Each layer resolves specification addresses against its own project and declared
dependencies before composition. Same coordinate/path across layers is not the same
source identity. Do not join every relative path onto the project root. Markers/
read accounting must include layer-qualified identity, so one layer's STATIC cannot
suppress another's read merely because node coordinate/path/hash are identical.
A partial segment is not read; freshness checks and marker verification are actual
mechanisms, while model compliance with router text is a limitation, not proof.

Project protected/unmarked rules remain binding. Later delivery does not promote
personal text into stronger provider authority or authorize commands, secret reads,
role/custody transfer or removal of protected project rules. Replacement authority
is separate typed override work, not inferred contradiction matching in this atom.

## 2. Stable prefix and full request are different artifacts

Keep project SYSTEM.md and its existing metadata byte-for-byte independent of
personal state. Do not add personal paths, identities, enabled flags or content to
shared entrypoints, STATIC, INDEX, lock or prefix witnesses. Existing shared router
prose may change deterministically with a product version to explain a generic
personal discovery protocol; its bytes may not depend on the installing user.

Reserve `<actual-selected-node-boot>/local/` for personal composed artifacts:
currently `vibevm/vibespecs/boot/local/`. This is relative to the selected node, not
an arbitrary cwd or global scratch directory. Put `.gitignore` containing `*` inside
that directory, without modifying repository-root ignore rules. This is a usability
measure, never export protection or permission to delete old directory contents.
Use one central layout/classification owner for XML/Markdown and supported layouts.

Persisted `local/SYSTEM.md`, when all relevant layers opt into existing prefix
machinery, represents the stable P segments only, with the project's P bytes as an
exact leading span. `local/SYSTEM.meta.json` records layer/order/root-binding and
stable input witnesses locally. No task, live continuation bytes or per-request
condition results enter that persisted prefix. Without personal layers, composed
stable bytes equal project prefix exactly; a materialization command need not create
a duplicate file merely to prove equality.

Crucial transport restriction: the stable concatenation P_project+P_user+P_override
is NOT the complete instruction request when project or user continuations exist.
A conforming adapter uses span metadata to interleave each layer's fresh complete
continuation before the next layer's stable span. It must not send the entire
composed P string first and append all continuations afterward. Advertise cache
boundaries only along actual delivered byte order; a changing project continuation
may prevent a cross-layer cache hit. Never promise both contiguous stable personal
prefix caching and later user order by quietly changing semantics.

Harness agent-lane delivery retains its existing lane-only rule: API templates are
not injected into a harness lane merely because the stable composed API artifact
exists. Read/classic boot requires no SYSTEM template. A layer without a template
can still contribute its ordinary static/index lane through request-time delivery;
absence of a layer prefix is not layer unavailability. Do not invent a template to
make that layer fit a prefix-only transport.

## 3. Freshness, failures and enablement

Separate absent, disabled, healthy/empty, unavailable and unresolved observations
per optional layer. Empty/no boot contribution can be healthy; initialized root
status alone does not establish compiled boot health. Disabled layers are not read
or resolved for delivery. Visibility/hiding in tree is separate and cannot disable
loading. Existing sessions/resumes do not unread old instructions; enable/disable
applies to subsequent preparation/launch, not retroactive in-process erasure.

On each request, discover admitted enabled roots, check stable prefix inputs and
ordered plan, then obtain bounded stable current continuation observations. Changed
continuation bytes update only that request observation; changed plan/order/condition/
root binding invalidates the pairing. Never serve a stale local composite after a
personal read/validation failure. Optional layer failure skips only that layer with
safe origin/reason/remedy, not all personal layers and never the project lane.
Override that depends on an unavailable personal binding is itself independently
unavailable with an explicit dependency reason. Project baseline failure retains
its ordinary behavior; personal error handling cannot make invalid project boot
appear healthy. No boot/discovery initialization, compilation, network, mutation,
repair or automatic memory migration.

Publishing local artifacts is an explicit finite operation through existing
transaction/cancellation owners, not part of boot file reads. Prepare complete new
artifacts before visibility switch; preserve the previous generation on precommit
failure and never use it as a fallback when stale. Local sensitive outputs need
private permissions, bounded no-follow operations, exact ownership and protected
publication/rollback. Shared project prefix regeneration remains the existing owner.

## 4. Classic/direct discovery — capability and unresolved UX

Launch/API adapters already possess settings authority and must pass admitted
layer roots explicitly. They can implement the ordered stream without a personal
locator in the repository. For classic file-reading boot, an adapter may supply an
explicit file-only personal locator through its user-local instruction/config
surface. The shared router refers only to the generic discovery protocol, never a
machine path. The locator is data: schema/version, absolute selected settings or
personal-root reference, and binding information; no credentials, shell instructions
or trusted executable content. Validate no-follow/bounds and independently admit
the pointed-to roots. A locator does not grant arbitrary filesystem access.

A plain reader with neither a supplied settings/home capability nor a reachable
user-local locator cannot discover an arbitrary relocated VIBE_SETTINGS path by
reading identical repository bytes. Do not conceal this information gap with a
shell command, HOME scan, repo-local personal path, SYSTEM opt-in or silent default
that selects another user project. It still boots the project and reports personal
discovery unavailable. This is graceful optional failure, not a claim that universal
classic discovery is complete.

**Owner-facing decision still needed for universal classic onboarding:** choose the
supported user-local locator registration carrier (for example a fixed home-relative
descriptor written by an explicit user command, or harness-provided user instructions
carrying an explicit locator). Recommendation: a provider-neutral fixed home-relative
data descriptor plus explicit registration, with adapter capability for locating
HOME. That also supports relocated settings without embedding them in project Git.
Exact registration command/path and which harnesses can read the carrier remain a
bounded design gate before classic onboarding implementation. No boot write or
change to UP-ROOT-01's init output set is implied. Do not ask the owner to choose
low-level carrier implementation until actual harness capabilities are inventoried.
The owner decides only if supported coverage requires a real usability/privacy trade.

## 5. Personal content admission at every exporter

The reserved personal subtree is any node's boot/local directory according to the
central supported layout classifier, including descendants of nested project or
package roots. Compare path components, not string-prefix lookalikes. Personal
contents, sidecars, discovery descriptors, absolute root bindings, memory records
and credentials must never be copied into shared boot or published payloads.
External user roots are not traversal inputs to project exporters at all.

Filesystem-copy exporters exclude the reserved subtree BEFORE reading its entries,
following links, opening a file, hashing bytes or constructing a flattened payload
inventory. Classify the relative entry before recursion; do not walk everything
and filter the finished list. Avoid following unrelated symlink/reparse aliases
into an excluded root. Existing exporter unsupported-entry/refusal policies remain;
no new arbitrary traversal permission. Negative tests must prove contents were
not read, not merely absent from a final archive.

Pinned Git-tree/tracked-source inputs have an advertised identity. If their selected
tree includes a reserved personal path, refuse BEFORE hashing/publishing that tree.
Do not silently strip tracked personal files and claim the original tree identity.
Enumerating tracked path names to reject the tree is permitted; reading personal
blob contents is not required. Untracked ignored local output is omitted by tracked
selection, but that fact does not excuse tracked/private canaries. Check staged and
final payload inventories; identity checking may use declared tree objects but
must not inspect the prohibited payload to discover it is personal.

`vibe check` flags tracked reserved paths using selected tracked-tree/index evidence
without dumping content. Ignore rules, cache hash exclusions and a secret regex are
not this privacy boundary. Fail closed on uncertain layout/admission at a real
publication boundary; do not delete or rewrite authored tracked files automatically.
This scope prevents reserved personal artifact export, not universal recognition
of every possible secret or unrelated private file elsewhere in source.

## 6. Verified owner census and remaining census obligations

Inspected at the compatibility baseline (line numbers are evidence locators):

- Workspace publication `crates/vibe-workspace/src/publish/staging.rs::stage_node`
  and `copy_tree_excluding`: currently excludes Git/.vibe and direct generated
  prompt files, but not the nested reserved subtree. Add classification before
  directory traversal, not just is_generated_prompt_artifact on leaf files.
- Direct Git publisher `crates/vibe-publish/src/git_publish/payload.rs`:
  copy_dir_controlled currently receives a recursively accumulated walk; filtering
  afterward is too late, and its walker follows path is_dir/is_file semantics.
- Layout publisher `crates/vibe-publish/src/layout_publish.rs::publish_batch`:
  Git-tree branch uses item.tree directly, bypassing initial filesystem copy before
  content_hash. Requires explicit tracked/pinned tree admission before that branch.
- CLI registry/layout publishing frontdoors route into those owners. Their existence
  does not make the above checks universal; trace actual staging and payload paths.
- Native/application source archives, Git-tree extraction, registry/package payload
  copies and other tracked source consumers need an exact implementation census
  before claiming coverage. Prior reports of Git archive selection are evidence to
  verify, not acceptance or permission to omit tracked-tree refusal.

The implementation brief requires every actual exporter/producer to be named with
input selection, classification point, hash/identity boundary, negative test and
result/unrun status. Do not call this census complete until all supported paths
are accounted for. No live publication is needed to test the boundaries.

## 7. Atomic implementation route and review

1. Ordered layer-qualified read model and pure plan tests, retaining INDEX ordinal.
2. Reserved-path classifier/export admission and synthetic tracked/copy canaries,
   before any personal composed artifact producer is enabled.
3. Launch/API full ordered transport and request-local freshness/failure matrix.
4. Explicit local composition publication, private metadata and cancellation.
5. Classic discovery capability inventory, carrier decision and direct-read tests,
   including no-template projects. No unsupported carrier claimed complete.

These are meaningful semantic atoms, not an excuse to suppress required worker
context. Builder owns implementation dispatch and tests. Each substantial milestone
returns an exact integrated candidate and cross-cutting review evidence. Final
USER-PROJECT acceptance still includes override identity/authority, memory lifecycle,
AI-native origin isolation, tree surfaces and comprehensive gates; this proposal
does not silently defer those obligations.
