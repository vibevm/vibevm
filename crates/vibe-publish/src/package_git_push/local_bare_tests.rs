use super::*;
use crate::package_git_plan::{RawRef, TagRef};
use crate::package_git_tree::git::LocalGit;
use std::path::PathBuf;
use std::process::Command;

#[path = "local_bare_output.rs"]
mod output;
pub(crate) use output::{bounded_output, porcelain_receipt};

pub(crate) struct LocalBarePort {
    remote: tempfile::TempDir,
    remote_project: Project,
    objects: Project,
    binary: PathBuf,
    identity: RepositoryIdentity,
    oid_len: usize,
    version_tag_ref: String,
    observations: usize,
    calls: usize,
    external_ref: Option<&'static str>,
    external_before_protected_push: Option<&'static str>,
    desired_before_protected_push: Option<&'static str>,
    external_calls: usize,
    external_commit_oid: Option<String>,
    external_tag_oid: Option<String>,
    last_push_exit_zero: Option<bool>,
    last_porcelain: Option<Vec<u8>>,
    commit_oid: String,
    tag_oid: String,
}

impl LocalBarePort {
    fn new(world: &World, approval_id: &str) -> Self {
        Self::new_run(
            &FakeRun {
                home: &world.home,
                lease: &world.lease,
                namespace: &world.namespace,
                scope: &world.scope,
                key: "approval",
                package_source: &world.source,
                local_bare_source: None,
                binary: &world.binary,
                limits: limits(),
            },
            approval_id,
        )
    }

    pub(super) fn new_run(input: &FakeRun<'_>, approval_id: &str) -> Self {
        Self::new_profile(
            &ProfileRun {
                home: input.home,
                lease: input.lease,
                namespace: input.namespace,
                scope: input.scope,
                key: input.key,
                package_source: input.package_source,
                binary: input.binary,
                profile: ReadyProfile::NoGitlinks {
                    limits: input.limits,
                    local_bare_source: input.local_bare_source,
                },
            },
            approval_id,
        )
    }

    pub(crate) fn new_profile(input: &ProfileRun<'_>, approval_id: &str) -> Self {
        let proof = proved_profile(
            input.home,
            input.lease,
            input.namespace,
            input.scope,
            input.key,
            Some(input.package_source),
            input.binary,
            input.profile,
            approval_id,
        )
        .unwrap();
        let remote = tempfile::tempdir().unwrap();
        let result = Command::new(input.binary)
            .env_clear()
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("HOME", remote.path())
            .env("USERPROFILE", remote.path())
            .args(["init", "--bare", "-q"])
            .arg(remote.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "local bare init: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let remote_project = Project::open(remote.path()).unwrap();
        let objects = journal::objects(input.lease, proof.run(), false)
            .unwrap()
            .unwrap();
        let git = LocalGit::new(input.binary, &objects, proof.plan.oid_hex_len).unwrap();
        for reference in ["refs/heads/main", proof.plan.version_tag_ref.as_str()] {
            let output = git
                .command(&["show-ref", "--verify", reference], true)
                .unwrap()
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "retained store must have no source refs"
            );
        }
        Self {
            remote,
            remote_project,
            objects,
            binary: input.binary.to_path_buf(),
            identity: proof.repository,
            oid_len: proof.plan.oid_hex_len,
            version_tag_ref: proof.plan.version_tag_ref,
            observations: 0,
            calls: 0,
            external_ref: None,
            external_before_protected_push: None,
            desired_before_protected_push: None,
            external_calls: 0,
            external_commit_oid: None,
            external_tag_oid: None,
            last_push_exit_zero: None,
            last_porcelain: None,
            commit_oid: proof.seal.selected_commit_oid,
            tag_oid: proof.seal.selected_tag_oid,
        }
    }

    fn git(&self) -> LocalGit<'_> {
        LocalGit::new(&self.binary, &self.objects, self.oid_len).unwrap()
    }
    fn remote_path(&self) -> &str {
        self.remote.path().to_str().unwrap()
    }
    fn root_unchanged(&self) -> Result<(), String> {
        let reopened = Project::open(self.remote.path()).map_err(|e| e.to_string())?;
        if reopened
            .root_correlation_token()
            .map_err(|e| e.to_string())?
            != self
                .remote_project
                .root_correlation_token()
                .map_err(|e| e.to_string())?
        {
            return Err("local bare pathname changed".into());
        }
        Ok(())
    }
    fn external_move(&mut self, selected: &str) -> Result<(), String> {
        let source = tempfile::tempdir().map_err(|e| e.to_string())?;
        let remote = self.remote_path().to_owned();
        let binary = self.binary.clone();
        let run = |args: &[&str]| -> Result<String, String> {
            let output = Command::new(&binary)
                .current_dir(source.path())
                .env_clear()
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .env("GIT_TERMINAL_PROMPT", "0")
                .env("HOME", source.path())
                .env("USERPROFILE", source.path())
                .env("PATH", binary.parent().unwrap())
                .args(args)
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(format!(
                    "external local Git exited {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            String::from_utf8(output.stdout)
                .map(|s| s.trim().to_owned())
                .map_err(|e| e.to_string())
        };
        run(&["init", "-q"])?;
        std::fs::write(
            source.path().join("foreign.txt"),
            b"foreign concurrent bytes\n",
        )
        .map_err(|e| e.to_string())?;
        run(&["add", "foreign.txt"])?;
        run(&[
            "-c",
            "user.name=External",
            "-c",
            "user.email=external@example.test",
            "commit",
            "-q",
            "-m",
            "external",
        ])?;
        let commit = run(&["rev-parse", "HEAD"])?;
        let (source_ref, destination) = if selected == "main" {
            ("HEAD", "refs/heads/main")
        } else {
            run(&[
                "-c",
                "user.name=External",
                "-c",
                "user.email=external@example.test",
                "tag",
                "-a",
                "-m",
                "external",
                "v1.1.0",
            ])?;
            self.external_tag_oid = Some(run(&["rev-parse", "refs/tags/v1.1.0"])?);
            ("refs/tags/v1.1.0", self.version_tag_ref.as_str())
        };
        let refspec = format!("{source_ref}:{destination}");
        run(&["push", "--", &remote, &refspec])?;
        self.external_commit_oid = Some(commit);
        self.external_calls += 1;
        Ok(())
    }
    fn desired_move(&self, selected: &str) -> Result<(), String> {
        let source = if selected == "main" {
            &self.commit_oid
        } else {
            &self.tag_oid
        };
        let destination = if selected == "main" {
            "refs/heads/main"
        } else {
            &self.version_tag_ref
        };
        let refspec = format!("{source}:{destination}");
        let command = self
            .git()
            .command(&["push", "--", self.remote_path(), &refspec], true)
            .map_err(|e| e.to_string())?;
        let (status, _, _) = bounded_output(command, 4096)?;
        if !status.success() {
            return Err(format!("concurrent desired move exited {status}"));
        }
        Ok(())
    }
    fn read_refs(&self) -> Result<RefSnapshot, String> {
        self.root_unchanged()?;
        let peeled = format!("{}^{{}}", self.version_tag_ref);
        let output = self
            .git()
            .command(
                &[
                    "ls-remote",
                    "--",
                    self.remote_path(),
                    "refs/heads/main",
                    &self.version_tag_ref,
                    &peeled,
                ],
                true,
            )
            .map_err(|e| e.to_string())?
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() || output.stdout.len() > 4096 {
            return Err("bounded local ls-remote failed".into());
        }
        let text = std::str::from_utf8(&output.stdout).map_err(|e| e.to_string())?;
        let mut main = None;
        let mut tag = None;
        let mut peeled_oid = None;
        for line in text.lines() {
            let (oid, reference) = line.split_once('\t').ok_or("malformed local ref row")?;
            if oid.len() != self.oid_len || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("malformed local Git OID".into());
            }
            if reference == "refs/heads/main" {
                if main.replace(oid.to_owned()).is_some() {
                    return Err("duplicate main".into());
                }
            } else if reference == self.version_tag_ref {
                if tag.replace(oid.to_owned()).is_some() {
                    return Err("duplicate raw tag".into());
                }
            } else if reference == peeled {
                if peeled_oid.replace(oid.to_owned()).is_some() {
                    return Err("duplicate peeled tag".into());
                }
            } else {
                return Err("foreign ref in bounded observation".into());
            }
        }
        self.root_unchanged()?;
        Ok(RefSnapshot {
            main: main.map_or(RawRef::Absent, |oid| RawRef::Present { oid }),
            version_tag: match (tag, peeled_oid) {
                (None, None) => TagRef::Absent,
                (Some(raw_oid), Some(peeled_commit_oid)) => TagRef::Present {
                    raw_oid,
                    peeled_commit_oid,
                },
                _ => return Err("partial annotated tag observation".into()),
            },
        })
    }

    pub(crate) fn calls(&self) -> usize {
        self.calls
    }
    pub(crate) fn exact_refs(&self) -> RefSnapshot {
        self.read_refs().unwrap()
    }
    pub(super) fn race_desired_main(&mut self) {
        self.desired_before_protected_push = Some("main");
    }

    pub(crate) fn drift_tag_after_push(&self) {
        let command = self
            .git()
            .command(
                &[
                    "--git-dir",
                    self.remote_path(),
                    "update-ref",
                    "-d",
                    &self.version_tag_ref,
                ],
                true,
            )
            .unwrap();
        let (status, _, _) = bounded_output(command, 4096).unwrap();
        assert!(status.success(), "local tag drift: {status}");
    }
}

impl FakePushPort for LocalBarePort {
    fn observe(&mut self) -> Result<Snapshot, String> {
        self.observations += 1;
        if self.observations == 3
            && let Some(reference) = self.external_ref
        {
            self.external_move(reference)?;
        }
        Ok(Snapshot {
            repository: RepositoryObservation::Present(self.identity.clone()),
            refs: self.read_refs()?,
        })
    }
    fn atomic_push(&mut self, request: PushRequest<'_>) -> Result<PushReceipt, String> {
        self.calls += 1;
        if request.repository != &self.identity || request.version_tag_ref != self.version_tag_ref {
            return Err("foreign local target".into());
        }
        if let Some(reference) = self.external_before_protected_push {
            self.external_move(reference)?;
        }
        if let Some(reference) = self.desired_before_protected_push {
            self.desired_move(reference)?;
        }
        let main_before = match &request.before_refs.main {
            RawRef::Absent => "",
            RawRef::Present { oid } => oid,
        };
        let tag_before = match &request.before_refs.version_tag {
            TagRef::Absent => "",
            TagRef::Present { raw_oid, .. } => raw_oid,
        };
        let main_lease = format!("--force-with-lease=refs/heads/main:{main_before}");
        let tag_lease = format!(
            "--force-with-lease={}:{}",
            request.version_tag_ref, tag_before
        );
        let main_refspec = format!("{}:refs/heads/main", request.commit_oid);
        let tag_refspec = format!("{}:{}", request.tag_oid, request.version_tag_ref);
        let command = self
            .git()
            .command(
                &[
                    "push",
                    "--porcelain",
                    "--atomic",
                    &main_lease,
                    &tag_lease,
                    "--",
                    self.remote_path(),
                    &main_refspec,
                    &tag_refspec,
                ],
                true,
            )
            .map_err(|e| e.to_string())?;
        let (status, stdout, _stderr) = bounded_output(command, 4096)?;
        self.last_push_exit_zero = Some(status.success());
        self.last_porcelain = Some(stdout.clone());
        self.root_unchanged()?;
        if !status.success() {
            return Err(format!("atomic local push exited {status}"));
        }
        let receipt = porcelain_receipt(
            &stdout,
            "refs/heads/main",
            request.version_tag_ref,
            request.commit_oid,
            request.tag_oid,
        )
        .map_err(|e| format!("{e}; porcelain={}", String::from_utf8_lossy(&stdout)))?;
        if self.read_refs()? != *request.desired_refs {
            return Err("local push did not produce desired refs".into());
        }
        Ok(receipt)
    }
}

#[test]
fn retained_bare_objects_push_exact_oid_refs_atomically_without_local_source_refs() {
    let Some(binary) = binary() else { return };
    let (world, id) = ready_world(&binary);
    let mut local = LocalBarePort::new(&world, &id);
    let status = drive_fake(&world, &id, &mut local);
    assert!(
        matches!(status, PushInspection::Landed { .. }),
        "{status:?}"
    );
    assert_eq!(local.calls, 1);
    assert_eq!(local.observations, 5);
}

#[test]
fn concurrent_local_bare_main_or_tag_drift_after_pending_blocks_owner_push() {
    let Some(binary) = binary() else { return };
    for reference in ["main", "tag"] {
        let (world, id) = ready_world(&binary);
        let mut local = LocalBarePort::new(&world, &id);
        local.external_ref = Some(reference);
        let result = drive_fake(&world, &id, &mut local);
        assert!(
            matches!(result, PushInspection::RecoveryRequired { .. }),
            "{reference}: {result:?}"
        );
        assert_eq!(local.calls, 0);
        assert_eq!(local.external_calls, 1);
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, PENDING)
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, LANDED)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn exact_raw_leases_reject_concurrent_ref_move_inside_atomic_push() {
    let Some(binary) = binary() else { return };
    for reference in ["main", "tag"] {
        let (world, id) = ready_world(&binary);
        let mut local = LocalBarePort::new(&world, &id);
        local.external_before_protected_push = Some(reference);
        let result = drive_fake(&world, &id, &mut local);
        assert!(
            matches!(result, PushInspection::RecoveryRequired { .. }),
            "{reference}: {result:?}"
        );
        assert_eq!(local.calls, 1);
        assert_eq!(local.external_calls, 1);
        let refs = local.read_refs().unwrap();
        match reference {
            "main" => {
                let foreign = local.external_commit_oid.clone().unwrap();
                assert_ne!(foreign, local.commit_oid);
                assert_eq!(refs.main, RawRef::Present { oid: foreign });
                assert_eq!(refs.version_tag, TagRef::Absent);
            }
            "tag" => {
                let foreign_tag = local.external_tag_oid.clone().unwrap();
                assert_ne!(foreign_tag, local.tag_oid);
                assert_eq!(refs.main, RawRef::Absent);
                assert_eq!(
                    refs.version_tag,
                    TagRef::Present {
                        raw_oid: foreign_tag,
                        peeled_commit_oid: local.external_commit_oid.clone().unwrap(),
                    }
                );
            }
            _ => unreachable!(),
        }
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, PENDING)
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, LANDED)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn desired_oid_race_is_noop_for_one_ref_and_never_attributed_to_owner() {
    let Some(binary) = binary() else { return };
    for reference in ["main", "tag"] {
        let (world, id) = ready_world(&binary);
        let mut local = LocalBarePort::new(&world, &id);
        local.desired_before_protected_push = Some(reference);
        let result = drive_fake(&world, &id, &mut local);
        assert!(
            matches!(result, PushInspection::RecoveryRequired { .. }),
            "{reference}: {result:?}"
        );
        assert_eq!(local.calls, 1);
        assert_eq!(local.last_push_exit_zero, Some(true));
        let porcelain = std::str::from_utf8(local.last_porcelain.as_ref().unwrap()).unwrap();
        assert!(
            porcelain.lines().any(|line| line.starts_with("=\t")),
            "{porcelain}"
        );
        let proof = proved(
            &world.home,
            &world.lease,
            &world.namespace,
            &world.scope,
            "approval",
            Some(&world.source),
            None,
            &binary,
            limits(),
            &id,
        )
        .unwrap();
        assert_eq!(local.read_refs().unwrap(), proof.desired_refs);
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, PENDING)
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, LANDED)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn porcelain_requires_two_distinct_exact_updating_rows() {
    let main = "*\ta:refs/heads/main\t[new branch]\n";
    let tag = "*\tb:refs/tags/v1.1.0\t[new tag]\n";
    for bad in [
        main.to_owned(),
        format!("{main}{main}{tag}"),
        format!("{main}*\tb:refs/tags/foreign\t[new tag]\n"),
        format!("{main}=\tb:refs/tags/v1.1.0\t[up to date]\n"),
        format!("{main}malformed\n"),
        format!("{main}{}", tag.trim_end_matches('\n')),
    ] {
        assert!(
            porcelain_receipt(
                bad.as_bytes(),
                "refs/heads/main",
                "refs/tags/v1.1.0",
                "a",
                "b"
            )
            .is_err()
        );
    }
    assert!(
        porcelain_receipt(
            format!("{main}{tag}").as_bytes(),
            "refs/heads/main",
            "refs/tags/v1.1.0",
            "a",
            "b"
        )
        .is_ok()
    );
}
