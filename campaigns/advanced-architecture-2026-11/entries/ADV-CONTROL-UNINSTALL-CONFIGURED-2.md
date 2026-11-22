# ADV-CONTROL-UNINSTALL-CONFIGURED-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe uninstall org.vibevm/oracle-extra`, when oracle-extra is not declared in the node and the configured owner org.vibevm/oracle-driver selects it through feature `enabled`, exits 1 before any write with a message naming the owner, the feature and the owner's configuration table; when oracle-extra is also declared directly, uninstall still drops the declaration and retains the package as today.

**Write.** `crates/vibe-cli/src/commands/uninstall.rs`, `crates/vibe-cli/tests/cli_common_package_optional/boundaries.rs`, `vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml`

**Check.** `cargo test -p vibe-cli --test cli_common_package_optional configured_uninstall_selected_dependency 2>&1 | grep -F 'test result: ok. 2 passed'`

**Test.** crates/vibe-cli/tests/cli_common_package_optional/boundaries.rs: (a) `configured_uninstall_selected_dependency_refuses_naming_owner_and_feature`: Fixture::new(true); fixture.install_ok(); save bytes of project vibe.lock and vibe.toml and fixture.static_boot(); output = uninstall(&fixture, EXTRA); diagnostic = stdout+stderr; assert_eq!(output.status.code(), Some(1)); assert diagnostic contains "is selected by configured package `org.vibevm/oracle-driver`" and "(feature `enabled`)"; assert vibe.lock bytes, vibe.toml bytes and static_boot unchanged; assert fixture.slot(EXTRA).exists(). Fails today: the generic "pure transitive dependency" refusal fires instead. (b) `configured_uninstall_selected_dependency_declared_root_is_retained`: Fixture::new(true); add_root(&fixture, EXTRA, None); fixture.install_ok(); configuration = fixture.lock().configurations; output = uninstall(&fixture, EXTRA); assert output.status.success() (print stdout/stderr on failure); assert no Manifest::read(project vibe.toml).requires.iter_pkgrefs() item has name EXTRA; assert fixture.slot(EXTRA).exists(); assert fixture.lock().find(GROUP, EXTRA).is_some(); assert_eq!(fixture.lock().configurations, configuration). (b) passes today and guards that the new refusal does not swallow the directly-declared case; the check requires exactly 2 passing tests.

**Read first.**
- crates/vibe-cli/src/commands/uninstall.rs:43-90
- crates/vibe-cli/src/commands/uninstall/plan.rs:93-104
- crates/vibe-cli/tests/cli_common_package_optional/boundaries.rs:1-40
- crates/vibe-cli/tests/cli_common_package_optional.rs:15-118
- crates/vibe-core/src/manifest/configuration/package.rs:21-40

**Steps.**
1. Write tests (a) and (b) first at the end of boundaries.rs, each with attribute #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#UNINSTALL-CLOSURE")]. Run the check and confirm it does NOT print `2 passed` (test (a) fails) before touching uninstall.rs. If test (b) fails at its own fixture.install_ok(), stop and report; do not alter the fixture.
2. In uninstall.rs, immediately after `let selected_slot = plan::slot(locked);` and before `let plan = plan::build(`, add: let target = format!("{group}/{}", pkgref.name); let declared = manifest.requires.iter_pkgrefs().any(|(g, n)| g == Some(group.as_str()) && n == pkgref.name) (match the types the same way plan.rs:93-96 does).
3. Only when `!declared`: find the first (BTreeMap order) entry of lockfile.configurations where owner != target, record.active_dependencies contains target, and record.dependency_causes.get(&target) is Some and non-empty; take (owner.clone(), causes.join("`, `")). If found, `bail!` with exactly: "`{target}` is selected by configured package `{owner}` (feature `{features}`); turn that feature off in [requires.packages.\"{owner}\".configuration] or uninstall `{owner}`".
4. Make the spec_edit below in PROP-009-loading-model.xml exactly as written; change no other unit.
5. Run `cargo fmt -p vibe-cli` then the check.

**Decisions already made.**
- The check runs before plan::build and only for a target NOT declared in this node's requires — exactly the case that would otherwise hit the pure-transitive refusal at plan.rs:102 — so it replaces that generic message with an actionable one and changes no other behaviour.
- A target that is declared directly AND selected by a feature keeps today's behaviour (PROP-009 UNINSTALL-CLOSURE: removing a declaration still required elsewhere retains the package and reports that fact): the declaration is dropped, the package stays, the owner's record is untouched. Test (b) pins this.
- Only feature-caused selections (non-empty dependency_causes) refuse here; an owner's mandatory dependency (empty causes, e.g. oracle-base) falls through to today's ordinary pure-transitive refusal.
- Plain anyhow `bail!` (exit code GENERAL = 1), not PackageConfigurationError::Unsupported; nothing is written.
- The configuration table named in the message is `[requires.packages."<owner>".configuration]`, because that is where the authored configuration input lives (there is no [requires.configurations] TOML table).
- Several features are joined with "`, `"; with several owners the first in lock order is named.
- Test (b) uses add_root(..., None) on purpose: no requires.optional record for oracle-extra.
- The worker makes the spec edit (PROP-009 is in the write area); the unit text is fixed below.
- Probe result (planner ran the unrestricted variant in a copy): test (a) passed. Test (b) and the `!declared` restriction were not probed.

**Not in this entry.** Offering to turn the feature off automatically, a --force path, refusing the directly-declared case, or changing behaviour for mandatory (non-feature) dependencies.

**Depends on.** ADV-CONTROL-UNINSTALL-CONFIGURED-1

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml, unit UNINSTALL-CLOSURE: append before `</UNINSTALL-CLOSURE>` the sentence: " Uninstalling an undeclared package that a retained configured package selected through a feature refuses before any write, naming the owner, the feature and the owner's configuration table to change."

Entries that write the same file run one after another, never in parallel.
