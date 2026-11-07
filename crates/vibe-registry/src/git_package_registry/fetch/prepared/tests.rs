//! Real local Git commits exercise preparation before project mutation.
#![cfg(all(test, unix))]
use super::*;
use crate::git_package_registry::test_support::{registry_with, registry_with_mirrors};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "protocol.file.allow=always",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
struct Fixture {
    _root: tempfile::TempDir,
    upstream: PathBuf,
    cache: PathBuf,
    slot: PathBuf,
    backend: Arc<dyn GitBackend>,
}
impl Fixture {
    fn new() -> Self {
        Self::format("sha1")
    }
    fn format(format: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let upstream = root.path().join("org/org.vibevm.giant.git");
        fs::create_dir_all(&upstream).unwrap();
        git(
            &upstream,
            &[
                "init",
                "--initial-branch=main",
                &format!("--object-format={format}"),
            ],
        );
        let wrapper = root.path().join("fixture-git");
        fs::write(&wrapper,"#!/bin/sh\nexport GIT_CONFIG_NOSYSTEM=1\nexport GIT_CONFIG_GLOBAL=/dev/null\nexec git -c core.hooksPath=/dev/null -c protocol.file.allow=always \"$@\"\n").unwrap();
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
        let cache = root.path().join("cache");
        let slot = root.path().join("project/slot");
        let backend = Arc::new(ShellGit::new().with_binary(wrapper));
        Self {
            _root: root,
            upstream,
            cache,
            slot,
            backend,
        }
    }
    fn publish(&self, version: &str, body: &str, common: bool) -> String {
        let extra = if common {
            "\n[[configuration.features]]\nid='extra'\nname='Extra'\ndefault=true\n"
        } else {
            ""
        };
        let description = if fs::read_to_string(self.upstream.join(".gitattributes"))
            .is_ok_and(|s| s.contains(" ident"))
        {
            "description='$Id$'\n"
        } else {
            ""
        };
        fs::write(self.upstream.join("vibe.toml"),format!("[package]\ngroup='org.vibevm'\nname='giant'\nkind='feat'\nversion='{version}'\nmaterialization='in-place'\n{description}{extra}")).unwrap();
        fs::write(self.upstream.join("payload.txt"), body).unwrap();
        fs::write(self.upstream.join(".gitignore"), "ignored.tmp\n").unwrap();
        if fs::read_to_string(self.upstream.join(".gitattributes"))
            .is_ok_and(|s| s.contains("working-tree-encoding=UTF-16"))
        {
            // Give Git a valid UTF-16 worktree input; its committed blob is UTF-8.
            let path = self.upstream.join("vibe.toml");
            let text = fs::read_to_string(&path).unwrap();
            let bytes: Vec<_> = [0xff, 0xfe]
                .into_iter()
                .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
                .collect();
            fs::write(path, bytes).unwrap();
        }
        git(&self.upstream, &["add", "-A"]);
        git(&self.upstream, &["commit", "-m", "fixture version"]);
        git(&self.upstream, &["tag", "-f", &format!("v{version}")]);
        git(&self.upstream, &["rev-parse", "HEAD"])
    }
    fn registry(&self) -> GitPerPackageRegistry {
        registry_with(
            &self.cache,
            self.upstream.parent().unwrap().to_str().unwrap(),
            NamingConvention::Fqdn,
            self.backend.clone(),
        )
    }
    fn selected(&self, r: &GitPerPackageRegistry, version: &str) -> ResolvedPackage {
        r.resolve(&PackageRef::parse(&format!("org.vibevm/giant@={version}")).unwrap())
            .unwrap()
    }
    fn seed(&self) {
        let r = self.registry();
        let selected = self.selected(&r, "1.0.0");
        r.materialise_in_place(&selected, &self.slot).unwrap();
    }
}
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let p = entry.unwrap().path();
            if p.file_name().unwrap() == ".git" {
                continue;
            }
            if p.is_dir() {
                walk(root, &p, into);
            } else {
                into.insert(p.strip_prefix(root).unwrap().into(), fs::read(p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[test]
fn fresh_preparation_pins_objects_despite_remote_retag_and_survives_drop() {
    let f = Fixture::new();
    let admitted = f.publish("1.0.0", "ADMITTED", false);
    let r = f.registry();
    let selected = f.selected(&r, "1.0.0");
    let prepared = r.prepare_in_place(&selected, &f.slot).unwrap();
    assert!(!f.slot.exists());
    assert!(prepared.manifest().configuration.is_none());
    assert_eq!(prepared.resolved_commit(), admitted);
    let cache = prepared.0._temporary.path().to_path_buf();
    let retained = prepared.clone();
    drop(prepared);
    f.publish("1.0.0", "RETARGETED", true);
    fs::rename(&f.upstream, f.upstream.with_extension("gone")).unwrap();
    let result = retained.materialise().unwrap();
    assert!(result.changed);
    drop(retained);
    assert!(!cache.exists());
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), admitted);
    assert_eq!(
        fs::read_to_string(f.slot.join("payload.txt")).unwrap(),
        "ADMITTED"
    );
    assert!(!f.slot.join(".git/objects/info/alternates").exists());
    git(&f.slot, &["fsck", "--full"]);
}

#[test]
fn prospective_common_manifest_does_not_advance_or_clean_existing_checkout() {
    let f = Fixture::new();
    let old = f.publish("1.0.0", "OLD", false);
    f.seed();
    fs::write(f.slot.join("payload.txt"), "DIRTY").unwrap();
    fs::write(f.slot.join("untracked.tmp"), "UNTRACKED").unwrap();
    fs::write(f.slot.join("ignored.tmp"), "IGNORED").unwrap();
    let before = tree(&f.slot);
    let config = fs::read(f.slot.join(".git/config")).unwrap();
    f.publish("1.1.0", "COMMON", true);
    let r = f.registry();
    let prepared = r
        .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
        .unwrap();
    assert!(prepared.manifest().configuration.is_some());
    drop(prepared);
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), old);
    assert_eq!(tree(&f.slot), before);
    assert_eq!(fs::read(f.slot.join(".git/config")).unwrap(), config);
}
#[test]
fn legacy_refresh_reuses_git_objects_repairs_dirt_and_has_no_dangling_preparation() {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    fs::write(f.slot.join(".git/sentinel"), "KEEP REPOSITORY").unwrap();
    let desired = f.publish("1.1.0", "NEW", false);
    let r = f.registry();
    let prepared = r
        .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
        .unwrap();
    fs::write(f.slot.join("payload.txt"), "DIRTY AFTER PREPARE").unwrap();
    fs::write(f.slot.join("untracked.tmp"), "UNTRACKED").unwrap();
    fs::write(f.slot.join("ignored.tmp"), "IGNORED").unwrap();
    assert!(prepared.materialise().unwrap().changed);
    drop(prepared);
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), desired);
    assert_eq!(
        fs::read_to_string(f.slot.join(".git/sentinel")).unwrap(),
        "KEEP REPOSITORY"
    );
    assert!(!f.slot.join("untracked.tmp").exists() && !f.slot.join("ignored.tmp").exists());
    assert_eq!(
        fs::read_to_string(f.slot.join("payload.txt")).unwrap(),
        "NEW"
    );
    let same = r
        .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
        .unwrap();
    assert!(!same.materialise().unwrap().changed);
    drop(same);
    fs::rename(&f.upstream, f.upstream.with_extension("gone")).unwrap();
    git(&f.slot, &["fsck", "--full"]);
}
#[test]
fn head_or_origin_changes_refuse_before_worktree_mutation() {
    for change_origin in [false, true] {
        let f = Fixture::new();
        let old = f.publish("1.0.0", "OLD", false);
        f.seed();
        f.publish("1.1.0", "NEW", false);
        let r = f.registry();
        let p = r
            .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
            .unwrap();
        if change_origin {
            git(
                &f.slot,
                &["remote", "set-url", "origin", "/nonexistent/changed-origin"],
            );
        } else {
            fs::write(f.slot.join("payload.txt"), "USER COMMIT").unwrap();
            git(&f.slot, &["add", "payload.txt"]);
            git(&f.slot, &["commit", "-m", "user changed HEAD"]);
            assert_ne!(git(&f.slot, &["rev-parse", "HEAD"]), old);
        }
        let before = tree(&f.slot);
        let head = git(&f.slot, &["rev-parse", "HEAD"]);
        assert!(matches!(
            p.materialise(),
            Err(RegistryError::PreparedInPlace { .. })
        ));
        assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), head);
        assert_eq!(tree(&f.slot), before);
    }
}
#[test]
fn existing_origin_is_the_prospective_source_even_when_canonical_metadata_differs() {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    f.publish("1.1.0", "CANONICAL", false);
    let alternate = Fixture::new();
    alternate.publish("1.1.0", "ACTUAL COMMON", true);
    git(
        &f.slot,
        &[
            "remote",
            "set-url",
            "origin",
            alternate.upstream.to_str().unwrap(),
        ],
    );
    let before = tree(&f.slot);
    let r = f.registry();
    let p = r
        .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
        .unwrap();
    assert!(p.manifest().configuration.is_some());
    assert_eq!(tree(&f.slot), before);
    assert_eq!(p.source_uri(), f.upstream.to_str().unwrap());
}

#[test]
fn authoritative_manifest_transform_refuses_without_running_smudge_or_touching_checkout() {
    for attributes in [
        "vibe.toml filter=canary\n",
        "vibe.toml working-tree-encoding=UTF-16\n",
    ] {
        let f = Fixture::new();
        let old = f.publish("1.0.0", "OLD", false);
        f.seed();
        let canary = f.slot.join("SMUDGE_RAN");
        git(
            &f.slot,
            &[
                "config",
                "filter.canary.smudge",
                &format!("touch '{}'; cat", canary.display()),
            ],
        );
        fs::write(f.upstream.join(".gitattributes"), attributes).unwrap();
        f.publish("1.1.0", "NEW", false);
        let r = f.registry();
        let before = tree(&f.slot);
        assert!(
            r.prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
                .is_err()
        );
        assert!(!canary.exists());
        assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), old);
        assert_eq!(tree(&f.slot), before);
    }
}
#[test]
fn target_info_attributes_are_checked_and_ordinary_eol_remains_supported() {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    fs::write(
        f.upstream.join(".gitattributes"),
        "vibe.toml text eol=crlf\n",
    )
    .unwrap();
    f.publish("1.1.0", "NEW", false);
    let r = f.registry();
    fs::write(
        f.slot.join(".git/info/attributes"),
        "vibe.toml filter=canary\n",
    )
    .unwrap();
    assert!(
        r.prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
            .is_err()
    );
    fs::remove_file(f.slot.join(".git/info/attributes")).unwrap();
    let p = r
        .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
        .unwrap();
    p.materialise().unwrap();
    assert!(
        fs::read(f.slot.join("vibe.toml"))
            .unwrap()
            .windows(2)
            .any(|v| v == b"\r\n")
    );
}

#[test]
fn missing_origin_can_use_a_prepared_mirror_but_inspection_errors_never_mean_missing() {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    let mirror = Fixture::new();
    mirror.publish("1.1.0", "MIRROR", false);
    git(&f.slot, &["remote", "remove", "origin"]);
    let r = registry_with_mirrors(
        &f.cache,
        f.upstream.parent().unwrap().to_str().unwrap(),
        NamingConvention::Fqdn,
        vec![
            mirror
                .upstream
                .parent()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ],
        f.backend.clone(),
    );
    let mut selected = f.selected(&r, "1.0.0");
    selected.version = semver::Version::new(1, 1, 0);
    let prepared = r.prepare_in_place(&selected, &f.slot).unwrap();
    let head = fs::read(f.slot.join(".git/HEAD")).unwrap();
    let before = tree(&f.slot);
    let config = f.slot.join(".git/config");
    let valid = fs::read(&config).unwrap();
    fs::write(&config, "[invalid\n").unwrap();
    assert!(prepared.materialise().is_err());
    assert_eq!(fs::read(f.slot.join(".git/HEAD")).unwrap(), head);
    assert_eq!(tree(&f.slot), before);
    assert!(r.prepare_in_place(&selected, &f.slot).is_err());
    fs::write(config, valid).unwrap();
    prepared.materialise().unwrap();
    assert_eq!(
        fs::read_to_string(f.slot.join("payload.txt")).unwrap(),
        "MIRROR"
    );
}

#[test]
fn prepared_root_keeps_exact_submodule_commit_and_result_survives_source_removal() {
    let f = Fixture::new();
    f.publish("1.0.0", "ROOT", false);
    let child = Fixture::new();
    let child_commit = child.publish("1.0.0", "CHILD PIN", false);
    git(
        &f.upstream,
        &["submodule", "add", child.upstream.to_str().unwrap(), "sub"],
    );
    f.publish("1.1.0", "ROOT WITH SUB", false);
    let r = f.registry();
    let prepared = r
        .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
        .unwrap();
    child.publish("1.1.0", "CHILD MOVED", false);
    prepared.materialise().unwrap();
    drop(prepared);
    assert_eq!(
        git(&f.slot.join("sub"), &["rev-parse", "HEAD"]),
        child_commit
    );
    assert_eq!(
        fs::read_to_string(f.slot.join("sub/payload.txt")).unwrap(),
        "CHILD PIN"
    );
    fs::rename(&f.upstream, f.upstream.with_extension("gone")).unwrap();
    fs::rename(&child.upstream, child.upstream.with_extension("gone")).unwrap();
    git(&f.slot, &["fsck", "--full"]);
    git(&f.slot.join("sub"), &["fsck", "--full"]);
}

#[test]
fn fresh_sha256_repository_preserves_the_remote_object_format() {
    let f = Fixture::format("sha256");
    let commit = f.publish("1.0.0", "SHA256", false);
    assert_eq!(commit.len(), 64);
    let r = f.registry();
    let prepared = r
        .prepare_in_place(&f.selected(&r, "1.0.0"), &f.slot)
        .unwrap();
    prepared.materialise().unwrap();
    drop(prepared);
    assert_eq!(
        git(&f.slot, &["rev-parse", "--show-object-format"]),
        "sha256"
    );
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), commit);
    git(&f.slot, &["fsck", "--full"]);
}

#[test]
fn changing_checkout_local_attribute_file_cannot_hide_manifest_transformation() {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    fs::write(f.slot.join("attrs-source"), "vibe.toml -filter\n").unwrap();
    git(&f.slot, &["config", "core.attributesFile", "attrs-source"]);
    fs::write(f.upstream.join("attrs-source"), "vibe.toml filter=canary\n").unwrap();
    f.publish("1.1.0", "NEW", false);
    let r = f.registry();
    let before = tree(&f.slot);
    assert!(
        r.prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
            .is_err()
    );
    assert_eq!(tree(&f.slot), before);
}

#[test]
fn manifest_ident_expansion_refuses_before_project_checkout() {
    let f = Fixture::new();
    let old = f.publish("1.0.0", "OLD", false);
    f.seed();
    fs::write(f.upstream.join(".gitattributes"), "vibe.toml ident\n").unwrap();
    f.publish("1.1.0", "NEW", false);
    let r = f.registry();
    let before = tree(&f.slot);
    match r.prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot) {
        Err(error) => assert!(error.to_string().contains("ident"), "{error}"),
        Ok(prepared) => {
            let applied = prepared.materialise();
            panic!(
                "ident was admitted before checkout: apply={applied:?}; HEAD changed={}; worktree changed={}",
                git(&f.slot, &["rev-parse", "HEAD"]) != old,
                tree(&f.slot) != before
            );
        }
    }
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), old);
    assert_eq!(tree(&f.slot), before);
}

#[test]
fn annotated_tag_object_is_preserved_beside_its_peeled_commit() {
    let f = Fixture::new();
    let commit = f.publish("1.0.0", "TAGGED", false);
    git(
        &f.upstream,
        &["tag", "-f", "-a", "v1.0.0", "-m", "annotation retained"],
    );
    let object = git(&f.upstream, &["rev-parse", "refs/tags/v1.0.0"]);
    assert_ne!(object, commit);
    let r = f.registry();
    let p = r
        .prepare_in_place(&f.selected(&r, "1.0.0"), &f.slot)
        .unwrap();
    assert_eq!(p.resolved_commit(), commit);
    p.materialise().unwrap();
    drop(p);
    assert_eq!(git(&f.slot, &["rev-parse", "refs/tags/v1.0.0"]), object);
    assert_eq!(git(&f.slot, &["cat-file", "-t", "refs/tags/v1.0.0"]), "tag");
}

#[path = "origin_tests.rs"]
mod origin_tests;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn prepared_admission_refusal_cannot_be_hidden_by_valid_legacy_mirror() {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    fs::write(
        f.upstream.join(".gitattributes"),
        "vibe.toml filter=canary\n",
    )
    .unwrap();
    f.publish("1.1.0", "UNVERIFIABLE", false);
    let mirror = Fixture::new();
    mirror.publish("1.1.0", "LEGACY_MIRROR", false);
    let r = registry_with_mirrors(
        &f.cache,
        f.upstream.parent().unwrap().to_str().unwrap(),
        NamingConvention::Fqdn,
        vec![
            mirror
                .upstream
                .parent()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ],
        f.backend.clone(),
    );
    let before = tree(&f.slot);
    let old = git(&f.slot, &["rev-parse", "HEAD"]);
    let error = r
        .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
        .unwrap_err();
    assert!(
        matches!(&error, RegistryError::Git(GitError::PreparedValidation { source }) if matches!(source.as_ref(), GitError::PreparedCheckout { reason } if reason.contains("filter"))),
        "{error}"
    );
    assert!(!error.to_string().contains("status -1"));
    assert_eq!(tree(&f.slot), before);
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), old);
}

#[path = "source_evidence_tests.rs"]
mod source_evidence_tests;

#[test]
fn direct_and_manually_wrapped_contact_refusals_are_terminal() {
    let refusal = crate::ContactRefusal::ExcludedHost {
        endpoint: "https://excluded.invalid".into(),
    };
    for error in [
        RegistryError::ContactRefused(refusal.clone()),
        RegistryError::Git(GitError::ContactRefused(refusal)),
    ] {
        assert!(matches!(
            PreparationFailure::acquisition(error),
            PreparationFailure::Admission(_)
        ));
    }
    let absent = RegistryError::Git(GitError::RepoNotFound {
        url: "https://absent.invalid/r".into(),
    });
    assert!(matches!(
        PreparationFailure::acquisition(absent),
        PreparationFailure::Acquisition(_)
    ));
}
