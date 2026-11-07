# SYSTEM-TEMPLATE → SYSTEM design risk review

Naive concatenation is not boot-equivalent. Recommended: an explicit authored API projection using the existing selected-node boot owners, with mutable local/task context outside the stable prefix. No implementation, runtime or provider claims.

## Concrete risks

### Repeated boot and changed interpretation

AGENTS opening and generated redirect both order another STATIC/INDEX read. Appending all bodies leaves these commands active and encourages rereads. INDEX read-kind static is unconditional dynamically linked authored input, not authorization for Normal compilation. Preserve authored literal #use; do not parse prose links as imports.

Basis: AGENTS.md; PROP-009 SESSION-START-ORDER/AUTHORED-INDEX-BOUNDARY; boot_artifacts::prepare_index_body.

### Conditions become unconditional

INDEX supports os conditions evaluated for session OS, while installed conditions are resolved during generation. AGENTS also has central/worker, harness, active-campaign and specspace selectors outside INDEX grammar. Concatenating all branches changes eligibility; generation on Linux cannot silently mean every consumer is Linux.

Basis: INDEX header; PROP-009 HISTORY-WHEN-SITE; AGENTS campaign/quiet/specspace sections.

### Local state and secrets leak into reusable prefix

AGENTS references ~/.vibe/steward settings, complete plan and optional formal handoff, alongside launcher/token-path operational facts. Instructions referencing local state are not permission to embed the contents. Expanding these references would publish user-specific state, possibly confidential material, freeze advisory custody, and make an ostensibly stable prefix differ by user/session. No local file contents were inspected in this review.

Basis: AGENTS User-local central stewardship / Explicit formal handoff.

### Authority elevation and lost override behavior

A project instruction loaded by an agent and the same bytes assigned an API instruction role are not intrinsically interchangeable. User-local preferences, package instructions, examples, and operational grants must not acquire unqualified provider-level authority just because they are in a file named SYSTEM.md. Delimiters cannot enforce a role boundary inside one message. Actual adapter envelope and intended override behavior require an explicit contract. This is design reasoning, not a provider capability claim.

Basis: AGENTS owner-driven settings and grants; PROP-072 SOURCE-DESCRIPTOR-GRAMMAR no authored trust field.

### Worker scope cannot be restored after full-prefix loading

An always-loaded full SYSTEM already violates the quiet packet limited-read/context boundary even if the model later sees the quiet exception. Central full boot and worker bounded packet require distinct selection before constructing the request. Do not rely on a later user message to deactivate already transmitted central material.

Basis: AGENTS quiet clause and recorded codexrunner guard.

### Root prefix overreaches member scope

Root effective boot aggregates members, whereas leaf scope inherits foundation plus its own subtree. Root AGENTS prose also selects registered specspaces. One universal root bundle loses scoped dependencies and may apply unrelated instructions; manually concatenating ancestor AGENTS may duplicate content and miss effective-view rules.

Basis: PROP-009 EFFECTIVE-BOOT/FOUNDATION-DOWN/MATRYOSHKA-SCOPES; AGENTS Specspaces.

### Current-state injection destroys stable identity

Time, cwd, branch, current atom, campaign completion, advisory custody and model selection should not be interpolated into the stable bytes. Some current-state material already resides in AGENTS operating ledger; copying the entire file inherently makes these edits prefix edits. A stable prefix means stable for identical explicit inputs, not permanently frozen stale rules.

Basis: AGENTS operating facts; architecture/map.json meta source/main pins and explicit limitations.

### Stale output mistaken for accepted instructions

A digest of SYSTEM.md alone proves byte identity, not freshness against template, selected node, INDEX, selected STATIC, entry content, policy or recipe. Boot graph verification explicitly does not authenticate STATIC bodies. Existing generation is transactional and fallible compilation happens before publication; a new independent writer must not leave mismatched lanes/system.

Basis: boot_artifacts::write_boot_artifacts_inner; PROP-072 SOURCE-BOOT-CHECK.

### Provider wrapper or tool envelope changes defeat claims

Equal SYSTEM.md files do not prove identical actual API prefixes or cache behavior. Provider wrapper text, role arrangement, tools and model-specific request framing may change independently. Keep artifact digest and actual sent-prefix identity distinct; root research must determine supported wrappers and verification hooks.

Basis: Provider integration code absent from named packet; no provider facts asserted.

### Generated output becomes its own input

Putting SYSTEM.md inside automatic boot discovery or a broad source directory can cause recursive growth on reinstall; SYSTEM-TEMPLATE may itself be enrolled and then repeated. Explicit self-include and indirect cycles also need deterministic refusal. Existing reserved generated artifact exclusions are exact shared-owner rules, not blanket suffix heuristics. New outputs need deliberate ownership integration, not a name-only exception in one caller.

Basis: PROP-072 SOURCE-DESCRIPTOR-GRAMMAR/SOURCE-HOST-BOOT; PROP-009 COMPILED-LANE-IS-NOT-A-CITATION-TARGET.

## Minimal coherent design

1. Define SYSTEM-TEMPLATE.md as the authored entrypoint for the requested API-oriented projection and SYSTEM.md as generated output. Keep the existing filesystem-agent boot route compatible. Template should carry authored adapter-facing instructions and an explicit boot insertion point; do not silently copy/filter arbitrary AGENTS prose or recursively follow Markdown links. Shared permanent project instructions can be referenced from one explicitly chosen owner rather than independently maintained copies.

2. Reuse the selected-node EffectiveBoot/prepared source and publication owners. Preserve STATIC-first order and INDEX entry ordering with source boundaries/provenance. Read unconditional authored inputs as authored bytes; do not recompile them as Normal. Preserve visibility, inheritance, selected format, installed binding and exact generated exclusions. Do not use show effective as a substitute: its declared display universe differs from INDEX boot expansion.

3. For conditions choose one specified contract: portable artifact retains explicitly guarded OS alternatives with runtime OS supplied separately, or target-bound artifact is generated for an explicit OS and rejected for mismatch. Do not choose generation-host OS implicitly. Unsupported selector syntax refuses; do not evaluate arbitrary session prose. Central/worker scope is selected before loading the prefix, not by its content.

4. Publish only repository-approved explicit inputs. Never expand user home, stewardship, token files, plan/custody, Git status, date or task text through discovery. Runtime adapter appends relevant observed local context/task separately. Preserve dynamic reads required by the actual central workflow without claiming those mutable observations are the stable prefix.

5. Give the generated artifact a versioned recipe and an ordered input witness including template bytes, selected node identity, format, condition target if any, boot selection and consumed content digests. Keep volatile timestamps and absolute checkout paths outside prefix bytes. Treat freshness/consistency separately from authentication. Compare current inputs before use or refuse stale artifact; never silently bless hand-edited output.

6. Compile/capture all required inputs and validate before replacing output; recheck relevant observations at publication. Extend existing transaction/ownership seams after reviewing them, rather than adding a second atomicity story. Fresh clones must use committed artifacts by pure reading; generation must not become an implicit install/network step at session boot.

7. At adapter boundary define exact instruction placement and which project instructions the user may override. Document full request prefix independently of SYSTEM file. Verify actual wire envelope with local fake-provider capture; provider caching/performance promises are contingent on root official research and measured request evidence.

## Discriminating tests (not executed)

- Golden differential: selected central boot order/bytes and provenance match incumbent STATIC + applicable INDEX; authored literal directives and examples remain literal; no second boot instruction reread.
- Condition matrix: Linux/macOS/Windows, unconditional dynamic entry, installed-present/absent selection; explicit unknown condition refusal; target mismatch or portable guards according to chosen contract.
- Root/member/specspace and quiet worker matrix: inherited foundation once, unrelated sibling excluded, worker receives only explicit packet-selected content and no central stewardship.
- Sentinel privacy test: home plan/custody/token files exist but are never opened or present in generated output; changing runtime task/date/cwd/local plan changes only dynamic request suffix.
- Determinism/freshness: identical inputs byte-identical across directories; template/entry/STATIC/INDEX/recipe changes invalidate; missing input, malformed metadata and unsupported declared media refuse before output replacement; prior artifact survives failure.
- Self-inclusion: direct/indirect template cycle, SYSTEM in broad own-boot directory, case aliases, output accidentally listed in INDEX, repeat regeneration produces no growth; exact ownership rules preserve unrelated user files.
- Publication/integration: changed source during generation refuses/rechecks according to existing finite boundary; format switch and stale output removal preserve coherent bundle; protected authored source members remain unwritten.
- Fake-provider capture: exact selected role/block ordering, wrapper version, tools prefix, runtime suffix, worker routing and absent duplicate repository auto-injection. No live-provider cache claim from this test.

## Owner decisions only if unresolved

- What override/authority contract does the owner intend for repository/package/user-preference instructions when supplied via the target API role? Recommend trusted application scaffold plus clearly scoped project policy; do not assume unchanged user override semantics.
- Is SYSTEM.md required to be one portable committed artifact, or may it be explicitly target-bound (OS/node/profile)? Node scoping is mandatory either way; portable guarded OS branches best preserve current committed INDEX design.
- How much of current AGENTS must be byte-for-byte represented versus explicitly authored in SYSTEM-TEMPLATE? Full AGENTS inclusion conflicts with removal of bootstrap/local/current-state behavior; no safe generic prose filter can infer the desired policy. If owner already specified exact template semantics, follow them rather than re-ask.

## Bounded source gaps

- Need bounded reads of boot_artifacts/{redirect,transaction,publication,declared_sources,inputs}.rs and effective boot selection before naming exact implementation seam; packet only supplied parent module.
- Need actual target launcher/API request constructor and auto-injection configuration to prove wire role, wrapper and duplicate-load behavior; no provider API facts established here.
- SYSTEM-TEMPLATE.md content or exact owner request was not supplied to this worker, so placeholder grammar, default inclusion policy and desired supported providers remain unverified.
- Architecture map is planning/prototype evidence with dated source pins and explicit runtime limitations; no task title names SYSTEM-TEMPLATE/SYSTEM.md. Do not count new implementation accepted from map labels.

## Source hashes

- `AGENTS.md`: `a03a6eac043fb1ed0fa4bf936c18a4235a718cea5df0c3540cfb780fd618ae2c`
- `vibevm/vibespecs/boot/INDEX.md`: `1b2edaa8cf419590fc32066a8dc8fbc8443317ffc46cb96125364643bb1d4e1a`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml`: `ce2931e621d8d76a57d62c0dcd625de01472920c273792e725b3b61f40e4b33b`
- `vibevm/vibespecs/common/PROP-072-boot-observability.xml`: `47f369f7b0e515d8b53b3b9fb1f89f53e2caafb2eb87d2ee0088f90526f33502`
- `crates/vibe-workspace/src/boot_artifacts.rs`: `63ce007fbdce1bb487187681c8feae6b2573bef2de9ab235c553d6148f847d10`
- `campaigns/next-semantic-transfer-2026-11/architecture/map.json`: `eec748973968d78a7b1501de0d3878cb27c16192c0bfe2a8b0c02a3de8f737cc`
