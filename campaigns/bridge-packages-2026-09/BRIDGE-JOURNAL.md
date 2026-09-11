# Bridge packages — engineering journal

Status: active

This is the chronological evidence log for the bridge-readiness work and the
first two bridge packages. It records discoveries, decisions, implemented
changes, verification evidence, and reusable lessons for a future
bridge-package authoring skill.

Privacy boundary: never record credentials, token values, private
infrastructure contents, or copied upstream source. Upstream facts are recorded
only as public URLs, immutable refs, metadata, and observed behaviour.

## Standing owner decisions

- Finish the technical bridge substrate before treating package population as
  the main activity.
- The first adaptation examples are GitHub Spec Kit and external-skills Skills.
- Do not modify either upstream repository.
- Do not vendor or republish their source in VibeVM-owned repositories. A
  bridge repository contains only VibeVM-owned metadata and adapters; upstream
  bytes are fetched from the original immutable source for the consumer.
- Bridge-package authored content is UPL-1.0. Upstream content retains its own
  licence and must be labelled separately; a bridge never claims to relicense
  upstream bytes.
- Every package and product release stays at `1.0.0` until the owner explicitly
  changes the version. Updates replace the mutable `1.0.0` publication.
- Internet publication and mutable replacement are authorised for this
  campaign. Secret material must never enter public repositories, archives,
  diagnostics, or command output.

## J-001 — The inherited plan was written against obsolete deployment state

Observed:

- `repo-citizen-vision` was written against vibevm `fb90c48f` and a public
  `vibespecs` organisation containing zero repositories.
- The current product tree is newer. Forty-four first-party package
  repositories and the static `vibespecs/index` have already been published.
- The four-platform native distribution, `vibe-index` bundling, installers,
  mutable GitHub release replacement, and Windows binary self-update path are
  already live at product version `1.0.0`.

Decision:

- Original T0 and T5 are complete/superseded and will not be replayed.
- The proposed bump to `1.1.0` is rejected for this campaign; mutable `1.0.0`
  is the owner-selected contract.
- Package-population phases from the inherited plan are replaced by a
  technical-readiness sequence and synthetic/local gates.

## J-002 — Current bridge support is metadata plus embedded files, not a safe external reference

Observed in current specs and code:

- PROP-023 recognises only vendored and git-submodule bridge classes.
- PROP-021 already names a future `dependency-declared embedded source`, but
  implements only `.gitmodules`-driven submodules.
- Registry publication recursively copies an initialised submodule's files,
  strips nested `.git`, and leaves `.gitmodules`; this silently republishes
  upstream bytes and loses an honest gitlink boundary.
- `[package].bridge` parses but is not surfaced by the index or the principal
  list/show/tree/search views.

Decision:

- Implement the already-anticipated dependency-declared embedded source as a
  third, reference-backed bridge class.
- Do not extend the current flattening behaviour for these packages.
- The bridge package and its `content_hash` cover only VibeVM-authored package
  content. Upstream source receives its own immutable commit and content
  identity in the consumer lock and cache.

## J-003 — Required reference-source properties

The coherent mechanism must provide:

1. A credential-free upstream Git URL plus immutable full commit in the bridge
   manifest; a tag may be retained as human provenance but is not authority.
2. A separate upstream licence expression and immutable licence URL. The
   package's own `license = "UPL-1.0"` remains unambiguous.
3. Fetch from the original upstream only after the bridge package is selected.
4. An extracted, content-addressed machine cache with no in-place rewrite.
5. Lockfile evidence for source id, URL, authored ref, resolved commit, content
   hash/recipe, and selected resource hashes.
6. Offline reuse after a successful online fetch; an absent cache under
   `--offline` is a hard, actionable refusal.
7. Projection through the existing containment, portable-path, collision,
   receipt, transaction, and recovery gates.
8. Mutable bridge `1.0.0`: ordinary install respects the lock; `vibe update`
   follows a newly published bridge manifest and records a changed upstream
   pin without inventing a package-version bump.

## J-004 — Upstream reconnaissance without cloning

GitHub Spec Kit:

- Repository: `https://github.com/github/spec-kit`
- Stable release: `v1.0.6`
- Annotated tag resolves to commit
  `96c9bd657bfd5de0d651a6165084932b7304ac99`.
- Upstream licence: MIT; immutable evidence is the `LICENSE` file at that
  commit.
- User workflow commands are generated from `templates/commands/*.md`; the two
  checked-in `.github/skills/*/SKILL.md` files maintain the upstream repository
  and are not the user-facing SDD workflow.
- The useful first core is `constitution -> specify -> plan -> tasks ->
  implement`. A command template alone is not self-contained; the upstream CLI
  supplies templates and helper scripts.

external-skills Skills:

- Repository: `https://example.invalid/removed-upstream`
- Stable release: `v1.2.3`
- Annotated tag resolves to commit
  `6acc160e4e0cd062dbbbd7a1b26ae92855edf07e`.
- Upstream licence: MIT; immutable evidence is the `LICENSE` file at that
  commit.
- The release contains real `SKILL.md` trees. `codebase-design` is the safest
  first direct projection and carries two referenced Markdown resources.
- High-collision names include `code-review`, `implement`, `research`,
  `prototype`, and `tdd`. Spec Kit adapters must retain their `speckit-`
  prefix.

No upstream repository was cloned and no upstream archive or source file was
written locally during reconnaissance.

## J-005 — Safety gaps found before implementation

- Automatic package-phase skill lowering already rejects duplicate physical
  targets before mutation and has receipt-based reconciliation.
- Standalone `vibe skill install` uses an older path that replaces a divergent
  destination directory wholesale and does not establish ownership.
- Skill frontmatter `name` is not checked against the declared name.
- `.vibeignore` is excluded as a file but its patterns are not parsed; changing
  this requires a new hash recipe rather than mutating recipes 0/1.
- `vibe registry test` still constructs an unqualified probe and can report a
  healthy registry as unreachable.
- `vibe outdated --upstream`, remote MCP, non-Cargo build providers,
  command/agent/rule/hook projection, and Cursor skill projection remain open.
- PROP-054's later owner ruling says package installation is consent and total
  observability replaces hook permission prompts. This supersedes the older
  PROP-020 allow-list plan; implementing `[hooks].allowed_groups` would move in
  the wrong direction.

## Reusable lesson candidates for the future skill

- Start with an immutable upstream commit, then retain the release tag only as
  a human-readable alias.
- Separate the bridge licence from upstream licence in every manifest, index,
  report, and README surface.
- Prefer direct projection when upstream already provides valid Agent Skills.
- Use a generated adapter only when upstream's native integration requires it;
  do not transcribe command bodies into the bridge repository.
- Collision analysis is part of package design, not a late install error.
- A bridge readiness gate must prove that the published repository contains no
  upstream source bytes.

## J-006 — The canonical specs now admit reference-backed bridges

Date: 2026-09-11

Changed:

- PROP-023 now defines three bridge classes: vendored, git-native submodule,
  and reference-backed.
- PROP-021's previously parked dependency-declared extension point is now the
  contract for `[[embedded_source]]`: original HTTPS URL, immutable full
  commit, separate source-tree hash, and upstream licence provenance.
- PROP-018 now allows a `[[skill]]` body to come from an authenticated embedded
  source and allows package-local adapters to mount selected upstream
  resources below `references/`.
- PROP-024 now states explicitly that external source bytes are outside the
  bridge package's shippable tree and package `content_hash`.

Why this matters for the future skill:

- It must ask which of the three bridge classes is intended before generating
  files.
- For the zero-copy class it must refuse to scaffold until an immutable commit,
  reproducible source hash, and upstream licence evidence are all known.
- It must describe the bridge package licence and upstream licence as two
  separate surfaces; a single combined `license` answer is structurally wrong.

## Next journal entry

Record the implemented manifest/lock/cache grammar for reference-backed
embedded sources, followed by focused test evidence and the first published
bridge repositories.

## J-007 — Manifest, lock, and authenticated cache substrate landed

Date: 2026-09-11

Implemented:

- `[[embedded_source]]` is a strict package-only table with a closed v1 Git / anonymous HTTPS
  vocabulary, full lowercase 40/64-character commit, recipe-labelled content
  hash, optional full `refs/...` hint, upstream SPDX expression, and immutable
  licence URL.
- `[[skill]].source` selects a direct external skill; nested
  `[[skill.resource]]` selects explicit upstream resources for a local adapter
  and confines their targets below `references/`.
- Lock schema 7 adds portable `[[package.embedded_source]]` evidence without a
  cache path or credential.
- The registry cache fetches the exact commit without submodule recursion,
  checks `HEAD`, records `HEAD^{tree}`, verifies `sha256-tree/1`, strips `.git`,
  and publishes a complete entry by atomic rename. A cache hit re-hashes the
  extracted tree before returning it.

Focused evidence so far:

- `cargo test -p vibe-core`: 469 unit and 188 doctests passed (worker evidence).
- strict core check/clippy and compile-only downstream pass succeeded (worker
  evidence).
- `cargo test -p vibe-registry embedded_source`: 2 passed; mismatch leaves no
  complete cache entry and a second identical request reuses the first entry.

Open loading decision discovered during implementation:

- A cache-only path is insufficient as the public package view: Vibe loaders
  and static composition start from the normal `vibedeps` slot.
- Copying upstream into that slot makes all legacy path loaders work, but
  `vibedeps` is intentionally committed, so it also vendors upstream bytes into
  every consumer repository. A symlink/junction is neither portable nor safe.
- The candidate contract under review is therefore a normal authored bridge
  slot plus portable embedded-source evidence and a source-aware package view;
  internal loaders resolve a named source through that view, while generic
  build tools receive an explicit hydrated physical path. Static compilation
  embeds bytes into its output; cache paths never enter committed metadata.

Windows E2E discovery:

- The first real hydration failed before checkout because the initial cache
  layout concatenated three full identities under an already long
  `%TEMP%`-scoped settings path; Git reported `Filename too long` while creating
  `.git`.
- The cache key is now one domain-separated SHA-256 over URL + commit + source
  hash, and every Windows Git subprocess gets invocation-local
  `core.longpaths=true`. The full identities remain in `source.toml`, the path
  stays bounded, and no machine-global Git setting is changed.

## J-008 — First Windows projection proved both bridge shapes

Date: 2026-09-11

The real upstream commits were pinned with `cargo xtask bridge pin`:

- GitHub Spec Kit `v1.0.6`: commit
  `96c9bd657bfd5de0d651a6165084932b7304ac99`, tree
  `c4a0ea3bfd221b707fed82913794f2bc0bd3c70d`, portable hash
  `sha256-tree/1:91249902cbdad534278a14c437ab9fd43439c3f1df96a7a051d47b08509c88fc`.
- external-skills Skills `v1.2.3`: commit
  `6acc160e4e0cd062dbbbd7a1b26ae92855edf07e`, tree
  `7e0251de7d262684e5e4a326c3ef1132314b9dc2`, portable hash
  `sha256-tree/1:fb4fb6a478b56528fe629245da6cfe97f8ff8650b1b5b241c24b02de81af23f3`.

Two distinct loading modes passed on Windows:

- local UPL Spec Kit adapter + five selected upstream command references;
- direct upstream Matt `codebase-design` skill with all three Markdown files.

After the online dry-run warmed the isolated cache, the actual projection was
rerun with `VIBE_OFFLINE=1` and `VIBE_GIT_BINARY` deliberately pointing to a
nonexistent executable. Both projections succeeded, proving the cache-hit path
did not invoke Git. The resulting target inventories were exactly 6 files for
`speckit-core` and 3 files for `codebase-design`.

## J-009 — Final loading boundary

Date: 2026-09-11

Decision after auditing the existing materialiser, boot compiler and lifecycle
provider roots:

- The bridge package is always a normal physical package slot in `vibedeps`.
- The upstream tree is never copied, hardlinked, junctioned or symlinked into
  that slot. `vibedeps` is committed and the slot record promises that its
  payload is the published package tree; any such mount would be legally,
  operationally or portably false.
- Vibe loaders use an explicit root-aware view. An unqualified path means the
  package slot. A named `embedded_source` means the separately authenticated
  cache root. A missing package path never falls back into upstream.
- Static Vibe compilation may read an explicit external root and commit only
  the compiled output/provenance. A dynamic external boot entry is invalid
  because a fresh clone cannot portably name another machine's cache path.
- Generic build tools receive the verified physical source path separately
  from `provider_root`. Any tool that may write receives a disposable worktree,
  not the canonical cache tree.

Windows full-install evidence:

- Installing `feat:org.vibevm.bridges/spec-kit@=1.0.0` from an isolated local
  registry created the normal committed-form slot
  `vibevm/vibedeps/org.vibevm.bridges.spec-kit/1.0.0`.
- The resulting `vibe.lock` used schema 7, marked `bridge = true`, nested the
  upstream commit/tree/source hash and mandatory licence-file hash, and stored
  no absolute cache path.
- `vibe list --verbose` rendered `ROLE = BRIDGE` and kept package/upstream
  provenance and licences visibly separate.
- `vibe show source-path` returned the package slot; `--embedded upstream`
  returned the verified physical cache tree. The latter and `vibe skill
  install` both succeeded offline with a deliberately nonexistent Git binary.
- `vibe install` itself intentionally did not project agent skills: that is the
  established explicit `vibe skill install` lifecycle, not an implicit side
  effect of dependency installation.

Frontmatter/collision lesson:

- A direct external skill retains its upstream `SKILL.md` identity. The first
  draft renamed Matt's direct projection to `matt-codebase-design`, but the new
  pre-mutation frontmatter check correctly exposed that the upstream document
  declares `name: codebase-design`.
- The bridge now keeps that native name. If namespacing is required, the bridge
  must publish a local adapter with the namespaced identity and mount the
  upstream skill under `references/`; it must not silently relabel somebody
  else's `SKILL.md`.

## J-010 — Adjacent bridge-readiness gaps closed

Date: 2026-09-11

- `vibe registry test` now probes a deliberately absent fully-qualified
  package coordinate. No configured registries is `not-configured`, a clean
  not-found answer proves reachability, and protocol/opening failures remain
  distinct without echoing potentially credential-bearing transport text.
- `vibe outdated --upstream` is opt-in and keeps package-version drift separate
  from the upstream version named by a bridge's GitHub PURL. It uses anonymous
  Git tag discovery, chooses the highest stable SemVer tag, and reports an
  unreachable upstream as `unknown`, never as current.
- Lock/index/list provenance is being made explicit rather than inferred:
  `bridge`, maintainer package identity, and separately licensed embedded
  source records are different fields. This removes the temporary need for
  `outdated` to guess bridge status from `describes` alone.

Implementation consequence for bridge authors:

- Package-local adapters are the default integration surface and work with all
  existing package-relative loaders.
- A direct external skill/resource is admitted only through a typed manifest
  qualifier and a matching locked source record.
- Never generate a symlink farm as a compatibility shortcut.
