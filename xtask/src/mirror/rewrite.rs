//! Explicit exact-ref leased rewrite for an owner-authorized source-history replacement.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::{Mode, Target, git, refresh_tracking, remote_ref, rev_parse, short};

pub(super) fn run(
    root: &Path,
    targets: &[Target],
    full_ref: &str,
    source: &str,
    raw_leases: &[String],
) -> Result<()> {
    let short_ref = configured_ref(full_ref)?;
    let checked = git(root, &["check-ref-format", full_ref])?;
    if !checked.status.success() {
        bail!("--ref is not a valid full Git ref: {full_ref}");
    }
    let new_oid = source_oid(root, full_ref, source)?;
    let leases = parse_leases(raw_leases)?;
    let push_targets = targets
        .iter()
        .filter(|target| matches!(target.mode, Mode::Push))
        .collect::<Vec<_>>();
    if push_targets.is_empty() {
        bail!("mirror rewrite-ref has no push targets");
    }
    for target in &push_targets {
        if !target_allows(target, full_ref, short_ref) {
            bail!(
                "target `{}` does not allow rewritten ref `{full_ref}` in mirrors.toml",
                target.name
            );
        }
        let expected = leases
            .get(&target.name)
            .with_context(|| format!("missing exact lease for target `{}`", target.name))?;
        let observed = remote_ref(root, &target.url, full_ref)?;
        if &observed != expected {
            bail!(
                "lease mismatch for {} {full_ref}: expected {}, observed {}",
                target.name,
                display_lease(expected),
                display_lease(&observed)
            );
        }
    }
    if leases.len() != push_targets.len() {
        let unknown = leases
            .keys()
            .filter(|name| !push_targets.iter().any(|target| &target.name == *name))
            .cloned()
            .collect::<Vec<_>>();
        bail!("lease set contains unknown or non-push targets: {unknown:?}");
    }

    let remotes = super::named_remotes(root).unwrap_or_default();
    for target in push_targets {
        let expected = leases.get(&target.name).expect("preflight required lease");
        let lease_arg = format!(
            "--force-with-lease={full_ref}:{}",
            expected.as_deref().unwrap_or("")
        );
        let refspec = format!("{new_oid}:{full_ref}");
        let output = git(root, &["push", &lease_arg, &target.url, &refspec])?;
        if !output.status.success() {
            bail!(
                "leased rewrite failed for {} {full_ref}: {}",
                target.name,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let after = remote_ref(root, &target.url, full_ref)?;
        if after.as_deref() != Some(new_oid.as_str()) {
            bail!(
                "leased rewrite for {} {full_ref} returned success but remote is {}",
                target.name,
                display_lease(&after)
            );
        }
        println!(
            "  rewrite {} {full_ref} -> {}",
            target.name,
            short(&new_oid)
        );
        if full_ref.starts_with("refs/heads/") {
            refresh_tracking(root, &remotes, &target.url, short_ref);
        }
    }
    println!("mirror rewrite-ref: every exact lease applied and re-verified.");
    Ok(())
}

fn source_oid(root: &Path, full_ref: &str, source: &str) -> Result<String> {
    let source_revision = format!("{source}^{{commit}}");
    let commit = rev_parse(root, &source_revision)?;
    if full_ref.starts_with("refs/tags/") {
        let object = rev_parse(root, source)?;
        if object == commit {
            return Ok(commit);
        }
        return Ok(object);
    }
    Ok(commit)
}

fn configured_ref(full_ref: &str) -> Result<&str> {
    if let Some(branch) = full_ref.strip_prefix("refs/heads/") {
        if valid_short_ref(branch) {
            return Ok(branch);
        }
    } else if let Some(tag) = full_ref.strip_prefix("refs/tags/") {
        if valid_short_ref(tag) {
            return Ok(tag);
        }
    }
    bail!("--ref must be one full refs/heads/NAME or refs/tags/NAME spelling")
}

fn valid_short_ref(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains(['~', '^', ':', '?', '*', '[', '\\', ' '])
}

fn target_allows(target: &Target, full_ref: &str, short_ref: &str) -> bool {
    if full_ref.starts_with("refs/tags/") {
        target.refs.iter().any(|value| value == "tags")
    } else {
        target.refs.iter().any(|value| value == short_ref)
    }
}

fn parse_leases(raw: &[String]) -> Result<BTreeMap<String, Option<String>>> {
    let mut leases = BTreeMap::new();
    for value in raw {
        let (target, expected) = value
            .split_once('=')
            .with_context(|| format!("lease `{value}` must be TARGET=OID|absent"))?;
        if target.is_empty() {
            bail!("lease target is empty");
        }
        let expected = if expected == "absent" {
            None
        } else if is_oid(expected) {
            Some(expected.to_string())
        } else {
            bail!("lease for `{target}` must be a full lowercase Git OID or `absent`");
        };
        if leases.insert(target.to_string(), expected).is_some() {
            bail!("duplicate lease for target `{target}`");
        }
    }
    Ok(leases)
}

fn is_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn display_lease(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("absent")
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::{configured_ref, parse_leases, source_oid};

    #[test]
    fn full_ref_and_exact_leases_are_strict() {
        assert_eq!(configured_ref("refs/heads/next").unwrap(), "next");
        assert_eq!(configured_ref("refs/tags/v1.0.0").unwrap(), "v1.0.0");
        assert!(configured_ref("next").is_err());
        assert!(configured_ref("refs/heads/../main").is_err());
        let leases = parse_leases(&[
            "gitverse=absent".into(),
            "github=0123456789abcdef0123456789abcdef01234567".into(),
        ])
        .unwrap();
        assert_eq!(leases["gitverse"], None);
        assert_eq!(
            leases["github"].as_deref(),
            Some("0123456789abcdef0123456789abcdef01234567")
        );
        assert!(parse_leases(&["github=short".into()]).is_err());
        assert!(parse_leases(&["github=absent".into(), "github=absent".into()]).is_err());
    }

    #[test]
    fn lease_push_shape_is_exact_ref_only() {
        let full_ref = "refs/heads/main";
        let expected = "0123456789abcdef0123456789abcdef01234567";
        let oid = "89abcdef0123456789abcdef0123456789abcdef";
        let lease = format!("--force-with-lease={full_ref}:{expected}");
        let refspec = format!("{oid}:{full_ref}");
        assert_eq!(
            [lease.as_str(), refspec.as_str()],
            [
                "--force-with-lease=refs/heads/main:0123456789abcdef0123456789abcdef01234567",
                "89abcdef0123456789abcdef0123456789abcdef:refs/heads/main"
            ]
        );
        assert!(!lease.contains("--force "));
        assert!(!refspec.starts_with('+'));
    }

    #[test]
    fn annotated_tag_source_keeps_its_tag_object_oid() {
        let repository = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["config", "user.name", "mirror test"],
            vec!["config", "user.email", "mirror@example.invalid"],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(repository.path())
                    .status()
                    .unwrap()
                    .success()
            );
        }
        std::fs::write(repository.path().join("file"), b"one").unwrap();
        for args in [
            vec!["add", "file"],
            vec!["commit", "--quiet", "-m", "one"],
            vec!["tag", "-a", "preview", "-m", "preview"],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(repository.path())
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let tag_object = super::rev_parse(repository.path(), "refs/tags/preview").unwrap();
        let commit = super::rev_parse(repository.path(), "refs/tags/preview^{commit}").unwrap();
        assert_ne!(tag_object, commit, "fixture must be an annotated tag");
        assert_eq!(
            source_oid(repository.path(), "refs/tags/preview", "refs/tags/preview").unwrap(),
            tag_object
        );
        assert_eq!(
            source_oid(repository.path(), "refs/heads/main", "refs/tags/preview").unwrap(),
            commit
        );
    }
}
