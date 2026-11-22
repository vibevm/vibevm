//! Explicit non-delivery attachment to one already-READY native conversation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HERMES-PI-NATIVE");
use anyhow::{Context, Result, bail};
use specmark::spec;
use std::path::Path;
use std::process::Command;
use vibe_app::Interrupt;

pub(crate) struct Request {
    session: String,
    profile_args: Vec<String>,
}

/// Consume only an explicit Vibe selector before the native argument delimiter.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HERMES-PI-NATIVE")]
pub(crate) fn parse(args: &[String]) -> Result<Option<Request>> {
    let before = args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(args.len());
    if !args[..before]
        .iter()
        .any(|arg| arg.starts_with("--vibe-ready-session"))
    {
        return Ok(None);
    }
    let mut session = None;
    let mut profile_args = Vec::new();
    let mut opted = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--vibe-ready-session=transport" if !opted => opted = true,
            "--vibe-ready-session=conversation" => bail!(
                "READY conversation instruction delivery is unavailable: the native stable/context boundary is not exposed; choose the owned route or explicitly request transport without adding instructions"
            ),
            "--resume" if session.is_none() => {
                index += 1;
                let value = args
                    .get(index)
                    .context("READY transport requires an exact --resume ID")?;
                if value.is_empty() || value.starts_with('-') || value.len() > 512 {
                    bail!("READY transport requires an exact --resume ID");
                }
                session = Some(value.clone());
            }
            "--profile" if profile_args.is_empty() => {
                index += 1;
                let value = args.get(index).context("missing native profile")?;
                if value.is_empty() || value.starts_with('-') {
                    bail!("missing native profile");
                }
                profile_args.extend(["--profile".to_owned(), value.clone()]);
            }
            "--" if index + 1 == args.len() => {}
            _ => bail!(
                "READY transport supports only --vibe-ready-session=transport, exact --resume ID and native --profile; strict delivery, query, worktree and session-changing options are unavailable"
            ),
        }
        index += 1;
    }
    if !opted {
        bail!("explicit READY transport selection required; strict delivery is unavailable");
    }
    Ok(Some(Request {
        session: session.context("READY transport requires an exact --resume ID")?,
        profile_args,
    }))
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HERMES-PI-NATIVE")]
pub(crate) fn run(settings: &Path, program: &Path, request: Request) -> Result<i32> {
    let runtime = super::hermes_runtime::discover(program)?;
    super::hermes_runtime::admit_ready(&runtime.root)?;
    let entry = super::publish_asset(
        settings,
        "hermes",
        "py",
        include_bytes!("hermes_ready_session.py"),
    )?;
    eprintln!(
        "Vibe is connecting to an existing conversation; this launch does NOT add Vibe instructions. Existing conversation instructions are retained."
    );
    let signals = crate::commands::app::interrupt::Signals::install()?;
    let code = "import runpy,sys;root,entry,*args=sys.argv[1:];sys.path.insert(0,root);sys.argv=[entry,*args];runpy.run_path(entry,run_name='__main__')";
    let child = Command::new(runtime.python)
        .args(["-I", "-B", "-c", code])
        .arg(runtime.root)
        .arg(entry)
        .arg(request.session)
        .args(request.profile_args)
        .spawn()?;
    let mut child = OwnedViewer(child);
    loop {
        signals.poll(Some(child.0.id()));
        if let Some(status) = child.0.try_wait()? {
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

struct OwnedViewer(std::process::Child);
impl Drop for OwnedViewer {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            // Only the invocation-owned viewer, never the serving gateway.
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| (*s).to_owned()).collect()
    }
    #[test]
    fn explicit_transport_and_delimiter() {
        assert!(parse(&args(&["--resume", "id"])).unwrap().is_none());
        assert!(
            parse(&args(&["--", "--vibe-ready-session=transport"]))
                .unwrap()
                .is_none()
        );
        let request = parse(&args(&[
            "--vibe-ready-session=transport",
            "--resume",
            "id",
            "--",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(request.session, "id");
        assert!(request.profile_args.is_empty());
    }
    #[test]
    fn rejects_delivery_implicit_scope_and_extra_arguments() {
        for words in [
            vec!["--vibe-ready-session=strict", "--resume", "id"],
            vec!["--vibe-ready-session=conversation", "--resume", "id"],
            vec!["--vibe-ready-session=transport"],
            vec![
                "--vibe-ready-session=transport",
                "--resume",
                "id",
                "--",
                "prompt",
            ],
            vec![
                "--vibe-ready-session=transport",
                "--resume",
                "id",
                "--worktree",
            ],
            vec![
                "--vibe-ready-session=transport",
                "--resume",
                "id",
                "--resume",
                "other",
            ],
        ] {
            assert!(parse(&args(&words)).is_err());
        }
    }
}
