use super::*;
use std::fs;

const MANIFEST: &str =
    "[package]\nname = 'guard'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.0.0'\n";
const RAW: &str = "[[package.sources]]\nfile = 'source.xml'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'explicit'\n";

#[test]
fn declared_doc_vocabulary_requires_the_actual_doc_package() {
    for kind in ["doc", "flow"] {
        let tmp = tempfile::tempdir().unwrap();
        let documents = if kind == "doc" {
            "[[documents]]\npackage='org.example/subject'\nversion='^1.0'\n"
        } else {
            ""
        };
        fs::write(tmp.path().join("vibe.toml"), format!("[package]\nname='manual'\ngroup='org.example'\nkind='{kind}'\nversion='1.0.0'\ntitle='Manual'\nabstract='Documents source reading for maintainers; assumes basic package knowledge and excludes installation.'\n[[package.sources]]\nfile='manual.payload'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n{documents}")).unwrap();
        vibe_core::manifest::Manifest::read(tmp.path().join("vibe.toml"))
            .expect("complete typed fixture manifest");
        fs::write(
            tmp.path().join("progress.toml"),
            "include=['manual.payload']\n",
        )
        .unwrap();
        fs::write(
            tmp.path().join("manual.payload"),
            "<spec xmlns=\"https://vibevm.org/spec/1\"><example id=\"e\" fixture=\"none\"><run>sample</run><expect></expect></example></spec>",
        )
        .unwrap();
        let result = ground(&ProgressCommonArgs {
            path: tmp.path().to_owned(),
            campaign: None,
            no_cache: false,
        });
        if kind == "doc" {
            assert_eq!(result.unwrap().docs.len(), 1);
        } else {
            assert!(format!("{:#}", result.err().unwrap()).contains("Doc vocabulary"));
        }
    }
}

#[test]
fn literal_glob_does_not_invent_a_directory_default_file_promise() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("vibe.toml"), "[project]\nname='host'\nversion='1.0.0'\n[[project.sources]]\ndirectory='docs'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    fs::write(
        tmp.path().join("progress.toml"),
        "include=['docs/absent.md']\n",
    )
    .unwrap();
    let args = ProgressCommonArgs {
        path: tmp.path().to_owned(),
        campaign: None,
        no_cache: false,
    };
    assert!(ground(&args).unwrap().docs.is_empty());
    let text = fs::read_to_string(tmp.path().join("vibe.toml")).unwrap();
    fs::write(tmp.path().join("vibe.toml"), format!("{text}\n[[project.sources]]\nfile='docs/absent.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n")).unwrap();
    assert!(
        ground(&args).is_err(),
        "an explicit File declaration is an existence promise"
    );
}

#[test]
fn unrecorded_legacy_slot_spelling_does_not_require_an_enclosing_lock() {
    for manifest in [None, Some(MANIFEST)] {
        let tmp = tempfile::tempdir().unwrap();
        let slot = tmp.path().join(vibe_workspace::vibedeps::slot_rel_path(
            &"org.example".parse().unwrap(),
            "guard",
            &"1.0.0".parse().unwrap(),
        ));
        fs::create_dir_all(&slot).unwrap();
        if let Some(manifest) = manifest {
            fs::write(slot.join("vibe.toml"), manifest).unwrap();
        }
        fs::write(slot.join("source.md"), "# Legacy\n").unwrap();
        fs::write(slot.join("progress.toml"), "include=['source.md']\n").unwrap();
        let args = ProgressCommonArgs {
            path: slot.clone(),
            campaign: None,
            no_cache: false,
        };
        assert_eq!(ground(&args).unwrap().docs.len(), 1);
        fs::write(
            slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME),
            "record cue",
        )
        .unwrap();
        assert!(
            ground(&args).is_err(),
            "a recorded root cannot use unbound compatibility"
        );
        fs::remove_file(slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME)).unwrap();
        fs::write(slot.join("vibe.toml"), format!("{MANIFEST}sources=[]\n")).unwrap();
        assert!(
            ground(&args).is_err(),
            "declared slot cannot downgrade even when empty"
        );
    }
}

#[test]
fn recorded_legacy_keeps_independent_lock_and_record_guards() {
    for changed in ["lock", "record"] {
        let tmp = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("vibe.toml"), MANIFEST).unwrap();
        fs::write(source.path().join("source.md"), "# Legacy\n").unwrap();
        fs::write(
            tmp.path().join("vibe.toml"),
            "[project]\nname='host'\nversion='1.0.0'\n",
        )
        .unwrap();
        let group = "org.example".parse().unwrap();
        let version = "1.0.0".parse().unwrap();
        let hash = vibe_core::ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
        vibe_workspace::vibedeps::materialise(
            tmp.path(),
            &group,
            "guard",
            &version,
            source.path(),
            &hash,
        )
        .unwrap();
        fs::write(tmp.path().join("vibe.lock"), format!("[meta]\nschema_version={}\ngenerated_by='test'\ngenerated_at='2026-01-01T00:00:00Z'\n[[package]]\nkind='flow'\ngroup='org.example'\nname='guard'\nversion='1.0.0'\nsource_url='https://example.invalid/guard.git'\ncontent_hash='{hash}'\n", vibe_core::manifest::CURRENT_SCHEMA_VERSION)).unwrap();
        let slot = tmp.path().join(vibe_workspace::vibedeps::slot_rel_path(
            &group, "guard", &version,
        ));
        fs::write(
            slot.join("progress.toml"),
            "include=['source.md']\n[progress]\ncache_dir='payload-cache'\n",
        )
        .unwrap();
        let campaign = tmp.path().join("campaigns/c");
        fs::create_dir_all(campaign.join("run")).unwrap();
        let args = ProgressCommonArgs {
            path: slot.clone(),
            campaign: Some(campaign.clone()),
            no_cache: false,
        };
        let mut g = ground(&args).unwrap();
        assert_eq!(g.docs.len(), 1);
        if changed == "lock" {
            fs::write(tmp.path().join("vibe.lock"), "changed").unwrap();
        } else {
            fs::write(
                slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME),
                "changed",
            )
            .unwrap();
        }
        assert!(refresh_state(&mut g).is_err());
        assert!(!campaign.join("run/cache.json").exists());
        assert!(!slot.join("payload-cache").exists());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn source_policy_low_fd_progress_ground_and_refresh() {
    const CHILD_ROOT: &str = "VIBE_TEST_SOURCE_POLICY_LOW_FD_PROGRESS_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let common = ProgressCommonArgs {
            path: root.clone(),
            campaign: Some(root.join("campaigns/guard")),
            no_cache: false,
        };
        let mut grounded =
            ground(&common).expect("128 sibling sources must ground under FD limit64");
        assert_eq!(grounded.docs.len(), 128);
        refresh_state(&mut grounded)
            .expect("the same bounded observations must permit cache refresh");
        assert!(root.join("campaigns/guard/run/cache.json").is_file());
        for n in 0..128 {
            assert_eq!(
                fs::read_to_string(root.join(format!("d{n:03}/source.md"))).unwrap(),
                "# T {#t}\n\n@fact:A body @status:impl/done\n\n"
            );
        }
        return;
    }
    // The parent owns cleanup even if the resource-limited child panics.
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("vibe.toml"), MANIFEST).unwrap();
    fs::write(
        tmp.path().join("progress.toml"),
        "include=['d*/*.md']\n[progress]\ncache_dir='payload-cache'\n",
    )
    .unwrap();
    fs::create_dir_all(tmp.path().join("campaigns/guard/run")).unwrap();
    for n in 0..128 {
        let dir = tmp.path().join(format!("d{n:03}"));
        fs::create_dir(&dir).unwrap();
        fs::write(
            dir.join("source.md"),
            "# T {#t}\n\n@fact:A body @status:impl/done\n\n",
        )
        .unwrap();
    }
    let output = std::process::Command::new("/bin/sh")
        .args(["-c", "ulimit -n 64 || exit 90; test \"$(ulimit -n)\" -eq 64 || exit 91; exec \"$@\"", "low-fd-test"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "commands::progress::grounding::tests::source_policy_low_fd_progress_ground_and_refresh", "--nocapture", "--test-threads=1"])
        .env(CHILD_ROOT, tmp.path())
        .output().unwrap();
    assert!(
        output.status.success(),
        "low-FD child {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("test result: ok. 1 passed;"));
    assert!(tmp.path().join("campaigns/guard/run/cache.json").is_file());
}

fn fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("progress.toml"),
        "include = ['*.md', '*.xml']\n[progress]\ncache_dir = 'payload-cache'\n",
    )
    .unwrap();
    fs::write(tmp.path().join("source.xml"), "<foreign/>").unwrap();
    fs::write(tmp.path().join("source.md"), "# source\n").unwrap();
    tmp
}

#[test]
fn source_policy_empty_excludes_raw_pairs_before_projection() {
    for key in ["sources", "source_exports"] {
        let tmp = fixture();
        fs::write(
            tmp.path().join("vibe.toml"),
            format!("{MANIFEST}{key} = []\n"),
        )
        .unwrap();
        let common = ProgressCommonArgs {
            path: tmp.path().to_owned(),
            campaign: None,
            no_cache: false,
        };
        let result = ground(&common).unwrap();
        assert!(result.docs.is_empty());
        assert!(!tmp.path().join("payload-cache").exists());
    }
}

#[test]
fn source_policy_explicit_and_invalid_refuse_before_pair_or_cache() {
    for suffix in [RAW, "sources = 'invalid'\n"] {
        let tmp = fixture();
        fs::write(tmp.path().join("vibe.toml"), format!("{MANIFEST}{suffix}")).unwrap();
        let common = ProgressCommonArgs {
            path: tmp.path().to_owned(),
            campaign: None,
            no_cache: false,
        };
        if suffix == RAW {
            assert!(ground(&common).unwrap().docs.is_empty());
            assert!(!tmp.path().join("payload-cache").exists());
            continue;
        }
        let error = ground(&common)
            .err()
            .expect("policy must refuse")
            .to_string();
        assert!(error.contains("SOURCE-ADMISSION-BOUNDARY"), "{error}");
        assert!(!error.contains("one document, one form"), "{error}");
        assert!(!tmp.path().join("payload-cache").exists());
        assert_eq!(
            fs::read_to_string(tmp.path().join("source.xml")).unwrap(),
            "<foreign/>"
        );
    }
}

#[test]
fn source_policy_legacy_and_missing_keep_vocabulary_and_detect_late_opt_in() {
    for manifest in [None, Some(MANIFEST)] {
        let tmp = tempfile::tempdir().unwrap();
        if let Some(manifest) = manifest {
            fs::write(tmp.path().join("vibe.toml"), manifest).unwrap();
        }
        let mut policies = VocabularyByPackage::default();
        assert_eq!(
            policies
                .of(tmp.path(), &tmp.path().join("source.md"))
                .unwrap(),
            Some(vibe_specdoc::Vocabulary::Spec)
        );
        policies.recheck().unwrap();
        fs::write(
            tmp.path().join("vibe.toml"),
            format!("{MANIFEST}sources = []\n"),
        )
        .unwrap();
        assert!(policies.recheck().is_err());
    }
}

#[test]
fn source_policy_changed_after_ground_refuses_cache_publication() {
    let tmp = fixture();
    fs::remove_file(tmp.path().join("source.xml")).unwrap();
    fs::write(tmp.path().join("vibe.toml"), MANIFEST).unwrap();
    let campaign = tmp.path().join("campaigns/guard");
    let cache_path = campaign.join("run/cache.json");
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    cache::Cache::default().store(&cache_path).unwrap();
    let before = fs::read(&cache_path).unwrap();
    let common = ProgressCommonArgs {
        path: tmp.path().to_owned(),
        campaign: Some(campaign),
        no_cache: false,
    };
    let mut grounded = ground(&common).unwrap();
    assert_eq!(grounded.docs.len(), 1);
    fs::write(
        tmp.path().join("vibe.toml"),
        format!("{MANIFEST}sources = []\n"),
    )
    .unwrap();
    assert!(refresh_state(&mut grounded).is_err());
    assert_eq!(fs::read(&cache_path).unwrap(), before);
    assert!(!tmp.path().join("payload-cache").exists());
}

#[test]
fn source_policy_new_intermediate_manifest_refuses_cache_publication() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("vibe.toml"), MANIFEST).unwrap();
    fs::write(
        tmp.path().join("progress.toml"),
        "include=['nested/*.md']\n[progress]\ncache_dir='payload-cache'\n",
    )
    .unwrap();
    let nested = tmp.path().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("source.md"), "# source\n").unwrap();
    let campaign = tmp.path().join("campaigns/guard");
    let cache_path = campaign.join("run/cache.json");
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    cache::Cache::default().store(&cache_path).unwrap();
    let before = fs::read(&cache_path).unwrap();
    let common = ProgressCommonArgs {
        path: tmp.path().to_owned(),
        campaign: Some(campaign),
        no_cache: false,
    };
    let mut grounded = ground(&common).unwrap();
    assert_eq!(grounded.docs.len(), 1);
    fs::write(nested.join("vibe.toml"), format!("{MANIFEST}sources=[]\n")).unwrap();
    let error = refresh_state(&mut grounded).unwrap_err();
    assert!(error.to_string().contains("SOURCE-ADMISSION-BOUNDARY"));
    assert_eq!(fs::read(cache_path).unwrap(), before);
    assert!(!tmp.path().join("payload-cache").exists());
    assert_eq!(
        fs::read_to_string(nested.join("source.md")).unwrap(),
        "# source\n"
    );
}
