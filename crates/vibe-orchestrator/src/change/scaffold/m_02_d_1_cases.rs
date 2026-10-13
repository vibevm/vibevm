use super::*;
use crate::document_analysis::*;
use std::fs;
use vibe_core::{
    ContentHash,
    manifest::{Lockfile, SpecFormat},
};
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
const PIN: &str = "flow:org.demo/dep@=1.0.0";
const ADDRESS: &str = "spec://org.demo/dep/DOC#DELIVERY";
const BODY: &str = "# D {#root}\n\n@fact:DELIVERY Deliver here. @status:spec\n";
pub(super) struct Fixture {
    pub root: tempfile::TempDir,
    pub host: Arc<DocumentAnalysisHost>,
    pub authority: Arc<ScaffoldAuthority>,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let host = root.path().join("host");
        fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
        fs::create_dir(&host).unwrap();
        fs::write(host.join("vibe.toml"),"[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'flow:org.demo/dep'='=1.0.0'\n").unwrap();
        fs::write(source.join("vibe.toml"),"[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
        fs::write(source.join("vibevm/vibespecs/DOC.md"), BODY).unwrap();
        let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
        vibe_workspace::vibedeps::materialise_with_spec_format(
            &host,
            &"org.demo".parse().unwrap(),
            "dep",
            &"1.0.0".parse().unwrap(),
            &source,
            vibe_workspace::vibedeps::CopyMode::Copy,
            SpecFormat::Mixed,
            &hash,
        )
        .unwrap();
        let mut lock = Lockfile::empty("fixture", "fixture");
        lock.packages.push(toml::from_str(&format!("kind='flow'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='{hash}'\n")).unwrap());
        lock.write(host.join("vibe.lock")).unwrap();
        let directory = PathBuf::from(format!("docs/custom/changes/{ID}"));
        let mut declarations = vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "vibe.lock".into(),
                role: ResourceRole::Lockfile,
            },
        ];
        for name in ["change.toml", "intent.md", "refinement.md", "acceptance.md"] {
            declarations.push(ResourceDeclaration {
                path: format!("{}/{name}", directory.display()),
                role: ResourceRole::Opaque,
            });
        }
        let held = DocumentAnalysisHost::new(
            &host,
            "vibe.toml",
            declarations,
            AnalysisLimits {
                resource_bytes: 4 * 1024 * 1024,
                capture_bytes: 16 * 1024 * 1024,
                open_buffers: 8,
            },
        )
        .unwrap();
        let authority =
            ScaffoldAuthority::new(held.clone(), held.resource("vibe.toml").unwrap(), directory)
                .unwrap();
        Self {
            root,
            host: held,
            authority,
        }
    }
    fn prepare(&self, capture: &AnalysisCapture) -> Result<PreparedScaffold, ScaffoldRefusal> {
        prepare_new_change(
            &self.authority,
            capture,
            PIN,
            ID,
            "local",
            "Local delivery",
            Some(Path::new("docs/custom")),
            &[],
        )
    }
}
#[test]
fn exact_saved_scaffold_creates_prepared_local_roles() {
    let fixture = Fixture::new();
    let client = fixture.host.client().unwrap();
    let other = fixture.host.client().unwrap();
    let capture = client.freeze().unwrap();
    let plan = fixture.prepare(&capture).unwrap();
    assert_eq!(plan.output().selected_addresses, [ADDRESS]);
    assert_eq!(
        plan.output().analysis.manifest.origins()[0]
            .selected_content_hash
            .as_deref(),
        Some(format!("sha256:{}", "a".repeat(64)).as_str())
    );
    let files = plan.output().files.clone();
    let directory = fixture.host.root().join(&plan.output().directory);
    assert!(!directory.exists());
    client
        .acknowledge_absent_edit(plan.barrier(), true)
        .unwrap();
    other.acknowledge_absent_edit(plan.barrier(), true).unwrap();
    let id = plan.id().to_string();
    let outcome = plan.apply().unwrap();
    assert!(
        matches!(outcome.state, ScaffoldState::Completed),
        "{outcome:?}"
    );
    assert_eq!(outcome.verified.len(), 4);
    assert!(outcome.uncertain.is_empty());
    assert!(outcome.pending.is_empty());
    for (name, bytes) in files {
        assert_eq!(fs::read(directory.join(name)).unwrap(), bytes);
    }
    assert!(
        fs::read_to_string(directory.join("intent.md"))
            .unwrap()
            .contains(&format!("@{ADDRESS}"))
    );
    assert!(
        !fs::read_to_string(directory.join("acceptance.md"))
            .unwrap()
            .contains("impl/done")
    );
    assert_eq!(
        fs::read(fixture.root.path().join("source/vibevm/vibespecs/DOC.md")).unwrap(),
        BODY.as_bytes()
    );
    assert!(matches!(
        fixture.authority.inspect_recovery(&id).unwrap().state,
        ScaffoldState::Completed
    ));
    println!(
        "SCAFFOLD_OUTCOME {}",
        serde_json::to_string(&outcome).unwrap()
    );
}

fn tree(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut rows = vec![];
    let mut stack = vec![root.to_owned()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path.clone());
                rows.push((path.strip_prefix(root).unwrap().into(), None));
            } else {
                rows.push((
                    path.strip_prefix(root).unwrap().into(),
                    Some(fs::read(&path).unwrap()),
                ));
            }
        }
    }
    rows.sort();
    rows
}
fn preserved(before: &[(PathBuf, Option<Vec<u8>>)], root: &Path) {
    let after = tree(root);
    assert_eq!(before, after);
    println!(
        "SCAFFOLD_SNAPSHOT_PAIR {}",
        serde_json::json!({"root":root,"before":before,"after":after})
    );
}

#[test]
fn native_registered_journal_corpus_uses_actual_strict_reader() {
    let valid =
        include_bytes!("../../../../../formats/corpora/change-scaffold-journal/e1/valid.toml");
    let row = journal::decode_contract(valid).unwrap();
    assert_eq!(row.files.len(), 4);
    assert!(!row.completed);
    for raw in [
        include_bytes!(
            "../../../../../formats/corpora/change-scaffold-journal/e1/unknown-field.toml"
        )
        .as_slice(),
        include_bytes!(
            "../../../../../formats/corpora/change-scaffold-journal/e1/unknown-epoch.toml"
        )
        .as_slice(),
    ] {
        assert!(matches!(
            journal::decode_contract(raw),
            Err(ScaffoldRefusal::Journal)
        ));
    }
    let encoded = toml::to_string(&row).unwrap();
    assert_eq!(
        journal::decode_contract(encoded.as_bytes()).unwrap().id,
        row.id
    );
}
#[test]
fn occupied_dirty_stale_origin_and_versions_refuse_before_creation() {
    for mode in [
        "occupied-empty",
        "dirty-create",
        "dirty-node-saved",
        "late-version",
        "stale-node",
        "stale-origin",
    ] {
        let fixture = Fixture::new();
        let client = fixture.host.client().unwrap();
        let other = fixture.host.client().unwrap();
        let target = fixture.host.root().join(&fixture.authority.directory);
        if mode == "occupied-empty" {
            fs::create_dir_all(&target).unwrap();
        }
        if mode == "dirty-create" {
            let id = fixture.authority.resources["intent.md"];
            other
                .open(
                    1,
                    id,
                    1,
                    ByteEncoding::Utf8,
                    Arc::from(b"private dirty body".as_slice()),
                    BufferIntent::Create,
                )
                .unwrap();
        }
        if mode == "dirty-node-saved" {
            let bytes = fs::read(fixture.host.root().join("vibe.toml")).unwrap();
            let mut dirty = bytes.clone();
            dirty.extend_from_slice(b"# unsaved\n");
            client
                .open(
                    1,
                    fixture.authority.node,
                    1,
                    ByteEncoding::Utf8,
                    Arc::from(dirty.clone()),
                    BufferIntent::Replace,
                )
                .unwrap();
            let capture = client.freeze().unwrap();
            fs::write(fixture.host.root().join("vibe.toml"), dirty).unwrap();
            let before = tree(fixture.root.path());
            assert!(matches!(
                fixture.prepare(&capture),
                Err(ScaffoldRefusal::Edit(EditError::SaveRequired))
            ));
            preserved(&before, fixture.root.path());
            continue;
        }
        let capture = client.freeze().unwrap();
        if mode == "occupied-empty" || mode == "dirty-create" {
            let before = tree(fixture.root.path());
            let error = fixture.prepare(&capture).unwrap_err();
            if mode == "occupied-empty" {
                assert!(matches!(error, ScaffoldRefusal::Occupied));
            } else {
                assert!(matches!(
                    error,
                    ScaffoldRefusal::Edit(EditError::SaveRequired)
                ));
                assert!(!error.to_string().contains("private dirty body"));
            }
            preserved(&before, fixture.root.path());
            continue;
        }
        let plan = fixture.prepare(&capture).unwrap();
        if mode == "late-version" {
            let saved = fs::read(fixture.host.root().join("vibe.toml")).unwrap();
            other
                .open(
                    1,
                    fixture.authority.node,
                    2,
                    ByteEncoding::Utf8,
                    Arc::from(saved),
                    BufferIntent::Replace,
                )
                .unwrap();
            let before = tree(fixture.root.path());
            assert_eq!(
                other
                    .acknowledge_absent_edit(plan.barrier(), true)
                    .unwrap_err(),
                EditError::Stale
            );
            preserved(&before, fixture.root.path());
        } else {
            client
                .acknowledge_absent_edit(plan.barrier(), true)
                .unwrap();
            other.acknowledge_absent_edit(plan.barrier(), true).unwrap();
            let path = fixture.host.root().join(if mode == "stale-node" {
                "vibe.toml"
            } else {
                "vibe.lock"
            });
            let text = fs::read_to_string(&path).unwrap();
            let old = format!("sha256:{}", "a".repeat(64));
            let new = format!("sha256:{}", "b".repeat(64));
            if mode == "stale-node" {
                fs::write(path, format!("{text}# disk node changed\n")).unwrap();
            } else {
                assert_eq!(text.matches(&old).count(), 1);
                fs::write(path, text.replacen(&old, &new, 1)).unwrap();
            }
            let before = tree(fixture.root.path());
            let result = plan.apply();
            assert!(
                if mode == "stale-node" {
                    matches!(result, Err(ScaffoldRefusal::Edit(EditError::Stale)))
                } else {
                    matches!(result, Err(ScaffoldRefusal::Origin))
                },
                "actual stale-origin outcome: {result:?}"
            );
            preserved(&before, fixture.root.path());
        }
        assert!(!target.exists());
        println!("SCAFFOLD_REFUSAL {mode}");
    }
}
#[test]
fn landed_partial_reconciliation_preserves_third_state_and_never_replays() {
    // A partial parent mkdir can leave the target absent. A fresh capture
    // must still attach/refuse the retained operation, never start a new one.
    let early = Fixture::new();
    let early_client = early.host.client().unwrap();
    let first = early_client.freeze().unwrap();
    let first_plan = early.prepare(&first).unwrap();
    early_client
        .acknowledge_absent_edit(first_plan.barrier(), true)
        .unwrap();
    vibe_safefs::arm_after_create_dir(Some(Box::new(|_, name| {
        if name == "docs" {
            Some(std::io::Error::other("parent created but unsealed"))
        } else {
            None
        }
    })));
    let partial = first_plan.apply().unwrap();
    assert!(matches!(partial.state, ScaffoldState::Partial));
    assert!(!early.host.root().join(&early.authority.directory).exists());
    let fresh = early_client.freeze().unwrap();
    let retry = early.prepare(&fresh).unwrap();
    early_client
        .acknowledge_absent_edit(retry.barrier(), true)
        .unwrap();
    let before = tree(early.root.path());
    let repeated = retry.apply();
    assert!(
        matches!(repeated, Err(ScaffoldRefusal::Journal)),
        "fresh capture reissued partial operation: {repeated:?}"
    );
    preserved(&before, early.root.path());
    let fixture = Fixture::new();
    let client = fixture.host.client().unwrap();
    let capture = client.freeze().unwrap();
    let plan = fixture.prepare(&capture).unwrap();
    let id = plan.id().to_string();
    let desired = plan.output().files.clone();
    client
        .acknowledge_absent_edit(plan.barrier(), true)
        .unwrap();
    vibe_safefs::fail_after_publish(Some("change.toml"));
    let result = plan.apply();
    vibe_safefs::fail_after_publish(None);
    let outcome = result.unwrap();
    assert!(matches!(outcome.state, ScaffoldState::Partial));
    assert_eq!(outcome.verified.len(), 1);
    assert_eq!(outcome.uncertain.len(), 1);
    assert_eq!(outcome.pending.len(), 2);
    let target = fixture.host.root().join(&fixture.authority.directory);
    assert_eq!(
        fs::read(target.join("change.toml")).unwrap(),
        desired["change.toml"]
    );
    assert!(!target.join("intent.md").exists());
    let before = tree(fixture.root.path());
    let reconciled = fixture.authority.inspect_recovery(&id).unwrap();
    assert!(matches!(reconciled.state, ScaffoldState::Partial));
    assert_eq!(reconciled.uncertain, outcome.uncertain);
    preserved(&before, fixture.root.path());
    let fresh = client.freeze().unwrap();
    assert!(matches!(
        fixture.prepare(&fresh),
        Err(ScaffoldRefusal::Occupied)
    ));
    preserved(&before, fixture.root.path());
    fs::write(target.join("acceptance.md"), "unrelated user change").unwrap();
    let before = tree(fixture.root.path());
    assert!(matches!(
        fixture.authority.inspect_recovery(&id),
        Err(ScaffoldRefusal::ThirdState)
    ));
    preserved(&before, fixture.root.path());
    println!(
        "SCAFFOLD_PARTIAL {}",
        serde_json::to_string(&outcome).unwrap()
    );
}
#[test]
fn directory_reopen_failure_and_poisoned_journal_remain_explicit() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.host.root().join("docs/custom/changes")).unwrap();
    let client = fixture.host.client().unwrap();
    let capture = client.freeze().unwrap();
    let plan = fixture.prepare(&capture).unwrap();
    let id = plan.id().to_string();
    client
        .acknowledge_absent_edit(plan.barrier(), true)
        .unwrap();
    vibe_safefs::arm_after_create_dir(Some(Box::new(|_, name| {
        if name == ID {
            Some(std::io::Error::other("owned fixture reopen failure"))
        } else {
            None
        }
    })));
    let outcome = plan.apply().unwrap();
    assert!(matches!(outcome.state, ScaffoldState::Partial));
    assert!(outcome.directory_uncertain);
    assert_eq!(outcome.pending.len(), 4);
    assert!(
        fixture
            .host
            .root()
            .join(&fixture.authority.directory)
            .is_dir()
    );
    let before = tree(fixture.root.path());
    let report = fixture.authority.inspect_recovery(&id).unwrap();
    assert!(report.directory_uncertain);
    preserved(&before, fixture.root.path());
    let journal_path = fixture.host.root().join(&outcome.journal);
    let raw = fs::read(&journal_path).unwrap();
    for modification in ["epoch", "field", "desired"] {
        let mut row: journal::Journal = toml::from_str(std::str::from_utf8(&raw).unwrap()).unwrap();
        let changed = match modification {
            "epoch" => {
                row.schema = 2;
                toml::to_string(&row).unwrap()
            }
            "field" => format!("unknown_field=true\n{}", std::str::from_utf8(&raw).unwrap()),
            _ => {
                row.files[0].desired.push(0);
                toml::to_string(&row).unwrap()
            }
        };
        assert!(matches!(
            journal::decode(changed.as_bytes(), &fixture.authority),
            Err(ScaffoldRefusal::Journal)
        ));
    }
    println!(
        "SCAFFOLD_DIRECTORY_UNCERTAIN {}",
        serde_json::to_string(&outcome).unwrap()
    );
}
