use super::*;
use crate::PhaseOutcome;
use std::fs;
struct HermeticEnvironment {
    prepares: AtomicUsize,
}
impl RegistryEnvironment for HermeticEnvironment {
    fn prepare(&self) -> anyhow::Result<RegistryEnvironmentSnapshot> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        Ok(RegistryEnvironmentSnapshot {
            embedded_root: None,
            global: Default::default(),
        })
    }
}
fn script(root: &Path) {
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(root.join("scripts/operation.sh"),r#"#!/bin/sh
python3 - <<'PY'
import os,json,pathlib,subprocess
context=json.load(open(os.environ['VIBE_CONTEXT']));root=pathlib.Path(context['project']['root']);op=context['execution']['id']
with (root/'calls').open('a') as out:out.write(op+'\n')
if op=='produce':
 (root/'src').mkdir(exist_ok=True);(root/'src/product.rs').write_text('pub fn answer()->u32{42}\n#[test] fn value(){assert_eq!(answer(),42);}\n');out=root/'src/product.rs';ident='source'
elif op=='compile':
 (root/'target').mkdir(exist_ok=True);out=root/'target/check';env=dict(os.environ,RUSTUP_AUTO_INSTALL='0',CARGO_NET_OFFLINE='true');subprocess.run(['rustc','--edition=2024','--test',str(root/'src/product.rs'),'-o',str(out)],check=True,env=env);ident='compiled'
elif op=='observe':
 result=subprocess.run([str(root/'target/check')],check=True,capture_output=True);(root/'evidence').mkdir(exist_ok=True);out=root/'evidence/stdout';out.write_bytes(result.stdout);ident='evidence'
else:raise RuntimeError(op)
json.dump({'envelope':1,'status':'ok','tasks':[],'artifacts':[{'id':ident,'kind':'file','path':str(out)}]},open(os.environ['VIBE_REPLY'],'w'))
PY
"#).unwrap();
}
fn authored(root: &Path, extra: &str) {
    let mut text = String::from("[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n");
    for (id, phase, effect, input) in [
        ("produce", "create", "product-source", "intent.txt"),
        ("compile", "build", "build-artifact", "src/product.rs"),
        ("observe", "verify", "evidence-observer", "src/product.rs"),
    ] {
        text.push_str(&format!("\n[[extension]]\nid='{id}'\npoint='phase:{phase}'\neffects=['{effect}']\ninputs=['{input}']\nhandler={{kind='script',base='scripts/operation'}}\n"));
    }
    text.push_str(extra);
    fs::write(root.join("vibe.toml"), text).unwrap();
    fs::write(
        root.join("intent.txt"),
        "stable explicit production intent\n",
    )
    .unwrap();
    script(root);
}
fn invoke(root: &Path, phase: Phase) -> (PhaseOutcome, usize, usize) {
    invoke_with(root, phase, false)
}
fn invoke_with(root: &Path, phase: Phase, full: bool) -> (PhaseOutcome, usize, usize) {
    let lease = lease_default_lifecycle(root).unwrap();
    let prepared = if full {
        crate::prepare_full_pass_lifecycle(lease, request(phase)).unwrap()
    } else {
        prepare_default_lifecycle(lease, request(phase)).unwrap()
    };
    let environment = HermeticEnvironment {
        prepares: AtomicUsize::new(0),
    };
    let sources = CountingFactory::default();
    let outcome = prepared.run(
        DefaultLifecyclePorts {
            observer: &SilentRun,
            install_observer: &SilentInstall,
            confirm_gate: &AlwaysYes,
            sources: &sources,
            environment: &environment,
            manifest_mutation: &NoManifestMutation,
            agent: Arc::new(vibe_lifecycle::NoAgentBackend),
        },
        None,
        vibe_core::manifest::TargetOs::Linux,
        "2026-10-13T08:30:00Z".parse().unwrap(),
        None,
    );
    (
        outcome,
        environment.prepares.load(Ordering::SeqCst),
        sources.builds(),
    )
}
#[test]
fn public_create_build_verify_use_current_prefix_without_install_or_another_producer() {
    let root = empty_project();
    authored(root.path(), "");
    let (create, _, sources) = invoke(root.path(), Phase::Create);
    assert!(matches!(create, PhaseOutcome::Completed(_)), "{create:?}");
    assert_eq!(sources, 0);
    let source = fs::read(root.path().join("src/product.rs")).unwrap();
    for phase in [Phase::Build, Phase::Verify] {
        let (outcome, environment, sources) = invoke(root.path(), phase);
        assert!(matches!(outcome, PhaseOutcome::Completed(_)), "{outcome:?}");
        assert_eq!(environment, 0);
        assert_eq!(sources, 0);
    }
    assert_eq!(
        fs::read_to_string(root.path().join("calls")).unwrap(),
        "produce\ncompile\nobserve\n"
    );
    assert_eq!(
        source,
        fs::read(root.path().join("src/product.rs")).unwrap()
    );
    assert!(
        fs::read_to_string(root.path().join("evidence/stdout"))
            .unwrap()
            .contains("1 passed")
    );
}
#[test]
fn hidden_slot_producer_and_unknown_compile_refuse_before_ports_or_handlers() {
    for (point, effects, code) in [
        (
            "slot:pre-install",
            "effects=['product-source']\n",
            "creation-required",
        ),
        ("compile:emitted", "", "unsupported-effect-classification"),
    ] {
        let root = empty_project();
        let handler = if point.starts_with("compile") {
            "handler={kind='builtin',name='log'}"
        } else {
            "handler={kind='script',base='scripts/operation'}"
        };
        authored(
            root.path(),
            &format!("\n[[extension]]\nid='hidden'\npoint='{point}'\n{effects}{handler}\n"),
        );
        if point.starts_with("slot") {
            admit_package(root.path(), false);
        }
        let (outcome, environment, sources) = invoke(root.path(), Phase::Verify);
        let PhaseOutcome::Failed { original, .. } = outcome else {
            panic!("expected precise failure")
        };
        assert!(format!("{original:#}").contains(code), "{original:#}");
        if point.starts_with("slot") {
            assert!(
                format!("{original:#}").contains("hidden@slot(org.demo/dep@1.0.0)"),
                "{original:#}"
            );
        }

        assert_eq!(environment, 0);
        assert_eq!(sources, 0);
        assert!(!root.path().join("calls").exists());
        assert!(!root.path().join("src").exists());
    }
}

#[test]
fn scoped_member_build_verify_reuse_only_its_completed_production() {
    let (workspace, member) = workspace_with_member();
    authored(&member, "");
    let (create, _, _) = invoke(&member, Phase::Create);
    assert!(matches!(create, PhaseOutcome::Completed(_)), "{create:?}");
    let source = fs::read(member.join("src/product.rs")).unwrap();
    for phase in [Phase::Build, Phase::Verify] {
        let (outcome, environment, providers) = invoke(&member, phase);
        assert!(matches!(outcome, PhaseOutcome::Completed(_)), "{outcome:?}");
        assert_eq!((environment, providers), (0, 0));
    }
    assert_eq!(
        fs::read_to_string(member.join("calls")).unwrap(),
        "produce\ncompile\nobserve\n"
    );
    assert_eq!(source, fs::read(member.join("src/product.rs")).unwrap());
    assert!(!workspace.path().join("src").exists());
    assert!(!workspace.path().join("target").exists());
}

#[test]
fn explicit_shared_full_pass_causally_produces_compiles_and_observes() {
    let root = empty_project();
    authored(root.path(), "");
    let (outcome, _, providers) = invoke_with(root.path(), Phase::Verify, true);
    assert!(matches!(outcome, PhaseOutcome::Completed(_)), "{outcome:?}");
    assert_eq!(providers, 0);
    assert_eq!(
        fs::read_to_string(root.path().join("calls")).unwrap(),
        "produce\ncompile\nobserve\n"
    );
    assert!(
        fs::read_to_string(root.path().join("evidence/stdout"))
            .unwrap()
            .contains("1 passed")
    );
}

#[test]
fn recognized_legacy_success_is_not_relabelled_or_used_as_original_subject() {
    let root = empty_project();
    authored(root.path(), "");
    let (outcome, _, _) = invoke(root.path(), Phase::Create);
    assert!(matches!(outcome, PhaseOutcome::Completed(_)), "{outcome:?}");
    // Owned setup projects the successful record back to the exact old shape:
    // it has input/output witnesses, but no original selected-subject basis.
    let mut state = vibe_lifecycle::LifecycleStateStore::peek(root.path())
        .unwrap()
        .unwrap();
    state.schema = 1;
    for row in state.execution.values_mut() {
        row.completion_basis = None;
    }
    let raw = toml::to_string(&state).unwrap();
    let old: vibe_wire::generated::lifecycle_state::LifecycleState = toml::from_str(&raw).unwrap();
    assert_eq!(old.schema, 1);
    let path = root.path().join(vibe_lifecycle::LifecycleStateStore::FILE);
    fs::write(&path, raw.as_bytes()).unwrap();
    let before = fs::read(&path).unwrap();
    let source = fs::read(root.path().join("src/product.rs")).unwrap();
    let (outcome, environment, providers) = invoke(root.path(), Phase::Build);
    let PhaseOutcome::Failed { original, .. } = outcome else {
        panic!("legacy row became current proof")
    };
    assert!(
        format!("{original:#}").contains("original-basis-unavailable"),
        "{original:#}"
    );
    assert_eq!((environment, providers), (0, 0));
    assert_eq!(before, fs::read(&path).unwrap());
    assert_eq!(
        source,
        fs::read(root.path().join("src/product.rs")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(root.path().join("calls")).unwrap(),
        "produce\n"
    );
}

fn admit_package(root: &Path, legacy_hook: bool) {
    use vibe_core::{
        ContentHash,
        manifest::{Lockfile, SpecFormat},
    };
    let source = root.join("fixture-original");
    fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
    let hook = if legacy_hook {
        "[hooks]\npre-install='scripts/operation'\n"
    } else {
        ""
    };
    fs::write(source.join("vibe.toml"),format!("[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n{hook}[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n")).unwrap();
    fs::write(
        source.join("vibevm/vibespecs/DOC.md"),
        "# Retained\n\n@fact:ONE Native fact.\n",
    )
    .unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    vibe_workspace::vibedeps::materialise_with_spec_format(
        root,
        &"org.demo".parse().unwrap(),
        "dep",
        &"1.0.0".parse().unwrap(),
        &source,
        vibe_workspace::vibedeps::CopyMode::Copy,
        SpecFormat::Markdown,
        &hash,
    )
    .unwrap();
    let mut lock = Lockfile::empty("owned-fixture", "2026-10-13T08:00:00Z");
    lock.packages.push(toml::from_str(&format!("kind='flow'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='{hash}'\n")).unwrap());
    lock.write(root.join("vibe.lock")).unwrap();
    let path = root.join("vibe.toml");
    let mut text = fs::read_to_string(&path).unwrap();
    text.push_str("\n[requires.packages]\n'flow:org.demo/dep'='=1.0.0'\n");
    fs::write(path, text).unwrap();
}

#[test]
fn captured_legacy_slot_hook_is_unknown_before_any_install_or_handler_effect() {
    let root = empty_project();
    authored(root.path(), "");
    admit_package(root.path(), true);
    let before = fs::read(root.path().join("vibe.lock")).unwrap();
    let (outcome, environment, providers) = invoke(root.path(), Phase::Verify);
    let PhaseOutcome::Failed { original, .. } = outcome else {
        panic!("unclassified legacy hook was admitted")
    };
    let diagnostic = format!("{original:#}");
    assert!(
        diagnostic.contains("unsupported-effect-classification"),
        "{diagnostic}"
    );
    assert!(
        diagnostic.contains("@slot(org.demo/dep@1.0.0)"),
        "{diagnostic}"
    );
    assert_eq!((environment, providers), (0, 0));
    assert!(!root.path().join("calls").exists());
    assert!(!root.path().join("src").exists());
    assert_eq!(before, fs::read(root.path().join("vibe.lock")).unwrap());
}

#[test]
fn complete_prefix_refuses_generate_and_mixed_verify_producers_for_ordinary_and_scoped_calls() {
    for scoped in [false, true] {
        for point in ["phase:generate", "phase:verify"] {
            let (workspace, member) = workspace_with_member();
            let root = if scoped {
                member.as_path()
            } else {
                workspace.path()
            };
            authored(
                root,
                &format!(
                    "\n[[extension]]\nid='hidden'\npoint='{point}'\neffects=['evidence-observer','product-source']\ninputs=[]\nhandler={{kind='script',base='scripts/operation'}}\n"
                ),
            );
            if point == "phase:verify" {
                for phase in [Phase::Create, Phase::Build] {
                    let (outcome, _, _) = invoke(root, phase);
                    assert!(matches!(outcome, PhaseOutcome::Completed(_)), "{outcome:?}");
                }
            }
            let prior_calls = fs::read(root.join("calls")).ok();
            let prior_source = fs::read(root.join("src/product.rs")).ok();
            for phase in [Phase::Build, Phase::Verify] {
                if point == "phase:verify" && phase == Phase::Build {
                    continue;
                }
                let (outcome, environment, providers) = invoke(root, phase);
                let PhaseOutcome::Failed { original, .. } = outcome else {
                    panic!("source prerequisite dispatched")
                };
                let diagnostic = format!("{original:#}");
                assert!(diagnostic.contains("creation-required"), "{diagnostic}");
                assert_eq!((environment, providers), (0, 0));
                assert!(diagnostic.contains("consumer#hidden"), "{diagnostic}");
                assert_eq!(prior_calls, fs::read(root.join("calls")).ok());
                assert_eq!(prior_source, fs::read(root.join("src/product.rs")).ok());
            }
        }
    }
}

#[test]
fn unselected_phase_rows_and_build_target_do_not_veto_selected_validation() {
    let root = empty_project();
    authored(
        root.path(),
        "\n[[extension]]\nid='unselected'\npoint='phase:verify'\nhandler={kind='script',base='scripts/operation'}\n[[artifacts.build]]\nid='unselected-build'\nmechanism='build:missing'\noutputs=[{id='unused-output',kind='file'}]\n",
    );
    let (outcome, environment, providers) = invoke(root.path(), Phase::Validate);
    assert!(matches!(outcome, PhaseOutcome::Completed(_)), "{outcome:?}");
    assert_eq!((environment, providers), (0, 0));
    assert!(!root.path().join("calls").exists());
    assert!(!root.path().join("src").exists());
}

#[test]
fn changed_declared_constraint_refuses_current_prefix_without_hydration_or_slot_edits() {
    fn tree(path: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
        fn visit(
            base: &Path,
            path: &Path,
            rows: &mut std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>>,
        ) {
            for item in fs::read_dir(path).unwrap() {
                let path = item.unwrap().path();
                let key = path.strip_prefix(base).unwrap().to_path_buf();
                if path.is_dir() {
                    rows.insert(key, None);
                    visit(base, &path, rows);
                } else {
                    rows.insert(key, Some(fs::read(path).unwrap()));
                }
            }
        }
        let mut rows = Default::default();
        visit(path, path, &mut rows);
        rows
    }
    let root = empty_project();
    authored(root.path(), "");
    admit_package(root.path(), false);
    let manifest = root.path().join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    assert_eq!(text.matches("'=1.0.0'").count(), 1);
    fs::write(&manifest, text.replace("'=1.0.0'", "'=2.0.0'")).unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(
        root.path(),
        &"org.demo".parse().unwrap(),
        "dep",
        &"1.0.0".parse().unwrap(),
    );
    let before = tree(&slot);
    let lock = fs::read(root.path().join("vibe.lock")).unwrap();
    let (outcome, environment, providers) = invoke(root.path(), Phase::Verify);
    let PhaseOutcome::Failed { original, .. } = outcome else {
        panic!("current constraint mismatch was ignored")
    };
    let error = format!("{original:#}");
    assert!(
        error.contains("preparation-required: current selected world is unavailable"),
        "{error}"
    );
    assert!(error.contains("selected requirement version"), "{error}");
    assert_eq!((environment, providers), (0, 0));
    assert!(!root.path().join("calls").exists());
    assert_eq!(before, tree(&slot));
    assert_eq!(lock, fs::read(root.path().join("vibe.lock")).unwrap());
    assert!(
        !root
            .path()
            .join(vibe_lifecycle::LifecycleStateStore::FILE)
            .exists()
    );
}

#[test]
fn implicit_native_compiler_route_with_unknown_effects_refuses_before_preparation() {
    let root = empty_project();
    fs::write(root.path().join("vibe.toml"),"[package]\ngroup='org.demo'\nname='consumer'\nkind='tool'\nversion='1.0.0'\n[[extension]]\nid='produce'\npoint='phase:create'\neffects=['product-source']\ninputs=[]\nhandler={kind='native',crate_dir='crates/not-built'}\n[[mechanism]]\nid='builder'\nrole='build'\nname='cargo'\nhandler={kind='script',base='scripts/should-not-run'}\nprotocol=1\nconfig_schema='schema.jtd.json'\nfreshness='provider'\n[mechanisms]\n'build:cargo'='org.demo/consumer#builder'\n").unwrap();
    fs::write(root.path().join("schema.jtd.json"), "{}").unwrap();
    fs::create_dir_all(root.path().join("scripts")).unwrap();
    fs::write(
        root.path().join("scripts/should-not-run.sh"),
        "#!/bin/sh\ntouch COMPILER-CALLED\nexit 1\n",
    )
    .unwrap();
    fs::create_dir_all(root.path().join("crates/not-built/src")).unwrap();
    fs::write(
        root.path().join("crates/not-built/Cargo.toml"),
        "[package]\nname='not-built'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    fs::write(
        root.path().join("crates/not-built/src/lib.rs"),
        "pub fn fixture(){}\n",
    )
    .unwrap();
    let (outcome, environment, providers) = invoke(root.path(), Phase::Create);
    let PhaseOutcome::Failed { original, .. } = outcome else {
        panic!("unknown implicit compiler was admitted")
    };
    let error = format!("{original:#}");
    assert!(
        error.contains("unsupported-effect-classification"),
        "{error}"
    );
    assert!(error.contains("org.demo/consumer#builder"), "{error}");
    assert_eq!((environment, providers), (0, 0));
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("src").exists());
    assert!(!root.path().join("COMPILER-CALLED").exists());
}
