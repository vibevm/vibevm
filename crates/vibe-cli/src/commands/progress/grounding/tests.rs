use super::*;
use std::fs;

const MANIFEST: &str =
    "[package]\nname = 'guard'\ngroup = 'org.example'\nkind = 'flow'\nversion = '1.0.0'\n";
const RAW: &str = "[[package.sources]]\nfile = 'source.xml'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'explicit'\n";

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
