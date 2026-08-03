# G1-B040-SEAMS — census of the host-crate seams

Read-only census for B-040 (the owner's decision to refactor vibevm's own
seams onto scaffold-B: typestate, newtypes, builders with field-obligation
expressed in the type, sealed traits, PhantomData state). It measures the
*seams as they are today* across the host crates, so the boss has a
measured map before any point refactor. Every claim is pinned to
`path:line` relative to the worktree root.

**Perimeter read.** Host crates `crates/*/src/**` — 18 crates
(`progress-core`, `vibe-actions`, `vibe-check`, `vibe-cli`, `vibe-core`,
`vibe-graph`, `vibe-index`, `vibe-install`, `vibe-llm`, `vibe-mcp`,
`vibe-publish`, `vibe-registry`, `vibe-resolver`, `vibe-settings`,
`vibe-spec`, `vibe-test-support`, `vibe-wire`, `vibe-workspace`) — plus
`xtask/src/**` for completeness. 506 `.rs` files under `crates/*/src`.
Excluded per packet: `packages/`, `vibedeps/`, `refs/`, `legacy-spec/`,
`campaigns/`, `fixtures/`. `crates/vibe-graph/src`, `crates/vibe-llm/src`,
`crates/vibe-test-support/src` were scanned and surface no pub traits,
no builders, no `#[must_use]`, no PhantomData — they are leaf/utility
crates with no seam surface of their own; `vibe-test-support` exists to be
imported by tests.

**Headline.** The seams are *open*, not *sealed*. All 24 pub traits are
plain `pub trait` with public methods and **no** sealed pattern (no private
supertrait, no module-private gate) — anyone may implement them, and the
impl census confirms many are implemented from other crates (notably
`vibe-cli` wires most observer/provider seams). Field-obligation in the one
real builder (`ActionBuilder`) is enforced **at runtime in `build()`**, not
by the type; there is **zero** typestate in the perimeter — `PhantomData`
does not appear once, so no builder carries a state marker. Newtypes exist
and are mature on the `vibe-core` identity seam (`ContentHash`, `RelPath`,
`Group`, `PackageName`, `CapabilityNamespace`/`Name`), all validating at
construction; the asymmetry is that only `Group` validates **on load**
(`serde(try_from)`), its validated siblings are `serde(transparent)` and so
accept any string off the wire, and the same `sha256:` content-hash that is a
validated newtype in `vibe-core` is a bare `pub content_hash: String`
throughout `progress-core`. `#[must_use]` is used 146 times, but it is
concentrated: 119 of them are in `vibe-cli`'s TUI builder widgets, and the
locator's "11 in `action.rs`" is exactly right (`action.rs:364`–`441`).

---

## Q1 — Pub traits per crate, sealed-ness, and impl counts

24 `pub trait` declarations in the perimeter. **None is sealed.** A search
for sealed-pattern constructs (`: Sealed`, `: private::`, `: crate::seal`,
`seal::Sealed`, `super::seal`, `impl Sealed`) returned **no matches**.
`VersionEnumerator: DepProvider` (`vibe-resolver/src/lib.rs:263`) is the only
trait with a supertrait at all, and `DepProvider` is itself `pub`
(`vibe-resolver/src/lib.rs:210`) — that is public inheritance, not sealing.
The word "seal"/"sealed" in the tree is the **domain** concept of verdict
sealing (`progress-core/src/seal.rs:1` — "Sealing — recording that a file's
verdicts hold for its current text", DRIFT-026 / PROP-043 §7.1), not the
trait-sealing pattern; its only other appearances are its CLI wiring
(`vibe-cli/src/commands/progress/seal.rs:80`, `progress.rs:44`).

Impl counts below distinguish **prod** (a non-test, non-doc-comment
`impl Trait for Type`) from **test/fake** (inside `#[cfg(test)]`, `mod
tests`, or a `tests.rs`/`*_tests.rs`/`test_support.rs` file) from
**doc-comment** examples (`/// impl Trait for …`, which are documentation,
not real impls). "Cross-crate" marks impls living outside the trait's
defining crate.

| Trait | Defined | Sealed? | Prod impls (crates) | Test/fake impls | Doc-comment examples |
|---|---|---|---|---|---|
| `Check` | `vibe-check/src/lib.rs:338` | no | 11 — all `vibe-check` (`checks/*.rs`: `wal_wellformed:25`, `wal_freshness:18`, `subskill_structure:27`, `review_aging:20`, `redirect_block:17`, `manifest_validity:17`, `lockfile_files:19`, `i18n_coverage:24`, `features_graph:26`, `boot_directory:20`, `activation_conflict:26`) | 0 | 0 |
| `SearchProvider` | `vibe-actions/src/search/mod.rs:161` | no | 5 — all cross-crate `vibe-cli` (`prefs/tui/search/providers.rs:105` SettingsProvider, `:204` PrefsActionProvider; `tree/tui/search/providers.rs:69` PackageProvider, `:158` FieldProvider, `:235` ActionProvider) | 1 — `vibe-actions` FakeProvider (`search/tests.rs:38`) | Commands (`search/mod.rs:133`, not in code) |
| `InterpreterProbe` | `vibe-workspace/src/hooks.rs:189` | no | 1 — `vibe-workspace` SystemProbe (`hooks.rs:196`) | 2 — `vibe-workspace` (`install/tests_hooks.rs:48`, `hooks/tests.rs:31`) | OnlyBash (`hooks.rs:180`) |
| `HookRunner` | `vibe-workspace/src/hooks.rs:230` | no | 1 — `vibe-workspace` SystemHookRunner (`hooks.rs:244`) | 2 — `vibe-workspace` (`install/tests_hooks.rs:58`, `hooks/tests.rs:42`) | AlwaysExit (`hooks.rs:216`) |
| `EvidenceProvider` | `progress-core/src/evidence.rs:44` | no | 2 — `progress-core` NoEvidence (`evidence.rs:51`); cross-crate `vibe-cli` SpecmapEvidence (`commands/progress_evidence.rs:83`) | 1 — `progress-core` Stub (`report.rs:296`) | Fixed (`evidence.rs:33`) |
| `VendorObserver` | `vibe-registry/src/vendor.rs:126` | no | 2 — `vibe-registry` NullObserver (`vendor.rs:134`); cross-crate `vibe-cli` CliVendorObserver (`commands/registry/vendor.rs:55`) | 0 | Collector (`vendor.rs:111`) |
| `Registry` | `vibe-registry/src/lib.rs:106` | no | 3 — all `vibe-registry` (`local_registry.rs:239` LocalRegistry, `git_registry.rs:173` GitRegistry, `git_package_registry/mod.rs:433` GitPackageRegistry) | 0 | EmptyRegistry (`lib.rs:67`) |
| `GitBackend` | `vibe-registry/src/git_backend/mod.rs:168` | no | 1 — `vibe-registry` ShellGit (`git_backend/shell.rs:135`) | 6 — all `vibe-registry` (`tests/index_fast_path.rs:163`, `tests/registry_cells_oracle.rs:116`, `git_registry/tests.rs:187`, `git_package_registry/test_support.rs:82`, `multi_registry_resolver/test_support.rs:69`, `git_package_registry/lookup/tests.rs:299`; plus `git_package_registry/auth.rs:273` inside a test mod) | StaticTags (`git_backend/mod.rs:144`) |
| `InstallSource` | `vibe-install/src/lib.rs:96` | no | 1 — cross-crate `vibe-cli` InstallResolver (`commands/install/resolver.rs:65`) | 1 — `vibe-install` MockSource (`tests/incremental_in_place.rs:41`) | LocalSource (`lib.rs:67`) |
| `PlanObserver` | `vibe-install/src/events.rs:54` | no | 2 — `vibe-install` NullObserver (`events.rs:62`); cross-crate `vibe-cli` CtxObserver (`commands/install/mod.rs:44`) | 0 | Collector (`events.rs:41`) |
| `TokenStore` | `vibe-index/src/server/auth.rs:34` | no | 1 — `vibe-index` FileTokenStore (`server/auth.rs:80`) | 1 — `vibe-index` FakeTokenStore (`tests/seam_fakes.rs:28`) | 0 |
| `PackageScanner` | `vibe-index/src/scanner/mod.rs:52` | no | 2 — all `vibe-index` (`scanner/from_github.rs:71` FromGithubScanner, `scanner/from_clones.rs:27` FromClonesScanner) | 0 | 0 |
| `RateLimiter` | `vibe-index/src/server/rate_limit.rs:183` | no | 1 — `vibe-index` TokenBucketRateLimiter (`server/rate_limit.rs:269`) | 1 — `vibe-index` AlwaysDenyRateLimiter (`tests/seam_fakes.rs:58`) | 0 |
| `RedirectSyncObserver` | `vibe-publish/src/redirect_sync.rs:102` | no | 2 — `vibe-publish` NullObserver (`redirect_sync.rs:110`); cross-crate `vibe-cli` CliRedirectSyncObserver (`commands/registry/redirect/sync.rs:27`) | 0 | Collector (`redirect_sync.rs:92`) |
| `RepoCreator` | `vibe-publish/src/creator.rs:116` | no | 3 — all `vibe-publish` (`github.rs:147` GitHubCreator, `gitverse.rs:126` GitVerseCreator, `direct_git.rs:63` DirectGitCreator) | 1 — cross-crate `vibe-cli` MockCreator (`commands/workspace/tests.rs:84`) | StaticHost (`creator.rs:81`) |
| `Transport` | `vibe-mcp/src/transport.rs:37` | no | 2 — all `vibe-mcp` (`transport.rs:71` StdioTransport, `:120` MemoryTransport) | 0 | OneShot (`transport.rs:24`) |
| `McpTool` | `vibe-mcp/src/tools.rs:35` | no | 4 — all `vibe-mcp` (`tools.rs:68` QueryPackage, `:155` ReadSubskill, `:286` MaterialiseSubskill, `:424` AgenticExplain) | 0 | 0 |
| `InferenceBackend` | `vibe-mcp/src/agentic.rs:258` | no | 2 — all `vibe-mcp` (`agentic.rs:294` RelayBackend, `:323` InlineBackend) | 0 | 0 |
| `Palette` | `vibe-cli/src/commands/tree/tui/theme/palette.rs:116` | no | 5 — all `vibe-cli` (`palettes/rose_pine.rs:44` RosePine; `palettes/catppuccin.rs:115` Mocha, `:128` Macchiato, `:141` Frappe, `:154` Latte) | 0 | Mono (`palette.rs:100`) |
| `SectionSource` | `vibe-spec/src/embed.rs:29` | no | 1 — `vibe-spec` FsSectionSource (`embed.rs:111`) | 5 — all `vibe-spec` (`embed.rs:166` MockSource, `use_graph.rs:143`, `pipeline/tests.rs:17`, `link_table.rs:128`; `embed.rs:249` DocSource) | 0 |
| `DepProvider` | `vibe-resolver/src/lib.rs:210` | no | 5 — all `vibe-resolver` (`embedded_provider.rs:101` EmbeddedProvider, `local_registry_provider.rs:28`, `local_composite_provider.rs:62`, `multi_registry_provider.rs:30`, `sat.rs:81` BoundedProvider) | ~5 — `vibe-resolver` (WorldProvider `tests/solver_properties.rs:95`; MapProvider in `tests/recommends.rs:41`, `sat.rs:314`, `resolvo_engine/tests.rs:48`, `naive/tests.rs:58`; Canned `embedded_provider.rs:319`) | `lib.rs:187`, `:238`, `:283`; `sat.rs:104`; `resolvo_engine/mod.rs:40` |
| `VersionEnumerator: DepProvider` | `vibe-resolver/src/lib.rs:263` | no (pub supertrait) | 4 — all `vibe-resolver` (`embedded_provider.rs:118`, `local_registry_provider.rs:80`, `local_composite_provider.rs:83`, `multi_registry_provider.rs:100`) | ~4 — `vibe-resolver` (WorldProvider `tests/solver_properties.rs:131`; MapProvider `tests/recommends.rs:76`, `resolvo_engine/tests.rs:87`; Canned `embedded_provider.rs:319`) | `lib.rs:246`; `resolvo_engine/mod.rs:48` |
| `DepSolver` | `vibe-resolver/src/lib.rs:318` | no | 3 — all `vibe-resolver` (`sat.rs:183` Sat<P>, `naive.rs:60` NaiveDepSolver<P>, `resolvo_engine/mod.rs:80` ResolvoDepSolver<P>) | 0 | 0 |
| `Watcher` | `vibe-settings/src/events/mod.rs:432` | no | **0** | 1 — `vibe-settings` Noop (`events/tests.rs:250`) | MockWatcher (`events/mod.rs:407`) |

Notable: every trait is implementable from any crate (no sealing), and the
observer/provider family is routinely implemented cross-crate in `vibe-cli`
(`SearchProvider` ×5, `PlanObserver`, `VendorObserver`, `RedirectSyncObserver`,
`InstallSource`, `EvidenceProvider`, plus test `RepoCreator`). `Watcher`
(`vibe-settings/src/events/mod.rs:432`) is the one pub seam trait with **no
production impl anywhere in the perimeter** — only a test `Noop`
(`events/tests.rs:250`) and a doc-comment example (`events/mod.rs:407`).

---

## Q2 — Builders: where field-obligation is in the type vs. runtime vs. neither

A "builder profile" = `new()` plus chainable setters and/or a `build()` and/or
`with_*`. Across the perimeter there is **one** builder that finishes with a
gated `build()`, a handful of `with_*` chains that never validate, and
**zero** typestate (no builder carries a generic state parameter; `PhantomData`
is absent — see Q5).

**The one runtime-validating builder — `ActionBuilder`**
(`vibe-actions/src/action.rs:333`).

- Constructor `fn new(addr)` is **private** (`action.rs:347`, no `pub`); the
  crate hands out builders through `action()`-style entry points, so the
  address (the one mandatory identity) is fixed before chaining starts.
- Setters all return `Self` and are `#[must_use]`: `name_en`/`name`/`description_en`/`description`/`icon`/`category_en`/`params`/`enablement`/`invoke`/`capability`/`search_meta`
  (`action.rs:365`–`444`).
- `pub fn build(self) -> Result<Action, ActionBuildError>` (`action.rs:449`)
  enforces the §3.3 legibility discipline **at runtime**: `name`, `description`,
  and `invoke` are stored as `Option<…>` (`action.rs:335`, `:336`, `:341`) and
  `build()` turns their absence or emptiness into `ActionBuildError::MissingName`
  / `EmptyPresentation` / etc. (`action.rs:452`–`460`). The required fields are
  **not** expressed in the type — there is no `ActionBuilder<MissingName>` /
  `<Built>` state; the obligation is a runtime `Result`.

**Other `pub fn build(…)` — constructors, not validating builders:**

- `vibe-cli/src/commands/tree/tui/search/providers.rs:48`, `:118`,
  `:214` — `build(…) -> Self` factory constructors (PackageProvider /
  FieldProvider / ActionProvider).
- `vibe-cli/src/commands/prefs/tui/lint.rs:62` — `build(schema, paths) -> Self`.
- `vibe-cli/src/commands/prefs/tui/form/mod.rs:159` —
  `build(app: &PrefsApp) -> Option<Self>` (returns `Option`, a runtime
  "may be unbuildable" check, but for a TUI form, not a domain seam).

**`with_*` chains (setters returning `Self`, no `build()`, no validation):**

- `vibe-registry` — `git_package_registry/mod.rs:381` `with_index_client`;
  `multi_registry_resolver/mod.rs:237` `with_git_packages`, `:257`
  `with_path_packages`, `:272` `with_strict_auth`.
- `vibe-settings` — `schema/types.rs:399` `with_default`, `:406` `with_applies`,
  `:413` `with_merge`, `:420` `with_deprecation`, `:427` `restricted`. These
  chain off `KeyMeta::new(…) -> Result<Self, SchemaError>`
  (`schema/types.rs:370`), which **does** validate (non-empty `path`/`description`,
  `schema/types.rs:378`–`383`) — so obligation for the mandatory fields is at
  `new()`-time (runtime `Result`), and the `with_*` setters only fill
  optionals.
- `vibe-cli` — `tree/tui/ui/text_field.rs:72` `with_value`;
  `prefs/tui/registry.rs:120` `with_parent`, `:127` `with_weight`, `:135`
  `with_scope`, `:142` `with_keys`, `:150` `with_body` (TUI widget builders,
  all `#[must_use]`, no `build()`).

**`with_*`-named constructors (no `self`, build from args):**
`vibe-mcp/src/transport.rs:105` `with_input`;
`vibe-actions/src/params.rs:134` `with_default`;
`vibe-actions/src/i18n.rs:153` `with_parent`;
`vibe-publish/src/orchestrator.rs:59` `with_defaults(source_dir: PathBuf, org_url: String)`;
`vibe-index/src/server/state.rs:56` `with_tokens`, `:71`
`with_tokens_and_rate_limit`, `:91` `with_seams`;
`vibe-publish/src/gitverse.rs:74` and `github.rs:89` `with_endpoint`;
`vibe-settings/src/schema/types.rs:290` `with_replacement`;
`vibe-cli/src/commands/tree/tui/settings.rs:245` `with_paths`.

**Summary by obligation mechanism:**

| Mechanism | Where | Count |
|---|---|---|
| Type / typestate / PhantomData (obligation in the type) | — | **0** |
| Runtime `Result`/`Option` in `build()`/`new()` | `ActionBuilder` (`action.rs:449`); `KeyMeta::new` (`schema/types.rs:370`); `PrefsApp::build` (`form/mod.rs:159`) | 3 |
| Chainable setters, no validation, no `build()` | registry/settings/cli `with_*` above | ~13 |
| `with_*`-named constructors (no `self`) | transport/actions/index/publish/cli above | ~10 |

`pub fn new(` appears 72 times across 60 files (`crates`); the overwhelming
majority are plain infallible struct constructors, not builders — only the
three above couple `new`/`build` to a runtime check, and none couple it to a
type.

---

## Q3 — `#[must_use]` census

146 `#[must_use]` attributes in the perimeter (crates only; `xtask` has 0).
The locator's "11 in `vibe-actions/src/action.rs`" is **confirmed exactly** —
`action.rs:364`, `:371`, `:379`, `:386`, `:393`, `:400`, `:407`, `:414`,
`:424`, `:434`, `:441` (the `ActionBuilder` setters, Q2).

| Crate | Count | Concentration |
|---|---|---|
| `vibe-cli` | 119 | TUI builder widgets — `tree/tui/theme/mod.rs` (22: lines `:70`,`:88`,`:101`,`:108`,`:115`,`:122`,`:130`,`:136`,`:142`,`:148`,`:156`,`:166`,`:175`,`:183`,`:189`,`:197`,`:206`,`:214`,`:226`,`:232`,`:238`,`:247`); `tree/tui/settings.rs` (17: `:68`,`:84`,`:94`,`:104`,`:113`,`:122`,`:132`,`:152`,`:162`,`:235`,`:244`,`:255`,`:266`,`:299`,`:311`,`:332`,`:340`); `prefs/tui/registry.rs` (8: `:100`,`:119`,`:126`,`:134`,`:141`,`:149`,`:187`,`:194`); `prefs/tui/form/control.rs` (7: `:69`,`:84`,`:90`,`:96`,`:154`,`:173`,`:190`); `prefs/tui/form/mod.rs` (6: `:61`,`:72`,`:83`,`:115`,`:218`,`:229`); `tree/tui/theme/tier.rs` (6: `:36`,`:42`,`:92`,`:123`,`:142`,`:174`); `tree/tui/copy/file_dest.rs` (5: `:40`,`:49`,`:80`,`:86`,`:92`); `tree/tui/ui/button.rs` (5: `:61`,`:71`,`:78`,`:84`,`:93`); `tree/tui/ui/radio_group.rs` (5: `:56`,`:67`,`:76`,`:82`,`:88`); `tree/tui/ui/text_field.rs` (5: `:52`,`:62`,`:71`,`:78`,`:84`); `tree/tui/copy/settings.rs` (4: `:54`,`:64`,`:74`,`:105`); `tree/tui/ui/card.rs` (4: `:55`,`:74`,`:81`,`:92`); `tree/tui/ui/group.rs` (4: `:51`,`:61`,`:73`,`:80`); `prefs/tui/state.rs` (2: `:39`,`:248`); `prefs/tui/lint.rs` (2: `:83`,`:89`); `tree/tui/ui/coming_soon.rs` (3: `:39`,`:47`,`:55`); `tree/tui/ui/msg_dialog.rs` (3: `:43`,`:52`,`:58`); `tree/tui/theme/glyphs.rs` (3: `:60`,`:86`,`:115`); `tree/tui/ui/mod.rs` (`:56`), `tree/tui/ui/window.rs` (`:56`); `tree/tui/theme/palette.rs` (2: `:73`,`:79`); `tree/tui/theme/palettes/mod.rs` (2: `:40`,`:59`); `prefs/tui/form/lifecycle.rs` (`:48`); `prefs/tui/form/validation.rs` (`:104`) |
| `vibe-actions` | 18 | `action.rs` (11, the `ActionBuilder` setters, above); `keymap.rs` (4: `:58`,`:64`,`:70`,`:151`); `params.rs` (2: `:184`,`:235`); `context.rs` (`:56`) |
| `vibe-settings` | 9 | `schema/types.rs` (7: `:277`,`:289`,`:398`,`:405`,`:412`,`:419`,`:427` — the `KeyMeta`/`Deprecation` setters); `events/mod.rs` (2: `:86`,`:285`) |
| `xtask` | 0 | — |
| all other crates | 0 | — |

Fact of distribution: `#[must_use]` is used almost exclusively on **builder
setters that return `Self`** (the `ActionBuilder` setters, the `KeyMeta`
setters, and the TUI widget setters). It is **not** used on the pub seam
traits, on the serde/wire types, or on the newtypes. 119 of 146 (82%) sit
inside `vibe-cli`'s two TUI trees (`commands/tree/tui/**` and
`commands/prefs/tui/**`).

---

## Q4 — Newtypes on the seams, and the String/PathBuf asymmetry

Tuple-struct newtypes wrapping a primitive, in pub surfaces:

| Newtype | Site | Wraps | Validation at construction | Serde |
|---|---|---|---|---|
| `ContentHash(String)` | `vibe-core/src/content_hash.rs:36` | String | **yes** — `parse()` checks `sha256:` prefix + non-empty lowercase hex (`:48`–`62`); `from_validated()` skips (`:66`) | `serde(transparent)` (`:35`) → **no validate-on-load** |
| `RelPath(String)` | `vibe-core/src/rel_path.rs:35` | String | normalization only — `new()` turns `\`→`/` and trims trailing `/` (`:43`–`51`); infallible, no grammar check | `serde(transparent)` (`:34`) → no validate-on-load |
| `Group(String)` | `vibe-core/src/package_ref.rs:107` | String | **yes** — `parse()` rejects empty segments and non-`[a-z0-9_-]` chars (`:112`–`143`); `TryFrom<String>` (`:165`) | `serde(try_from="String", into="String")` (`:106`) → **validates on load** |
| `PackageName(String)` | `vibe-core/src/package_ref.rs:201` | String | **yes** — `parse()` kebab-case (`:205`–`208`); `from_validated()` skips (`:217`) | `serde(transparent)` (`:200`) → **no validate-on-load** |
| `CapabilityNamespace(String)` | `vibe-core/src/capability_ref.rs:50` | String | **yes** — via `kebab_newtype!` macro, `parse()` (`:71`–`74`) | `serde(transparent)` (`:49`) → no validate-on-load |
| `CapabilityName(String)` | `vibe-core/src/capability_ref.rs:65` | String | **yes** — same macro (`:71`–`74`) | `serde(transparent)` (`:64`) → no validate-on-load |
| `Icon(String)` | `vibe-actions/src/action.rs:70` | String | **none** — `new()` wraps as-is (`:74`–`76`) | not derived (not a wire type) |
| `MessageKey(String)` | `vibe-actions/src/i18n.rs:29` | String | **none** — `new()` wraps as-is (`:33`–`35`); derived one-to-one from an address via `for_action` (`:47`) | not derived |
| `Localized(String)` | `vibe-actions/src/i18n.rs:66` | String | **none** — `new()` wraps as-is (`:70`–`72`) | `Serialize` only (`:65`, output-only) |
| `KeyModifiers(u8)` | `vibe-actions/src/keymap.rs:35` | u8 | self-validating bitset via `const` ctors (`:39`–`45`) and `with_shift/ctrl/alt` (`:59`–`73`) | not derived |
| `ItemRef(pub usize)` | `vibe-actions/src/search/mod.rs:76` | usize | **none** — public field, a list index | not derived |
| `CancellationToken(Arc<AtomicBool>)` | `vibe-actions/src/invoke.rs:158` | `Arc<AtomicBool>` | not a primitive wrapper | not derived |
| `CapabilityTag(String)` | `vibe-resolver/src/activation.rs:45` | String | **yes** — `parse()` enforces `<namespace>:<name>` (errors `TagError::MissingNamespace` etc., `:52`–`61`) | not derived (not a wire type) |
| `NodeId(usize)` | `vibe-spec/src/doctree.rs:21` | usize | **none** — arena index, in-bounds by construction | not derived |
| `Rgb(pub u8, pub u8, pub u8)` | `vibe-cli/src/commands/tree/tui/theme/palette.rs:69` | 3× u8 | **none** — public fields | not derived |

Non-tuple value types that also validate-on-load via `serde(try_from="String")`:
`PackageRef` (`vibe-core/src/package_ref.rs:425`), `ActionAddr`
(`vibe-actions/src/address.rs:87`), `CapabilityRef`
(`vibe-core/src/capability_ref.rs:157`), `When`
(`vibe-core/src/manifest/package/when.rs:35`) — these are parsed composite
identifiers, not single-field newtypes, listed for completeness.

**Asymmetry facts (measured, no recommendation attached):**

- *Content hash.* `vibe-core` ships a validated `ContentHash(String)` newtype
  (`content_hash.rs:36`). `progress-core` carries the same `sha256:` digest as
  a bare `String` throughout: `pub content_hash: String` on the cache record
  (`progress-core/src/cache.rs:39`) and on `ParsedDoc`/unit
  (`progress-core/src/doc.rs:82`, `:140`), the free function
  `pub fn content_hash(s: &str) -> String` (`parse/mod.rs:60`), and the
  `SealClaim` fields `now: String` / `was: Option<String>`
  (`progress-core/src/seal.rs:40`, `:38`). Same concept, two representations.
- *Load validation.* Among the validated `vibe-core` identity newtypes, only
  `Group` validates **on deserialize** (`try_from`, `package_ref.rs:106`);
  `ContentHash`, `PackageName`, `CapabilityNamespace`, `CapabilityName` are all
  `serde(transparent)` and therefore accept any string off the wire — their
  validation runs only when a caller explicitly invokes `parse()`.
- *Paths / URLs.* A `RelPath(String)` newtype exists (`vibe-core/src/rel_path.rs:35`),
  but neighboring seams carry paths and URLs as raw `PathBuf`/`String` — e.g.
  `vibe-publish/src/orchestrator.rs:59`
  `with_defaults(source_dir: PathBuf, org_url: String)`, the
  `vibe-index/src/server/state.rs:56`–`91` seam constructors, and the
  `vibe-publish` `with_endpoint` constructors (`gitverse.rs:74`, `github.rs:89`).
  There is no `Url`/`SourceUrl` newtype; source URLs are bare `String`.

---

## Q5 — Sealed traits and PhantomData

**`PhantomData`: 0 occurrences** in the perimeter — confirmed by an
exhaustive search of `crates/*/src/**` (no matches) and `xtask/src/**` (no
matches). The B-040 locator's expectation of zero is **confirmed exactly**.

**Sealed-trait pattern: 0 occurrences.** No `pub trait` in the perimeter has
a private supertrait or a module-private gate (see Q1). The constructs a
sealed pattern would need (`: Sealed`, `: private::`, `: crate::seal`,
`seal::Sealed`, `super::seal`, `impl Sealed`) returned **no matches**.

Consequence (measured, not recommended): because there is no `PhantomData`,
there is also **no typestate** in the perimeter — no builder, state machine,
or handle carries a generic state/type-level marker. Q2's "obligation in the
type" row is empty for the same reason.

The only `sealed`/`seal` text in the tree is the **domain** concept of
verdict sealing (Q1): `progress-core/src/seal.rs:1` and its CLI wiring
(`vibe-cli/src/commands/progress/seal.rs`, `progress.rs:44`) — unrelated to
trait sealing.

---

## Q6 — Serde protocol pub types that cross crate boundaries

~140 `derive(…, Serialize, Deserialize)` items in the perimeter. The
cross-crate hub is **`vibe-core`**: `use vibe_core::` appears **343 times
across 171 files** outside `vibe-core` itself, consumed by every other host
crate (`vibe-cli`, `vibe-registry`, `vibe-resolver`, `vibe-publish`,
`vibe-index`, `vibe-workspace`, `vibe-install`, `vibe-mcp`, `vibe-check`,
`vibe-spec`).

Per-crate serde-type exports (representative file anchors; fields are `pub`
on these structs unless noted):

| Crate | Serde pub-types | Cross-boundary role |
|---|---|---|
| `vibe-core` | `manifest/**` is the hub — `package.rs` (`:75`,`:153`,`:217`,`:283`,`:330`,`:362`,`:417`,`:498`), `package/capabilities.rs` (`:29`,`:66`,`:173`,`:188`,`:210`,`:235`), `lockfile.rs` (`:77`,`:102`,`:162`,`:197`,`:253`,`:375`), `project.rs` (`:37`,`:64`,`:83`,`:108`,`:163`,`:270`,`:346`,`:373`), `document.rs` (`:65`,`:201`,`:237`,`:268`), `subskill.rs` (`:42`,`:76`,`:118`,`:164`,`:242`,`:271`,`:297`), `redirect.rs` (`:42`,`:98`,`:131`,`:144`), `package/{binary,skill,mcp_server,wire,weak_deps,hooks,when,deps}.rs`, `i18n.rs:47`; identity types `package_ref.rs` (`:28`,`:105`,`:199`,`:424`), `content_hash.rs:34`, `capability_ref.rs` (`:48`,`:63`,`:156`), `rel_path.rs:33`, `provenance.rs:28` | the manifest/lockfile/identity schema every other crate imports |
| `progress-core` | `doc.rs` (`:12`,`:27`,`:43`,`:56`,`:73`,`:86`,`:94`,`:101`,`:136`), `model.rs` (`:12`,`:34`,`:55`,`:65`,`:74`,`:86`,`:100`), `cache.rs` (`:37`,`:108`), `state.rs` (`:20`,`:32`,`:46`), `rollup.rs` (`:19`,`:68`,`:99`), `baseline.rs` (`:37`,`:175`), `report.rs:13`, `journal.rs:17`, `evidence.rs:13` | consumed by `vibe-cli` (progress commands) |
| `vibe-index` | `types/entry/{mod,content,relations,aggregate}.rs`, `types/repomd.rs` (`:14`,`:41`,`:73`), `types/kinds.rs:80`, `index/checkpoint.rs` (`:18`,`:43`), `index/inverted.rs` (`:65`,`:74`,`:87`) | consumed by `vibe-registry` |
| `vibe-wire` | generated reports — `generated/{uninstall_report,registry_sync_report,install_report,install_plan,init_report,list_report,registry_publish_report}/mod.rs` (~16 types) | cross-crate IPC/report envelope |
| `vibe-actions` | `params.rs` (`:22`,`:56`,`:102`,`:166`,`:217`), `address.rs:86` | consumed by `vibe-cli` |
| `vibe-mcp` | `jsonrpc.rs` (`:24`,`:46`,`:77`,`:136`), `lib.rs:116` | consumed by `vibe-cli` |
| `vibe-registry` | `index_client.rs` (`:370`,`:381`), `search/cache.rs:60`, `git_registry.rs:38` | internal + `vibe-cli` |
| `vibe-cli` | `commands/vvm/{model,placer}.rs` (~7) | leaf (CLI), not imported by other crates |
| `vibe-publish` | `post_hook.rs:381` | 1 type |
| `vibe-spec` | **none** — no `derive(Serialize, Deserialize)` in `vibe-spec/src/**` | — |

**Validate-at-load — the measured fact.** Of ~140 serde types, only **7**
carry real semantic validation at deserialize time:

- Two custom `impl<'de> Deserialize<'de>`:
  `vibe-core/src/manifest/purl.rs:128` (`Purl`),
  `vibe-core/src/manifest/package/features.rs:74` (`FeaturesTable`).
- Five `#[serde(try_from = "String", into = "String")]` (validation runs in
  `TryFrom`): `Group` (`package_ref.rs:106`), `PackageRef`
  (`package_ref.rs:425`), `ActionAddr` (`vibe-actions/src/address.rs:87`),
  `CapabilityRef` (`capability_ref.rs:157`), `When`
  (`vibe-core/src/manifest/package/when.rs:35`).

Everything else **derives `Deserialize` directly** (no custom impl, no
`try_from`) and therefore accepts any structurally-valid TOML/JSON without a
semantic check. Where validation exists it is **opt-in, post-load**, via an
explicit method the caller must invoke: `Document::validate()`
(`vibe-core/src/manifest/document.rs:336`), `ParamSchema::validate()` /
free `validate()` (`vibe-actions/src/params.rs:211`, `:319`), and the
settings `validate(schema, table)` (`vibe-settings/src/schema/validate.rs:130`).
The validated `vibe-core` identity newtypes that are `serde(transparent)`
(`ContentHash`, `PackageName`, `CapabilityNamespace`, `CapabilityName` — Q4)
likewise do **not** validate on load; only `Group` among them does.

---

*Measurement only. Where to apply scaffold-B is the boss's call, not recorded
here.*
