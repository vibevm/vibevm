//! One owned native supervisor stays alive across project-prefix selection.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use vibe_app::Interrupt;
use vibe_safefs::Project;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Ready {
    session: String,
    cwd: PathBuf,
    mode: String,
}

pub(crate) struct Selection {
    child: Child,
    temporary: tempfile::TempDir,
    signals: crate::commands::app::interrupt::Signals,
    ready: Ready,
    route: Option<crate::commands::tool::px::AgentRoute>,
}

/// Only native cwd/session selectors require the two-stage launch.
pub(crate) fn requested(args: &[String]) -> bool {
    args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--resume"
                | "-r"
                | "-c"
                | "--continue"
                | "-w"
                | "--worktree"
                | "-z"
                | "--oneshot"
                | "--in"
                | "-q"
                | "--query"
                | "--query-file"
                | "-Q"
                | "--quiet"
        ) || [
            "--resume=",
            "--continue=",
            "--oneshot=",
            "--in=",
            "--query=",
            "--query-file=",
        ]
        .iter()
        .any(|prefix| arg.starts_with(prefix))
    })
}

impl Selection {
    pub(crate) fn start(
        settings: &Path,
        program: &Path,
        cwd: &Path,
        args: &[String],
    ) -> Result<Self> {
        let runtime = super::hermes_runtime::discover(program)?;
        let carrier = super::publish_asset(
            settings,
            "hermes",
            "py",
            include_bytes!("hermes_bootstrap.py"),
        )?;
        let entry =
            super::publish_asset(settings, "hermes", "py", include_bytes!("hermes_entry.py"))?;
        let selector = super::publish_asset(
            settings,
            "hermes",
            "py",
            include_bytes!("hermes_selection.py"),
        )?;
        let temporary = tempfile::Builder::new()
            .prefix("vibe-hermes-selection-")
            .tempdir()?;
        let capability = Project::open(temporary.path())?;
        let directory = capability.dir(&[], false)?;
        for name in ["ready.json", "ack.json"] {
            capability.publish_new_in_with_mode(&directory, name, b"", private_mode())?;
        }
        let signals = crate::commands::app::interrupt::Signals::install()?;
        let code = "import runpy,sys;root,entry,*args=sys.argv[1:];sys.path.insert(0,root);sys.argv=[entry,*args];runpy.run_path(entry,run_name='__main__')";
        let route_required = vibe_net::current().route().mode_name() != "direct";
        let child = Command::new(&runtime.python)
            .args(["-I", "-c", code])
            .arg(&runtime.root)
            .arg(entry)
            .arg("--select")
            .arg(carrier)
            .arg(selector)
            .arg(cwd)
            .arg(temporary.path())
            .args(args)
            .env(
                "VIBE_HERMES_MANAGED_ROUTE_REQUIRED",
                if route_required { "1" } else { "0" },
            )
            .current_dir(cwd)
            .spawn()?;
        let mut selected = Self {
            child,
            temporary,
            signals,
            route: None,
            ready: Ready {
                session: String::new(),
                cwd: PathBuf::new(),
                mode: String::new(),
            },
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(150);
        loop {
            if selected.signals.poll(Some(selected.child.id())) {
                bail!("native Hermes selection interrupted");
            }
            if selected.child.try_wait()?.is_some() {
                bail!("native Hermes selection refused before prefix admission");
            }
            if std::time::Instant::now() >= deadline {
                bail!("native Hermes selection timed out");
            }
            if let Some(bytes) = capability.read_file_bounded("ready.json", 65536)?
                && let Ok(ready) = serde_json::from_slice::<Ready>(&bytes)
            {
                if ready.session.is_empty()
                    || ready.mode != "selected"
                    || !ready.cwd.is_absolute()
                    || ready.cwd.canonicalize()? != ready.cwd
                {
                    bail!("invalid native selection binding");
                }
                selected.ready = ready;
                return Ok(selected);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    pub(crate) fn cwd(&self) -> &Path {
        &self.ready.cwd
    }

    pub(crate) fn launch(mut self, prefix: Option<&Path>) -> Result<i32> {
        let interrupted = self.signals.poll(Some(self.child.id()));
        let capability = Project::open(self.temporary.path())?;
        self.route = crate::commands::tool::px::AgentRoute::selected()?;
        let ack = self
            .route
            .as_ref()
            .map(|route| route.owned_ack())
            .transpose()?;
        acknowledge(&capability, &self.ready, prefix, interrupted, ack.as_ref())?;
        loop {
            self.signals.poll(Some(self.child.id()));
            if let Some(status) = self.child.try_wait()? {
                if let Some(route) = &self.route {
                    route.close();
                }
                // The child is reaped before exit; its supervisor already released native owners.
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    return Ok(status
                        .code()
                        .unwrap_or_else(|| 128 + status.signal().unwrap_or(1)));
                }
                #[cfg(not(unix))]
                return Ok(status.code().unwrap_or(1));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

impl Drop for Selection {
    fn drop(&mut self) {
        // Close rendezvous first: a waiting supervisor refuses and runs native cleanup.
        if self.child.try_wait().ok().flatten().is_some() {
            if let Some(route) = &self.route {
                route.close();
            }
            return;
        }
        if let Ok(capability) = Project::open(self.temporary.path())
            && let Ok(directory) = capability.dir(&[], false)
        {
            let _ = capability.write_atomic_in(&directory, "ack.json", b"{\"cancelled\":true}");
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(240);
        while std::time::Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                if let Some(route) = &self.route {
                    route.close();
                }
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        eprintln!(
            "warning: native Hermes cleanup exceeded its owned-process leash; worktree state may remain; inspect native retained state, do not delete unknown paths"
        );
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(route) = &self.route {
            route.close();
        }
    }
}

fn acknowledge(
    capability: &Project,
    ready: &Ready,
    prefix: Option<&Path>,
    interrupted: bool,
    route: Option<&serde_json::Value>,
) -> Result<()> {
    if interrupted {
        bail!("native Hermes selection interrupted before prefix admission");
    }
    let directory = capability.dir(&[], false)?;
    let mut value =
        serde_json::json!({ "session": ready.session, "cwd": ready.cwd, "prefix": prefix });
    if let Some(route) = route {
        value["route"] = route.clone();
    }
    let bytes = serde_json::to_vec(&value)?;
    capability.write_atomic_in(&directory, "ack.json", &bytes)?;
    Ok(())
}

fn private_mode() -> Option<u32> {
    #[cfg(unix)]
    {
        Some(0o600)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_native_selection_arguments_enter_rendezvous() {
        for args in [
            vec!["--resume", "local-demo"],
            vec!["--resume=local-demo"],
            vec!["-r", "local-demo"],
            vec!["-c"],
            vec!["--continue=topic"],
            vec!["-w"],
            vec!["--oneshot", "hello"],
            vec!["--in", "/project"],
        ] {
            assert!(requested(
                &args.into_iter().map(str::to_owned).collect::<Vec<_>>()
            ));
        }
        assert!(requested(&[
            "chat".into(),
            "--query".into(),
            "hello".into()
        ]));
        assert!(!requested(&["chat".into()]));
    }
    #[test]
    fn cancelled_read_fallback_never_acknowledges_frontend_start() {
        let temporary = tempfile::tempdir().unwrap();
        let capability = Project::open(temporary.path()).unwrap();
        let directory = capability.dir(&[], false).unwrap();
        capability
            .publish_new_in_with_mode(&directory, "ack.json", b"", private_mode())
            .unwrap();
        let ready = Ready {
            session: "exact-session".into(),
            cwd: temporary.path().into(),
            mode: "selected".into(),
        };
        assert!(acknowledge(&capability, &ready, None, true, None).is_err());
        assert_eq!(
            capability.read_file_bounded("ack.json", 65536).unwrap(),
            Some(Vec::new())
        );
        acknowledge(&capability, &ready, None, false, None).unwrap();
        let bytes = capability
            .read_file_bounded("ack.json", 65536)
            .unwrap()
            .unwrap();
        let ack: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(ack["prefix"].is_null());
        assert_eq!(ack["session"], "exact-session");
    }
    #[test]
    fn ready_requires_exact_bounded_vocabulary() {
        assert!(
            serde_json::from_str::<Ready>(
                r#"{"session":"x","cwd":"/x","mode":"selected","secret":"forbidden"}"#
            )
            .is_err()
        );
    }
}
