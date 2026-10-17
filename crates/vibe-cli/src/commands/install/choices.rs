//! Presentation for the shared prepared-choice operations. No local admission.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003-dep-evolution#M05E4-ATTENDED"
);
use crate::{cli::InstallArgs, output};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::io::IsTerminal;
#[cfg(test)]
use vibe_orchestrator;
use vibe_orchestrator::machine_operation::configuration_choice::ChoiceOperation;
use vibe_wire::generated::consumer_configuration::e1::request::{
    Request, Selection, SelectionIntent,
};
#[cfg(test)]
#[path = "../../../../vibe-orchestrator/src/machine_operation/configuration_choice_fixture.rs"]
mod fixture;
#[cfg(all(test, unix))]
mod m_05_e_4_cases {
    use super::*;
    use clap::Parser;
    #[test]
    fn production_install_machine_and_tty() {
        if let Ok(socket) = std::env::var("M05E4_CHILD_SOCKET") {
            let mode = std::env::var("M05E4_CHILD_MODE").unwrap();
            let mut argv = vec!["vibe".to_owned()];
            if mode != "tty" {
                argv.push("--json".into());
            }
            argv.extend([
                "install".into(),
                "--choice-host".into(),
                socket,
                "--choice-capture".into(),
                std::env::var("M05E4_CHILD_CAPTURE").unwrap(),
                "--path".into(),
                std::env::var("M05E4_CHILD_ROOT").unwrap(),
                "--assume-yes".into(),
            ]);
            if mode == "defaults" || mode == "offline" {
                argv.extend([
                    "--allow-defaults".into(),
                    "--shown-definition".into(),
                    std::env::var("M05E4_CHILD_SHOWN").unwrap(),
                ]);
            }
            if mode == "tty" {
                argv.push("--apply-choices".into());
            }
            let crate::cli::Command::Install(args) =
                crate::cli::Cli::try_parse_from(argv).unwrap().command
            else {
                panic!("install")
            };
            let ctx = output::Context::from_flags(
                false,
                mode != "tty",
                None,
                false,
                crate::cli::AgentModeArg::Auto,
            );
            let result = super::super::direct::run(&ctx, args, None, true);
            if mode == "offline" {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("offline prepared choice host")
                );
            } else {
                result.unwrap();
            }
            return;
        }
        for mode in ["missing", "defaults", "tty", "offline"] {
            let source_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let (_dir, host) = if mode == "offline" {
                fixture::online_refusing_fixture(source_calls.clone())
            } else {
                fixture::fixture()
            };
            let q = host.questions();
            let socketdir = tempfile::Builder::new()
                .prefix("e4-")
                .tempdir_in("/tmp")
                .unwrap();
            let socket = socketdir.path().join("host.sock");
            let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
            listener.set_nonblocking(true).unwrap();
            let exe = std::env::current_exe().unwrap();
            let case =
                "commands::install::choices::m_05_e_4_cases::production_install_machine_and_tty";
            let mut command = if mode == "tty" {
                let mut c = std::process::Command::new("python3");
                c.args([
                    "-c",
                    r#"import os,pty,select,sys,time
pid,fd=pty.fork()
if pid==0: os.execv(sys.argv[1],[sys.argv[1],'--exact',sys.argv[2],'--nocapture'])
buf=b''; answered=False; extras=False; deadline=time.monotonic()+30
while time.monotonic()<deadline:
 if select.select([fd],[],[],0.1)[0]:
  try: data=os.read(fd,65536)
  except OSError: break
  if not data: break
  buf+=data
  if b'choice (defaults:' in buf and not answered:
   os.write(fd,b'\x1b[B\r'); answered=True
  if b'extras (defaults:' in buf and not extras:
   os.write(fd,b'\r'); extras=True
 done,status=os.waitpid(pid,os.WNOHANG)
 if done:
  sys.stdout.buffer.write(buf);sys.exit(os.waitstatus_to_exitcode(status))
else:
 os.kill(pid,9);os.waitpid(pid,0);sys.stdout.buffer.write(buf);sys.exit(99)
_,status=os.waitpid(pid,0)
sys.stdout.buffer.write(buf);sys.exit(os.waitstatus_to_exitcode(status))
"#,
                ]);
                c.arg(&exe).arg(case);
                c
            } else {
                let mut c = std::process::Command::new(&exe);
                c.args(["--exact", case, "--nocapture"]);
                c
            };
            let mut child = command
                .env("M05E4_CHILD_SOCKET", &socket)
                .env("M05E4_CHILD_CAPTURE", host.capture())
                .env("M05E4_CHILD_ROOT", host.root())
                .env("M05E4_CHILD_SHOWN", q["shown"].as_str().unwrap())
                .env("M05E4_CHILD_MODE", mode)
                .env_remove("VIBE_UNATTENDED")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(40);
            loop {
                if let Ok((mut connection, _)) = listener.accept() {
                    host.serve_cli(&mut connection).unwrap();
                }
                if child.try_wait().unwrap().is_some() {
                    break;
                }
                if std::time::Instant::now() > deadline {
                    child.kill().unwrap();
                    panic!("choice client timed out: {mode}");
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            let output = child.wait_with_output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success(),
                "{mode}: {stdout}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            match mode {
                "offline" => {
                    assert_eq!(source_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
                    assert!(!host.root().join("vibe.lock").exists());
                    assert!(!host.root().join("vibedeps").exists());
                }
                "missing" => {
                    assert!(stdout.contains("needs_selection"));
                    assert!(!host.root().join("vibe.lock").exists());
                }
                "defaults" => {
                    assert!(stdout.contains("prepared"));
                    let start = stdout.find('{').unwrap();
                    let actual = serde_json::Deserializer::from_str(&stdout[start..])
                        .into_iter::<Value>()
                        .next()
                        .unwrap()
                        .unwrap();
                    let expected = host
                        .dispatch(ChoiceOperation::Prepare {
                            capture: host.capture().into(),
                            shown: q["shown"].as_str().unwrap().into(),
                            request: serde_json::from_value(q["request"].clone()).unwrap(),
                            allow_defaults: true,
                        })
                        .unwrap();
                    for field in ["request", "graph", "effects", "record", "plan"] {
                        assert_eq!(actual[field], expected[field], "{field}");
                    }
                    assert!(!host.root().join("vibe.lock").exists());
                }
                _ => {
                    assert!(stdout.contains("defaults:"), "{stdout}");
                    assert!(stdout.contains("completed"), "{stdout}");
                    let lock =
                        vibe_core::manifest::Lockfile::read(host.root().join("vibe.lock")).unwrap();
                    let (requested, _) = vibe_wire::behaviour::lock_configuration::decode(
                        lock.meta.configuration.as_ref().unwrap(),
                    )
                    .unwrap();
                    assert!(requested.defaults.is_empty());
                    let request: Value =
                        serde_json::from_str(&requested.configuration.unwrap()).unwrap();
                    assert_eq!(
                        request["edges"][0]["selections"]["extras"]["members"],
                        serde_json::json!([])
                    );
                }
            }
        }
    }
}

fn query(args: &InstallArgs, operation: &ChoiceOperation) -> Result<Value> {
    #[cfg(unix)]
    {
        use std::{
            io::{Read, Write},
            os::unix::net::UnixStream,
        };
        let mut stream = UnixStream::connect(args.choice.choice_host.as_ref().unwrap())?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;
        serde_json::to_writer(&mut stream, operation)?;
        stream.flush()?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut bytes = Vec::new();
        stream.take(8 * 1024 * 1024).read_to_end(&mut bytes)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
    #[cfg(not(unix))]
    {
        let _ = (args, operation);
        bail!("prepared choice endpoint unavailable on this platform")
    }
}

pub(super) fn run(ctx: &output::Context, args: &InstallArgs, root_offline: bool) -> Result<()> {
    if args.global
        || !args.packages.is_empty()
        || args.git.is_some()
        || !args.features.is_empty()
        || args.all_features
        || args.no_default_features
    {
        bail!(
            "prepared choices use the host's retained workspace request; omit install-root/source/feature overrides"
        );
    }
    let user_config =
        vibe_core::user_config::UserConfig::load().context("loading the user config")?;
    let offline = output::resolve_offline(root_offline || args.offline, user_config.net.offline);
    let capture = args
        .choice
        .choice_capture
        .clone()
        .context("missing choice capture")?;
    let mut reply = query(
        args,
        &ChoiceOperation::Questions {
            capture: capture.clone(),
        },
    )?;
    let attended = !ctx.is_json()
        && !ctx.is_unattended()
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    if reply["status"] != "needs_selection" {
        return emit(ctx, &reply);
    }
    // The retained host/source cannot be reconfigured by this client. Refuse
    // before evaluation, preparation or application if it cannot promise offline.
    // An older host without the posture field is also unproved.
    if offline && reply["offline"].as_bool() != Some(true) {
        bail!("offline install requires an offline prepared choice host");
    }
    if reply["root"].as_str()
        != Some(
            std::fs::canonicalize(&args.path)?
                .to_str()
                .context("non-UTF8 choice root")?,
        )
    {
        bail!("prepared choice host root differs from --path");
    }
    let shown = reply["shown"]
        .as_str()
        .context("missing shown identity")?
        .to_owned();
    if let Some(expected) = &args.choice.shown_definition {
        if expected != &shown {
            bail!("shown definition changed; rediscovery required");
        }
    } else if !attended {
        return emit(ctx, &reply);
    }
    let mut request: Request = if let Some(path) = &args.choice.choice_request {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        serde_json::from_value(reply["request"].clone())?
    };
    // The complete shown defaults/effects precede any semantic question.
    if attended {
        ctx.summary(&serde_json::to_string_pretty(&reply)?);
    }
    let mut previous = None;
    while attended {
        reply = query(
            args,
            &ChoiceOperation::Evaluate {
                capture: capture.clone(),
                shown: shown.clone(),
                request: request.clone(),
                allow_defaults: args.choice.allow_defaults,
            },
        )?;
        if reply["status"] != "questions" {
            return emit(ctx, &reply);
        }
        if previous.as_ref() == Some(&reply) {
            return emit(ctx, &reply);
        }
        previous = Some(reply.clone());
        // B2 supplies active needs. Guards are never re-evaluated by the UI.
        let outcomes = reply["questions"]["outcomes"]
            .as_array()
            .context("missing typed questions")?;
        let mut answered = false;
        for outcome in outcomes {
            let representative = outcome["representative"]
                .as_u64()
                .context("missing representative")? as usize;
            if outcome["status"] == "conflict" {
                return emit(ctx, &reply);
            }
            for group in outcome["selected"]
                .as_object()
                .context("missing active groups")?
                .keys()
            {
                let group = group.as_str();
                if request.edges[representative]
                    .selections
                    .get(group)
                    .is_some_and(|s| s.members.is_some())
                {
                    continue;
                }
                if args.choice.allow_defaults
                    && !outcome["needs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|n| n == group)
                {
                    continue;
                }
                let declared = reply["controls"]
                    .as_array()
                    .context("missing typed controls")?
                    .iter()
                    .find(|q| q["edge"] == representative && q["group"] == group)
                    .context("missing active control")?;
                let members = declared["members"]
                    .as_array()
                    .context("missing members")?
                    .iter()
                    .map(|v| v.as_str().map(str::to_owned).context("invalid member"))
                    .collect::<Result<Vec<_>>>()?;
                if members.is_empty() {
                    continue;
                }
                let prompt = format!("{group} (defaults: {})", declared["defaults"]);
                let picked = if declared["cardinality"] == "exactly-one" {
                    let index = dialoguer::Select::new()
                        .with_prompt(&prompt)
                        .items(&members)
                        .interact()?;
                    vec![members[index].clone()]
                } else if declared["cardinality"] == "zero-or-one" {
                    let mut options = vec!["(none)".to_owned()];
                    options.extend(members.iter().cloned());
                    let index = dialoguer::Select::new()
                        .with_prompt(&prompt)
                        .items(&options)
                        .interact()?;
                    if index == 0 {
                        Vec::new()
                    } else {
                        vec![members[index - 1].clone()]
                    }
                } else {
                    loop {
                        let indices = dialoguer::MultiSelect::new()
                            .with_prompt(&prompt)
                            .items(&members)
                            .interact()?;
                        if indices.is_empty() && declared["cardinality"] == "one-or-more" {
                            ctx.summary("Choose at least one member.");
                            continue;
                        }
                        break indices.into_iter().map(|i| members[i].clone()).collect();
                    }
                };
                request.edges[representative].selections.insert(
                    group.into(),
                    Selection {
                        intent: SelectionIntent::Exact,
                        provenance: "cli:explicit-tty-answer".into(),
                        members: Some(picked),
                    },
                );
                answered = true;
                break;
            }
            if answered {
                break;
            }
        }
        if !answered {
            break;
        }
    }
    reply = query(
        args,
        &ChoiceOperation::Prepare {
            capture: capture.clone(),
            shown: shown.clone(),
            request,
            allow_defaults: args.choice.allow_defaults,
        },
    )?;
    if attended && reply["status"] == "prepared" {
        ctx.summary(&serde_json::to_string_pretty(&reply)?);
    }
    if reply["status"] == "prepared" && args.choice.apply_choices {
        // Assume-yes only confirms application after all semantic questions resolve.
        let confirmed = args.assume_yes
            || (attended
                && dialoguer::Confirm::new()
                    .with_prompt("Apply this prepared configuration?")
                    .default(false)
                    .interact()?);
        if confirmed {
            reply = query(
                args,
                &ChoiceOperation::Apply {
                    capture,
                    plan: reply["plan"].as_str().context("missing plan")?.into(),
                },
            )?;
        }
    }
    emit(ctx, &reply)
}
fn emit(ctx: &output::Context, value: &Value) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(value)?;
    } else {
        ctx.summary(&serde_json::to_string_pretty(value)?);
    }
    Ok(())
}
