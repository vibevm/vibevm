use super::*;
use crate::package_git_approval::seed_selected_ready_for_test;
use crate::package_git_objects::{self, ObjectInspection};
use crate::package_git_parent::tests::{World, binary, limits, world};
use crate::package_git_parent::{self, ParentInspection};
use crate::package_git_plan::{
    RawRef, RefSnapshot, RepositoryIdentity, RepositoryObservation, TagRef,
};
use crate::package_git_push::tests::local_bare_tests::{bounded_output, porcelain_receipt};
use crate::package_git_push::tests::{
    FakePushPort, FakeRun, PushPortError, PushRequest, Snapshot, drive_fake_run,
};
use crate::package_git_ready::ReadyLimits;
use crate::package_git_tree::git::LocalGit;
use std::process::Command;

fn direct_ready(
    binary: &Path,
) -> (
    World,
    tempfile::TempDir,
    ApprovalHome,
    SubmissionNamespace,
    String,
    ReadyLimits,
) {
    let world = world(binary, false);
    let limits = limits();
    assert!(matches!(
        package_git_parent::stage_local_parent_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            binary,
            limits,
        ),
        ParentInspection::ClosureCompleteButNotReady { .. }
    ));
    assert!(matches!(
        package_git_objects::stage_existing_main_objects_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            binary,
            limits,
        ),
        ObjectInspection::ObjectsCompleteButNotReady { .. }
    ));
    let selected = ReadyLimits {
        census: limits.census,
        parent: Some(limits),
    };
    assert!(matches!(
        package_git_ready::seal_local_ready(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            Some(&world.origin.project),
            binary,
            selected,
        ),
        ReadyInspection::ReadyForApproval { .. }
    ));
    let (settings, home, namespace, approval_id) = seed_selected_ready_for_test(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        &world.origin.project,
        binary,
        selected,
    );
    (world, settings, home, namespace, approval_id, selected)
}

struct DirectPort<'a> {
    remote: &'a Project,
    objects: Project,
    binary: &'a Path,
    identity: RepositoryIdentity,
    version_tag_ref: String,
    oid_len: usize,
    observations: usize,
    calls: usize,
    lose_reply: bool,
    drift_on_push: Option<&'static str>,
    foreign_main: String,
}

impl<'a> DirectPort<'a> {
    fn new(world: &'a World, binary: &'a Path) -> Self {
        let objects = journal::objects(&world.lease, &world.run_id, false)
            .unwrap()
            .unwrap();
        Self {
            remote: &world.origin.project,
            objects,
            binary,
            identity: match &world.plan.before_repo {
                RepositoryObservation::Present(value) => value.clone(),
                _ => panic!("direct fixture lacks selected repository"),
            },
            version_tag_ref: world.plan.version_tag_ref.clone(),
            oid_len: world.plan.oid_hex_len,
            observations: 0,
            calls: 0,
            lose_reply: false,
            drift_on_push: None,
            foreign_main: world.origin.previous.clone(),
        }
    }
    fn git(&self) -> LocalGit<'_> {
        LocalGit::new(self.binary, &self.objects, self.oid_len).unwrap()
    }
    fn remote_path(&self) -> &str {
        self.remote.root_path().to_str().unwrap()
    }
    fn root_unchanged(&self) -> Result<(), String> {
        let named = Project::open(self.remote.root_path()).map_err(|e| e.to_string())?;
        if named.root_correlation_token().map_err(|e| e.to_string())?
            != self
                .remote
                .root_correlation_token()
                .map_err(|e| e.to_string())?
        {
            return Err("selected direct bare root changed".into());
        }
        Ok(())
    }
    fn read_refs(&self) -> Result<RefSnapshot, String> {
        self.root_unchanged()?;
        let peeled = format!("{}^{{}}", self.version_tag_ref);
        let command = self
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
            .map_err(|e| e.to_string())?;
        let (status, stdout, _) = bounded_output(command, 4096)?;
        if !status.success() {
            return Err(format!("direct ls-remote exited {status}"));
        }
        let text = std::str::from_utf8(&stdout).map_err(|e| e.to_string())?;
        let mut main = None;
        let mut raw = None;
        let mut peeled_oid = None;
        for line in text.lines() {
            let (oid, reference) = line.split_once('\t').ok_or("malformed direct ref row")?;
            if oid.len() != self.oid_len || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("malformed direct ref OID".into());
            }
            if reference == "refs/heads/main" {
                if main.replace(oid.to_owned()).is_some() {
                    return Err("duplicate main".into());
                }
            } else if reference == self.version_tag_ref {
                if raw.replace(oid.to_owned()).is_some() {
                    return Err("duplicate raw tag".into());
                }
            } else if reference == peeled {
                if peeled_oid.replace(oid.to_owned()).is_some() {
                    return Err("duplicate peeled tag".into());
                }
            } else {
                return Err("foreign direct ref row".into());
            }
        }
        self.root_unchanged()?;
        Ok(RefSnapshot {
            main: main.map_or(RawRef::Absent, |oid| RawRef::Present { oid }),
            version_tag: match (raw, peeled_oid) {
                (None, None) => TagRef::Absent,
                (Some(raw_oid), Some(peeled_commit_oid)) => TagRef::Present {
                    raw_oid,
                    peeled_commit_oid,
                },
                (Some(raw_oid), None) => TagRef::Present {
                    peeled_commit_oid: raw_oid.clone(),
                    raw_oid,
                },
                _ => return Err("peeled tag without raw tag".into()),
            },
        })
    }
    fn inject_ref_drift(&self, selected: &str) -> Result<(), String> {
        let reference = if selected == "main" {
            "refs/heads/main"
        } else {
            &self.version_tag_ref
        };
        let result = Command::new(self.binary)
            .env_clear()
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("HOME", self.remote.root_path())
            .env("USERPROFILE", self.remote.root_path())
            .env("PATH", self.binary.parent().unwrap())
            .arg(format!("--git-dir={}", self.remote.root_path().display()))
            .args(["update-ref", reference, &self.foreign_main])
            .output()
            .map_err(|e| e.to_string())?;
        if !result.status.success() {
            return Err(format!(
                "concurrent direct ref move exited {}",
                result.status
            ));
        }
        Ok(())
    }
}

impl FakePushPort for DirectPort<'_> {
    fn observe(
        &mut self,
        repository: &RepositoryIdentity,
        version_tag_ref: &str,
    ) -> Result<Snapshot, PushPortError> {
        (|| -> Result<Snapshot, String> {
            if repository != &self.identity || version_tag_ref != self.version_tag_ref {
                return Err("foreign observation target".into());
            }
            self.observations += 1;
            Ok(Snapshot {
                repository: RepositoryObservation::Present(self.identity.clone()),
                refs: self.read_refs()?,
            })
        })()
        .map_err(|_| PushPortError::Uncertain)
    }
    fn atomic_push(
        &mut self,
        request: PushRequest<'_>,
    ) -> Result<crate::package_git_push::PushReceipt, PushPortError> {
        (|| -> Result<crate::package_git_push::PushReceipt, String> {
            self.calls += 1;
            if request.repository != &self.identity
                || request.version_tag_ref != self.version_tag_ref
            {
                return Err("foreign selected DirectGitExisting target".into());
            }
            if let Some(selected) = self.drift_on_push {
                self.inject_ref_drift(selected)?;
            }
            let before_main = match &request.before_refs.main {
                RawRef::Absent => "",
                RawRef::Present { oid } => oid,
            };
            let before_tag = match &request.before_refs.version_tag {
                TagRef::Absent => "",
                TagRef::Present { raw_oid, .. } => raw_oid,
            };
            let main_lease = format!("--force-with-lease=refs/heads/main:{before_main}");
            let tag_lease = format!(
                "--force-with-lease={}:{}",
                request.version_tag_ref, before_tag
            );
            let main_refspec = format!("{}:refs/heads/main", request.commit_oid);
            let tag_refspec = format!("{}:{}", request.tag_oid, request.version_tag_ref);
            let git = LocalGit::new(self.binary, request.objects, self.oid_len)
                .map_err(|e| e.to_string())?;
            let command = git
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
            let (status, stdout, _) = bounded_output(command, 4096)?;
            self.root_unchanged()?;
            if !status.success() {
                return Err(format!("direct atomic push exited {status}"));
            }
            let receipt = porcelain_receipt(
                &stdout,
                "refs/heads/main",
                request.version_tag_ref,
                request.commit_oid,
                request.tag_oid,
            )?;
            if self.read_refs()? != *request.desired_refs {
                return Err("direct post refs differ".into());
            }
            if self.lose_reply {
                return Err("injected lost direct push response".into());
            }
            Ok(receipt)
        })()
        .map_err(|_| PushPortError::Uncertain)
    }
}

fn fake_run<'a>(
    world: &'a World,
    home: &'a ApprovalHome,
    namespace: &'a SubmissionNamespace,
    binary: &'a Path,
    limits: ReadyLimits,
) -> FakeRun<'a> {
    FakeRun {
        home,
        lease: &world.lease,
        namespace,
        scope: &world.scope,
        key: "parent",
        package_source: &world.package,
        local_bare_source: Some(&world.origin.project),
        binary,
        limits,
    }
}

#[test]
fn parented_direct_git_push_and_terminal_cold_without_source() {
    let Some(binary) = binary() else { return };
    let (world, _settings, home, namespace, id, selected) = direct_ready(&binary);
    let mut port = DirectPort::new(&world, &binary);
    let ready_status = package_git_ready::inspect_ready_seal_cold(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        Some(&world.package),
        Some(&world.origin.project),
        &binary,
        selected,
    );
    assert!(
        matches!(ready_status, ReadyInspection::ReadyForApproval { .. }),
        "{ready_status:?}"
    );
    let result = drive_fake_run(
        &fake_run(&world, &home, &namespace, &binary, selected),
        &id,
        &mut port,
    );
    assert!(
        matches!(result, PushInspection::Landed { .. }),
        "{result:?}"
    );
    assert_eq!(port.calls, 1);
    assert_eq!(port.observations, 5);
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "repo-create-pending")
            .unwrap()
            .is_none()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "repo-create")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        seal_result(
            &home,
            &world.lease,
            &namespace,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            selected,
            &id
        ),
        ResultInspection::Completed { .. }
    ));
    assert!(matches!(
        inspect_cold(
            &home,
            &world.lease,
            &namespace,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            selected,
            &id
        ),
        ResultInspection::Completed { .. }
    ));
    drop(port);
    drop(world.lease);
    let reopened = LifecycleLease::acquire(world.root.path()).unwrap();
    assert!(matches!(
        inspect_cold(
            &home,
            &reopened,
            &namespace,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            selected,
            &id
        ),
        ResultInspection::Completed { .. }
    ));
}

#[test]
fn direct_push_lost_reply_retains_partial_and_never_replays() {
    let Some(binary) = binary() else { return };
    let (world, _settings, home, namespace, id, selected) = direct_ready(&binary);
    let mut port = DirectPort::new(&world, &binary);
    port.lose_reply = true;
    assert!(matches!(
        drive_fake_run(
            &fake_run(&world, &home, &namespace, &binary, selected),
            &id,
            &mut port
        ),
        PushInspection::RecoveryRequired { .. }
    ));
    assert!(matches!(
        inspect_cold(
            &home,
            &world.lease,
            &namespace,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            selected,
            &id
        ),
        ResultInspection::PartialRecovery { .. }
    ));
    assert!(matches!(
        {
            let mut historical = fake_run(&world, &home, &namespace, &binary, selected);
            historical.local_bare_source = None;
            drive_fake_run(&historical, &id, &mut port)
        },
        PushInspection::PendingRecoveryRequired { .. }
    ));
    assert_eq!(port.calls, 1);
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, RESULT)
            .unwrap()
            .is_none()
    );
}

#[test]
fn direct_concurrent_raw_main_or_tag_move_rejects_atomic_push() {
    let Some(binary) = binary() else { return };
    for selected_ref in ["main", "tag"] {
        let (world, _settings, home, namespace, id, selected) = direct_ready(&binary);
        let mut port = DirectPort::new(&world, &binary);
        port.drift_on_push = Some(selected_ref);
        assert!(matches!(
            drive_fake_run(
                &fake_run(&world, &home, &namespace, &binary, selected),
                &id,
                &mut port
            ),
            PushInspection::RecoveryRequired { .. }
        ));
        assert_eq!(port.calls, 1);
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, "atomic-push-pending")
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, "atomic-push")
                .unwrap()
                .is_none()
        );
        let after = port.read_refs().unwrap();
        match selected_ref {
            "main" => {
                assert_eq!(
                    after.main,
                    RawRef::Present {
                        oid: world.origin.previous.clone()
                    }
                );
                assert_eq!(after.version_tag, world.plan.before_refs.version_tag);
            }
            "tag" => {
                assert_eq!(after.main, world.plan.before_refs.main);
                assert_eq!(
                    after.version_tag,
                    TagRef::Present {
                        raw_oid: world.origin.previous.clone(),
                        peeled_commit_oid: world.origin.previous.clone()
                    }
                );
            }
            _ => unreachable!(),
        }
    }
}
