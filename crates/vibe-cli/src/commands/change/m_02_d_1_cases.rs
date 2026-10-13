use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;
use std::fs;
use vibe_core::{
    ContentHash,
    manifest::{Lockfile, SpecFormat},
};
pub(super) fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let host = root.path().join("host");
    fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
    fs::create_dir(&host).unwrap();
    fs::write(host.join("vibe.toml"),"[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'flow:org.demo/dep'='=1.0.0'\n").unwrap();
    fs::write(source.join("vibe.toml"),"[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    fs::write(
        source.join("vibevm/vibespecs/DOC.md"),
        "# D {#root}\n\n@fact:DELIVERY Deliver here. @status:spec\n",
    )
    .unwrap();
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
    root
}
fn inventory(root: &std::path::Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
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
fn value(report: report::Report) -> serde_json::Value {
    let raw = report::bytes(&report).unwrap();
    let decoded = report::decode(&raw).unwrap();
    serde_json::to_value(decoded).unwrap()
}
#[test]
fn parsed_cli_prepare_apply_and_no_overwrite_match_artifacts() {
    let root = fixture();
    let host = root.path().join("host");
    let argv = [
        "vibe",
        "--json",
        "change",
        "new",
        "--from",
        "flow:org.demo/dep@=1.0.0",
        "--id",
        "chg-0123456789abcdef0123456789abcdef",
        "--spec-root",
        "docs/custom",
        "--path",
        host.to_str().unwrap(),
    ];
    let parsed = Cli::try_parse_from(argv).unwrap();
    let Command::Change(args) = parsed.command else {
        panic!("wrong command")
    };
    let ChangeCommand::New(mut args) = args.command else {
        panic!("wrong change subcommand")
    };
    let before = inventory(root.path());
    let preview = value(execute(&args).unwrap());
    assert_eq!(before, inventory(root.path()));
    args.apply = true;
    let applied = value(execute(&args).unwrap());
    assert_eq!(preview["prepared"], applied["prepared"]);
    assert_eq!(applied["outcome"]["state"], "completed");
    let directory = host.join(applied["prepared"]["directory"].as_str().unwrap());
    for (name, bytes) in applied["prepared"]["files"].as_object().unwrap() {
        assert_eq!(
            fs::read(directory.join(name)).unwrap(),
            bytes.as_str().unwrap().as_bytes()
        );
    }
    let after = inventory(root.path());
    let error = execute(&args).unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ScaffoldRefusal>(),
        Some(ScaffoldRefusal::Occupied)
    ));
    let refusal = value(report::refused(&error, true));
    assert_eq!(refusal["status"], "refused");
    assert_eq!(refusal["code"], "occupied");
    assert_eq!(after, inventory(root.path()));
    args.id = "chg-fedcba9876543210fedcba9876543210".into();
    args.spec_root = Some("../escape".into());
    let error = execute(&args).unwrap_err();
    let declaration = error
        .downcast_ref::<vibe_core::manifest::change::ChangeError>()
        .unwrap();
    assert_eq!(
        declaration.kind,
        vibe_core::manifest::change::ChangeErrorKind::Path
    );
    assert_eq!(declaration.field, "spec_root");
    assert_eq!(after, inventory(root.path()));
    println!("CLI_PREVIEW {preview}\nCLI_APPLIED {applied}");
}
#[test]
fn save_owned_fixture_for_production_cli_smoke() {
    // Separate fixture setup; no production query proof is claimed here.
    let root = fixture().keep();
    println!("CLI_SMOKE_ROOT {}", root.display());
}

#[test]
fn generated_cli_report_corpus_refuses_unknown_epoch_fields_and_false_success() {
    for raw in [
        include_bytes!(
            "../../../../../formats/corpora/change-scaffold-cli-report/e1/prepared.json"
        )
        .as_slice(),
        include_bytes!(
            "../../../../../formats/corpora/change-scaffold-cli-report/e1/completed.json"
        )
        .as_slice(),
        include_bytes!("../../../../../formats/corpora/change-scaffold-cli-report/e1/partial.json")
            .as_slice(),
        include_bytes!("../../../../../formats/corpora/change-scaffold-cli-report/e1/refused.json")
            .as_slice(),
    ] {
        let decoded = report::decode(raw).unwrap();
        let encoded = report::bytes(&decoded).unwrap();
        assert_eq!(report::decode(&encoded).unwrap(), decoded);
    }
    let error = report::decode(include_bytes!(
        "../../../../../formats/corpora/change-scaffold-cli-report/e1/unknown-field.json"
    ))
    .unwrap_err();
    assert!(error.to_string().contains("unknown field `unrecognized`"));
    assert_eq!(
        report::decode(include_bytes!(
            "../../../../../formats/corpora/change-scaffold-cli-report/e1/unknown-epoch.json"
        ))
        .unwrap_err()
        .to_string(),
        "scaffold report epoch"
    );
    assert_eq!(
        report::decode(include_bytes!(
            "../../../../../formats/corpora/change-scaffold-cli-report/e1/false-completed.json"
        ))
        .unwrap_err()
        .to_string(),
        "scaffold completed truth"
    );
}

#[test]
fn actual_cli_partial_returns_failure_and_preserves_existing_package_bytes() {
    let root = fixture();
    let host = root.path().join("host");
    let before = inventory(root.path());
    let parsed = Cli::try_parse_from([
        "vibe",
        "--json",
        "change",
        "new",
        "--from",
        "flow:org.demo/dep@=1.0.0",
        "--id",
        "chg-0123456789abcdef0123456789abcdef",
        "--path",
        host.to_str().unwrap(),
        "--spec-root",
        "docs/custom",
        "--apply",
    ])
    .unwrap();
    let Command::Change(args) = parsed.command else {
        panic!("wrong command")
    };
    vibe_safefs::fail_after_publish(Some("change.toml"));
    let exit = run(&args, true);
    vibe_safefs::fail_after_publish(None);
    assert_eq!(exit, ExitCode::from(1));
    let target = host.join("docs/custom/changes/chg-0123456789abcdef0123456789abcdef");
    assert!(target.join("acceptance.md").is_file());
    assert!(target.join("change.toml").is_file());
    assert!(!target.join("intent.md").exists());
    assert!(!target.join("refinement.md").exists());
    let after = inventory(root.path());
    for row in &before {
        assert!(
            after.contains(row),
            "preexisting authored bytes/dir changed: {:?}",
            row.0
        );
    }
    let journal = fs::read_dir(host.join(".vibe/change-scaffolds"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let row: toml::Value = toml::from_str(&fs::read_to_string(journal).unwrap()).unwrap();
    assert_eq!(row["completed"].as_bool(), Some(false));
    let states: Vec<_> = row["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["state"].as_str().unwrap())
        .collect();
    assert_eq!(states, ["verified", "uncertain", "pending", "pending"]);
}
