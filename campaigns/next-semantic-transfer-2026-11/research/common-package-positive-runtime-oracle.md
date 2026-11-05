# First positive package optional runtime oracle

Independent fixture plan only. No source/tests edited, CLI/Cargo executed, or runtime outcome claimed. Governing accepted choices: `campaigns/next-semantic-transfer-2026-11/research/common-package-positive-decisions.md`. This is a bounded executable fixture plan for primary, not another solver design.

## Use these existing owners/helpers

- `crates/vibe-cli/tests/common/mod.rs` reexports `UserScratch`; its implementation in `vibe-test-support/src/lib.rs:110-153` isolates settings, registry cache and search cache. Create each CLI command with `user.vibe()`, set HOME/USERPROFILE to the fixture home, and clear VIBE_AGENT_COMMAND/VIBE_AGENT_ARGS/VIBE_INVOKED_BY. Do not use AppRegistry: that helper exercises the different global application installer owner.
- `cli_common_package_maintenance.rs::Fixture::{package,run}` gives the actual local directory registry layout, ordinary project manifest, command capture and semantic snapshots. Its helpers are private to that test binary: reuse the pattern in the new target, not an invented cross-test public import. Do not call its `Fixture::new` unchanged: it seeds a maintenance package immediately, before the optional graph is assembled.
- Local registry layout: `<registry>/org.vibevm/<name>/v1.0.0/vibe.toml`; `[[registry]] name='fixture', url='file://<absolute registry>'`. This is the existing directory-registry fixture, so ordinary product `--offline` is appropriate. No Git/network is required for the first vertical.
- `common::{slot_dir,slot_rel,deps_root,boot_dir,index_rel,static_md_rel,static_xml_rel}` bind assertions to current layout. For example `project.join(common::slot_dir("org.vibevm.oracle-extra", "1.0.0"))`.
- `cli_registry_mgmt.rs::install_with_features_records_active_features_in_lockfile` supplies actual install→typed Lockfile→slot assertions. `reinstall_regenerates_deleted_boot_artifacts` supplies the real command shape and deleted-boot control. `tree_fixture.rs::run_tree_json/by_id` demonstrates the public tree JSON shape, but its hand-written lock/boot fixture is **not** suitable evidence for this positive install test.
- Root link `static-transitive` is supported by current LinkType and exercised in `tree_fixture.rs`. `vibe-workspace/src/install/bootgen/transitive.rs` follows actual `ResolvedDep.requires`. Use this link to make dependency activation observable in generated boot, rather than assume a plain static root link includes all transitives.

## Minimal source fixtures

Create one isolated root with `registry`, separate projects `off` and `on`, `home`, and optional evidence directory. Create every manifest/source file before either install. Do not hand-write lock/slots/boot or invoke private resolver helpers to seed the expected answer.

Driver (`registry/org.vibevm/oracle-driver/v1.0.0/vibe.toml`):

```toml
[package]
group = "org.vibevm"
name = "oracle-driver"
version = "1.0.0"
kind = "flow"

[boot_snippet]
source = "boot/rules.md"
category = "flow"

[requires.packages]
"org.vibevm/oracle-base" = "^1"
"org.vibevm/oracle-extra" = { version = "^1", optional = true }

[[configuration.features]]
id = "enabled"
name = "Enable extra"
effects = [{ kind = "dependency", target = "org.vibevm/oracle-extra" }]

[[configuration.features]]
id = "disabled"
name = "No extra"

# This feature is never selected: its reference must NOT turn base optional.
[[configuration.features]]
id = "mention-required"
name = "Mention mandatory dependency"
effects = [{ kind = "dependency", target = "org.vibevm/oracle-base" }]

[[configuration.groups]]
mode = "choice"
id = "extra"
name = "Extra support"
cardinality = "exactly-one"
members = ["enabled", "disabled"]
defaults = ["disabled"]
```

Project `off/vibe.toml`:

```toml
[project]
name = "optional-off"
version = "0.1.0"

[requires.packages."org.vibevm/oracle-driver"]
version = "^1"
link = "static-transitive"
configuration = { policy = "require-selection", default_features = false, selections = { extra = { intent = "exact", members = ["disabled"] } } }

[[registry]]
name = "fixture"
url = "file://ABSOLUTE_REGISTRY"
```

For `on`, change project name and only the selected member to `enabled`. Use the ordinary TOML **`default_features`**, not the generated JSON spelling `defaultFeatures`. These group/effect/intent forms are already exercised by current `vibe-core/src/manifest/configuration/{tests,intent}.rs`. The only accepted-design addition not present in the inspected current source is inline **`optional=true`**: the present strict InlinePackageDepWire still rejects it. Do not claim this fixture already parses before primary's declaration patch lands; then assert ordinary `Manifest::parse_str` succeeds and `configuration.compile_choices()` succeeds.

For each leaf `oracle-base` and `oracle-extra`, use:

```toml
[package]
group = "org.vibevm"
name = "oracle-base" # oracle-extra in the second package
version = "1.0.0"
kind = "flow"

[boot_snippet]
source = "boot/rules.md"
category = "flow"

[hooks]
pre-install = "hooks/record"
```

Create each leaf's `boot/rules.md` with a distinct ASCII body marker (`ORACLE_BASE_LOADED`, `ORACLE_EXTRA_LOADED`), `payload.txt` with a distinct payload marker, and `hooks/record.sh`:

```sh
set -eu
printf '%s\n' 'oracle-base' >> "$VIBE_PROJECT_ROOT/effects.log"
```

Substitute the actual leaf name in the second script. `org.vibevm` uses the incumbent default trusted hook group; no new test-only hook seam. Driver gets its own `ORACLE_DRIVER_LOADED` boot marker and an `unused-source.txt` sentinel that must remain in its complete materialized slot in both cases. Seed ordinary AGENTS/CLAUDE/GEMINI launcher headers as the maintenance fixture does, so generated output is real.

## Exact invocations

Run through **a new `user.vibe()` child process each time** with the same isolated user home for that project:

```text
vibe install --path OFF --offline --assume-yes --json
vibe install --path ON  --offline --assume-yes --json
vibe tree --path OFF --json
vibe tree --path ON  --json
```

Use bare authored install, not `install oracle-extra`, not `--features` overriding the authored root, not app `-g`, and not a prepared host. Record exit/stdout/stderr. Successful install JSON should be parsed as the real install response; `ok:true` by itself is never the oracle.

Then run a cold second child process for each:

```text
vibe install --path OFF --offline --assume-yes --json
vibe install --path ON  --offline --assume-yes --json
```

Keep registry and source unchanged for this **first** cold gate; its purpose is durable state and real independent process replay, not to silently add source-unavailability semantics. An optional strengthening uses the already supported non-force reinstall: delete only generated INDEX/static artifact and invoke `vibe reinstall PROJECT --offline --assume-yes --json` (PROJECT is positional, not --path). Require reconstructed boot to preserve the selected graph and no hook repetition. Do not copy a saved boot file back into place.

## Assertions that discriminate false implementations

For each result read actual `vibe.lock` with `vibe_core::manifest::Lockfile::read`. Index packages by `(group,name)`; compare sets, not incidental solver order.

| Surface | OFF | ON |
|---|---|---|
| Actual locked package set | exactly driver + base | exactly driver + base + extra |
| Driver `LockedPackage.dependencies` | exactly base | exactly base + extra |
| Required base | locked, payload present, hook line present, boot marker present | same |
| Optional extra | no lock row, no slot directory, no hook line, no generated boot marker | lock row + correct payload + hook line + generated boot marker |
| Source materialization | driver's `unused-source.txt` survives unchanged | same |

Use `PackageRef` fields/qualified_name for dependency assertions, not Debug formatting. Current `LockedPackage.dependencies` is real at `lockfile.rs:378`; root presence alone cannot prove the owner edge is correct. `features` (`:421`) is only supporting evidence; a filled feature/effects field with extra still installed must fail OFF's lock/slot/hook assertions.

Read only **generated static lane(s)** (`common::static_md_rel()` / `static_xml_rel()` that actually exist) for markers. Do not search the entire project/registry: raw source bytes intentionally include inactive content. The static-transitive fixture makes base/extra output meaningful. Public `tree --json` (`packages[].id`, `load.type`, `load.in_static_md`) is a useful independent surface check, but is not a substitute for actual generated bytes and lock edges.

Count hook lines as a multiset, not execution order. OFF has one base invocation and no extra; ON has one each. Cold invocation preserves graph, payloads, original intent and counts—no repeated leaf effects. Prefer full lock byte equality when the response advertises unchanged; at minimum compare its complete semantic requested/resolved records and selected graph, without mistaking generated timestamps for authored meaning. Report Fresh/unchanged truthfully; do not require the response to say Fresh if the implementation safely replans.

**Durable intent oracle:** record the exact authored `ConfigurationIntentDecl` before running. Its policy=require-selection, default_features=false, exact group selection and original empty direct feature additions must survive in the actual package lock's requested record separately from resolved selected features. Effective feature `disabled`/`enabled` is not an original direct feature request. The rich package lock shape/API is still being designed by primary; do not invent a `package.configuration` JSON path or manufacture a test-owned persistence layer. Bind this assertion to the actual new reader/writer before accepting the positive gate. Authored vibe.toml remaining unchanged alone is insufficient.

A small high-value strengthening: change only `intent="exact"` to `intent="additive"` while keeping the same selected member/effects. A new ordinary invocation must retain the new original intent and cold replay must read it correctly, even though effective graph/features are identical. This catches persistence reconstructed solely from the resolved set. Treat it as a separate case if not in the first bounded gate.

## Mandatory control is already embedded

`oracle-base` is a normal string requires entry, and only the unselected `mention-required` feature names it. OFF must still install base and execute/load it. This catches the bad shortcut “every requires target mentioned by an effect is optional.” Repeat with inline `{version='^1', optional=false}` in the declaration reader/round-trip family; existing omission/string behavior is the stronger compatibility control here.

## Unrelated-owner weak case — separate forwarding obligation

Do not count forwarding complete merely because the optional vertical passes. Once primary enables the actual forwarding route, use the same directory registry and add:

- Driver feature `diagnostics` with `effects=[{kind='forward', target='org.vibevm/oracle-extra', feature='trace', strength='weak'}]`.
- Extra is now a configured package with public feature `trace`, whose dependency effect targets optional `org.vibevm/oracle-leaf`. Leaf has its own payload/hook/boot marker.
- Add package `oracle-other` with an ordinary **mandatory** requires edge to extra. Give that edge explicit child configuration `{policy='require-selection', default_features=false}`; extra has no required group. Driver's optional extra edge retains the same child configuration so default consent is not guessed on activation.
- Project requires both driver and other through actual authored declarations; driver's input selects `disabled` and directly enables `diagnostics`. Other is a real selected package/owner, not a synthetic domain label.

First install expectation: extra **is installed because other requires it**; other's lock edge includes extra; driver's lock edge does **not** include extra; trace leaf has no selected edge, slot, hook or boot marker. Extra's global presence must not activate driver's weak request. A check only asserting “extra exists” would miss this bug entirely.

In a separate fresh project, select driver's `enabled` plus `diagnostics`. Now driver's optional edge is active, weak trace must reach extra, and the actual trace leaf must be selected/materialized/executed/loaded. Assert actual owner edges and leaf effects. This catches both unconditional weak forwarding and permanently dropping weak requests. Use fresh projects to avoid making this discriminator depend on later removal/pruning behavior. Before forwarding is supported, keep its truthful typed refusal as an explicit boundary; do not weaken this positive scenario into a field-only test or call it passed.

## Unclosed details to bind, not guess

1. The new optional wire field and rich package lock record have not appeared in the inspected current types. Primary must supply their actual owner/API before these assertions compile; this memo intentionally provides no fake readers or guessed persistence key.
2. A new common-package fixture should get a narrow test target rather than import private fixtures from cli_common_package_maintenance or seed fake lock/boot. Existing common helpers above are callable today.
3. Boot dialect can vary; inspect the real generated static lane(s) through layout helpers. The first optional vertical does not establish every source mode, UI, forwarding or subskill route.
4. Run no provider/bootstrap host in the fixture. Missing real owner context must remain a typed refusal, not a synthetic answer supplied by the oracle.

The acceptance discriminator is the full OFF/ON graph + filesystem + actual hook + generated loading + durable-original/cold-process conjunction. No single filled DTO, successful exit, lock row or presence-only check can substitute for it.
