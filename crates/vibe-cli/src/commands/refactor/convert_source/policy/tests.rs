use super::*;
const MANIFEST: &str =
    "[package]\nname='guard'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n";
const RAW: &str = "[[package.sources]]\nfile='source.md'\nmedia='text/markdown'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n";

#[cfg(target_os = "linux")]
#[test]
fn source_policy_low_fd_conversion_of_sibling_directories() {
    const CHILD_ROOT: &str = "VIBE_TEST_SOURCE_POLICY_LOW_FD_CONVERSION_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        run(vec![ConversionTarget::Path(root.clone())])
            .expect("128 sibling sources must convert under FD limit64");
        for n in 0..128 {
            let source = root.join(format!("d{n:03}/source.md"));
            assert!(!source.exists());
            let xml = fs::read_to_string(source.with_extension("xml")).unwrap();
            assert!(xml.contains("<spec"));
            assert!(xml.contains("body"));
        }
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("vibe.toml"), MANIFEST).unwrap();
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
        .args(["--exact", "commands::refactor::convert_source::policy::tests::source_policy_low_fd_conversion_of_sibling_directories", "--nocapture", "--test-threads=1"])
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
    assert!(tmp.path().join("d127/source.xml").is_file());
    assert!(!tmp.path().join("d127/source.md").exists());
}

fn run(targets: Vec<ConversionTarget>) -> anyhow::Result<()> {
    let context =
        crate::output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Cli);
    super::super::run_conversion(
        &context,
        targets,
        super::super::ResolvedConversion {
            from: super::super::SourceFormat::Markdown,
            to: super::super::SourceFormat::Xml,
            force: true,
            dry_run: false,
        },
        "convert-source",
    )
}

#[test]
fn source_policy_direct_file_cannot_bypass_enclosing_opt_in_or_invalid_policy() {
    for suffix in [
        "sources=[]\n",
        "source_exports=[]\n",
        RAW,
        "sources='invalid'\n",
    ] {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("vibe.toml"), format!("{MANIFEST}{suffix}")).unwrap();
        let source = tmp.path().join("nested/source.md");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "# source\n").unwrap();
        let error = run(vec![ConversionTarget::Path(source.clone())]).unwrap_err();
        assert!(
            error.to_string().contains("SOURCE-ADMISSION-BOUNDARY"),
            "{error}"
        );
        assert_eq!(fs::read_to_string(&source).unwrap(), "# source\n");
        assert!(!source.with_extension("xml").exists());
    }
}

#[test]
fn source_policy_discovery_stops_at_selected_or_nearest_root() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("vibe.toml"),
        format!("{MANIFEST}sources=[]\n"),
    )
    .unwrap();
    let child = tmp.path().join("child");
    fs::create_dir_all(&child).unwrap();
    let source = child.join("source.md");
    fs::write(&source, "# source\n").unwrap();
    // Explicit content-tree authority must not climb above its boundary.
    ConversionPolicies::admit(&[ConversionTarget::Selected {
        root: child.clone(),
        path: source.clone(),
    }])
    .unwrap();
    fs::write(child.join("vibe.toml"), MANIFEST).unwrap();
    // The nearest legacy package owns the source, regardless of its parent.
    let policies = ConversionPolicies::admit(&[ConversionTarget::Path(source)]).unwrap();
    fs::write(child.join("vibe.toml"), format!("{MANIFEST}sources=[]\n")).unwrap();
    assert!(policies.recheck().is_err());
}

#[test]
fn source_policy_nested_package_is_preflighted_before_any_legacy_conversion() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("vibe.toml"), MANIFEST).unwrap();
    fs::write(tmp.path().join("first.md"), "# first\n").unwrap();
    let nested = tmp.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("vibe.toml"), format!("{MANIFEST}{RAW}")).unwrap();
    fs::write(nested.join("source.md"), "raw").unwrap();
    assert!(run(vec![ConversionTarget::Path(tmp.path().to_owned())]).is_err());
    assert_eq!(
        fs::read_to_string(tmp.path().join("first.md")).unwrap(),
        "# first\n"
    );
    assert!(!tmp.path().join("first.xml").exists());
}

#[test]
fn source_policy_legacy_and_standalone_conversion_preserve_success() {
    for manifest in [None, Some(MANIFEST)] {
        let tmp = tempfile::tempdir().unwrap();
        if let Some(manifest) = manifest {
            fs::write(tmp.path().join("vibe.toml"), manifest).unwrap();
        }
        let path = tmp.path().join("source.md");
        fs::write(&path, "# T {#t}\n\n@fact:A body @status:impl/done\n\n").unwrap();
        run(vec![ConversionTarget::Path(path.clone())]).unwrap();
        assert!(!path.exists());
        let xml = fs::read_to_string(path.with_extension("xml")).unwrap();
        assert!(xml.contains("<spec"));
        assert!(xml.contains("body"));
    }
}

#[test]
fn source_policy_new_intermediate_manifest_refuses_conversion_publication() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("vibe.toml"), MANIFEST).unwrap();
    let nested = tmp.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let source = nested.join("source.md");
    fs::write(&source, "# source\n").unwrap();
    let policies = ConversionPolicies::admit(&[ConversionTarget::Path(source.clone())]).unwrap();
    fs::write(nested.join("vibe.toml"), format!("{MANIFEST}sources=[]\n")).unwrap();
    let mut counts = super::super::Counts::default();
    super::super::write_or_refuse(
        &source,
        "must not be published",
        super::super::SourceFormat::Xml,
        "converted",
        &mut counts,
        false,
        &policies,
    );
    assert_eq!(counts.refused, 1);
    assert_eq!(counts.converted, 0);
    assert_eq!(fs::read_to_string(&source).unwrap(), "# source\n");
    assert!(!source.with_extension("xml").exists());
}
