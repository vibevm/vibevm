//! Public Linux fixture for agent hooks (PROP-064 `#acceptance`): a fake
//! agent executable that records its arguments, environment, prompt and
//! context and writes a result proves the three selection sources and
//! their order, consent refusal and `--assume-yes`, the external
//! registration argument contract, parking and `--resume`, the transcript
//! record, a failing agent stopping the installation with the work
//! directory kept, the skip announced without an agent, the required
//! refusal, and `vibe agent list | presets`.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::path::PathBuf;

use common::fake_agent::{
    self, Fixture, KEY, NAME, newest_record, ok, refused, registered, run, step, steps_json,
};
use serde_json::Value;

#[test]
fn without_an_agent_the_step_is_skipped_and_announced_before_anything_runs() {
    let fixture = Fixture::new("");
    let plan = ok(&run(fixture.app(&["app", "plan"], &[])));
    // A row per step, a skipped one dim under `○` with its reason as its
    // metadata (PROP-065 `#plan`, PROP-067 `#list`).
    assert!(
        plan.contains("○ 2.  fix ") && plan.contains("skipped: no agent configured"),
        "{plan}"
    );
    assert!(
        plan.contains("! no agent is configured, so agent step(s) fix will be skipped")
            && plan.contains("--agent NAME")
            && plan.contains("VIBE_AGENT_COMMAND"),
        "{plan}"
    );
    let output = run(fixture.app(&["install", "-g"], &[]));
    ok(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("! no agent is configured"), "{stderr}");
    assert_eq!(fixture.log(), "AFTER\n");
    assert_eq!(fixture.record.calls(), 0);
    let record = &fixture.index()["installations"][KEY];
    assert_eq!(record["steps"][1]["outcome"], "skipped");
}

#[test]
fn a_required_agent_step_without_an_agent_is_refused_with_its_reason() {
    let fixture = Fixture::new("required = true");
    let output = run(fixture.app(&["install", "-g"], &["--json"]));
    let (code, envelope) = refused(&output);
    assert_eq!(code, Some(2));
    assert_eq!(envelope["code"], "agent_missing");
    assert_eq!(envelope["step"], "fix");
    assert!(
        envelope["error"]
            .as_str()
            .unwrap()
            .contains("the payload needs a host-specific touch"),
        "{envelope}"
    );
    assert!(!fixture.work().exists());
}

#[test]
fn an_unconsented_prompt_is_refused_before_any_step_and_assume_yes_runs_it() {
    let fixture = Fixture::new("");
    let output = run(registered(fixture.app(&["install", "-g"], &[])));
    assert_eq!(output.status.code(), Some(2), "a refusal before any effect");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "agent step `fix` (Let an agent adapt the payload) will run agent `external`"
        ),
        "the prompt is shown: {stderr}"
    );
    assert!(stderr.contains("    Keep $HOME untouched."), "{stderr}");
    assert!(stderr.contains("needs consent"), "{stderr}");
    assert_eq!(fixture.record.calls(), 0);
    assert!(!fixture.work().exists(), "nothing ran");
    assert_eq!(fixture.log(), "");

    let output = run(registered(fixture.app(&["install", "-g"], &["--json"])));
    let (code, envelope) = refused(&output);
    assert_eq!(
        (code, envelope["code"].as_str()),
        (Some(2), Some("consent_required"))
    );

    // A trusted group needs no question.
    fs::write(
        fixture.user.settings.join("agents.toml"),
        "trusted_groups = [\"org.example\"]\n",
    )
    .unwrap();
    ok(&run(registered(fixture.app(&["install", "-g"], &[]))));
    assert_eq!(fixture.record.calls(), 1);
    fs::remove_file(fixture.user.settings.join("agents.toml")).unwrap();
    ok(&run(registered(
        fixture.app(&["install", "-g", "--unattended"], &[]),
    )));
    assert_eq!(
        fixture.record.calls(),
        2,
        "--unattended implies --assume-yes"
    );
}

#[test]
fn an_unattended_agent_receives_the_envelope_and_its_answer_is_recorded() {
    let fixture = Fixture::new("");
    let output = run(registered(fixture.app(
        &["install", "-g"],
        &["--assume-yes", "--json", "--progress-format", "json"],
    )));
    let report: Value = serde_json::from_str(&ok(&output)).unwrap();
    assert_eq!(report["installation"], KEY);
    assert_eq!(fixture.log(), "AFTER\n");

    // The external-registration argument contract.
    let work = fixture.work();
    let run_dir = work.join("steps/fix");
    let path = |file: &str| run_dir.join(file).display().to_string();
    assert_eq!(
        fixture.record.args(),
        [
            "--hook".to_owned(),
            "fix".into(),
            "--mode".into(),
            "unattended".into(),
            "--prompt-file".into(),
            path("prompt.md"),
            "--context-file".into(),
            path("context.json"),
            "--work".into(),
            work.display().to_string(),
        ]
    );
    assert_eq!(
        fixture.record.read("cwd").trim(),
        work.display().to_string()
    );
    assert_eq!(fixture.record.env("VIBE_HOOK_STEP"), "fix");
    assert_eq!(
        fixture.record.env("VIBE_HOOK_RUN"),
        run_dir.display().to_string()
    );
    assert_eq!(
        fixture.record.env("VIBE_OUT"),
        work.join("out").display().to_string()
    );
    assert_eq!(
        fixture.record.env("VIBE_APP_ROOT"),
        "",
        "unknown to a prepare step"
    );
    let out = work.join("out").display().to_string();
    assert_eq!(
        fixture.record.read("prompt.md"),
        format!("Adapt the payload in {out}.\nKeep $HOME untouched.\n")
    );
    let context: Value = serde_json::from_str(&fixture.record.read("context.json")).unwrap();
    assert_eq!(context["protocol"], "vibe-agent-hook/1");
    assert_eq!(context["package"]["name"], "hooked");
    assert_eq!(
        (context["step"].as_str(), context["mode"].as_str()),
        (Some("fix"), Some("unattended"))
    );
    assert_eq!(context["paths"]["result_file"], path("result.json"));
    assert_eq!(context["steps"][0]["id"], "payload");
    assert_eq!(context["steps"][0]["outcome"], "done");

    // The event stream names the agent run.
    let events: Vec<Value> = String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let detail = |prefix: &str| {
        events.iter().any(|event| {
            event["kind"] == "detail"
                && event["message"]
                    .as_str()
                    .is_some_and(|text| text.starts_with(prefix))
        })
    };
    assert!(detail(
        "agent_started: agent=external mode=unattended envelope="
    ));
    assert!(detail("agent_finished: agent=external status=ok"));
    assert!(
        detail("fake agent ran step fix"),
        "agent output is a detail line"
    );

    // The run record keeps the envelope, the transcript and the answer.
    let record = newest_record(&fixture);
    let kept = record.join("steps/fix");
    assert!(kept.join("prompt.md").is_file() && kept.join("context.json").is_file());
    assert!(
        fs::read_to_string(kept.join("transcript.log"))
            .unwrap()
            .contains("fake agent ran step fix")
    );
    assert!(
        fs::read_to_string(kept.join("result.json"))
            .unwrap()
            .contains("\"ok\"")
    );
    let state = steps_json(&record);
    let fix = step(&state, "fix");
    assert_eq!(fix["outcome"], "done");
    assert_eq!(fix["reason"], "fake agent answered ok");
    assert_eq!(fix["agent"]["agent"], "external");
    assert_eq!(fix["agent"]["mode"], "unattended");
    assert_eq!(fix["agent"]["status"], "ok");
    assert_eq!(fix["agent"]["envelope"], run_dir.display().to_string());
    assert!(fix["agent"]["durationMs"].is_u64());
    assert!(!work.exists(), "success deletes the work directory");
    let runs: Value =
        serde_json::from_str(&ok(&run(fixture.command(&["app", "runs", NAME, "--json"])))).unwrap();
    assert_eq!(runs["runs"][0]["outcome"], "done");
}

#[test]
fn a_failing_agent_stops_the_installation_and_keeps_the_work_directory() {
    let fixture = Fixture::new("");
    let mut command = registered(fixture.app(&["install", "-g"], &["--assume-yes", "--json"]));
    command.env("FAKE_AGENT_STATUS", "fail");
    let (code, envelope) = refused(&run(command));
    assert_eq!(code, Some(1));
    assert_eq!(envelope["code"], "step_failed");
    assert_eq!(envelope["step"], "fix");
    assert!(
        envelope["error"]
            .as_str()
            .unwrap()
            .contains("it answered fail: fake agent answered fail"),
        "{envelope}"
    );
    let work = fixture.work();
    assert_eq!(envelope["work"], work.display().to_string());
    let state = steps_json(&work);
    assert_eq!(step(&state, "fix")["outcome"], "failed");
    assert_eq!(step(&state, "fix")["agent"]["status"], "fail");
    assert_eq!(fixture.log(), "", "nothing after the failed step ran");
    assert!(
        !fixture
            .user
            .settings
            .join("applications/index.json")
            .exists()
    );

    // A non-zero exit fails the step too; vibe never retries on its own.
    let mut command = registered(fixture.app(&["install", "-g"], &["--assume-yes", "--resume"]));
    command
        .env("FAKE_AGENT_STATUS", "none")
        .env("FAKE_AGENT_EXIT", "3");
    let output = run(command);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exited with code 3"), "{stderr}");
    assert_eq!(fixture.record.calls(), 2);

    // --resume with a working agent finishes from `fix`.
    ok(&run(registered(
        fixture.app(&["install", "-g"], &["--assume-yes", "--resume"]),
    )));
    assert_eq!(fixture.record.calls(), 3);
    assert_eq!(fixture.log(), "AFTER\n");
}

#[test]
fn inside_a_hosting_agent_the_step_is_parked_and_resume_validates_the_answer() {
    let fixture = Fixture::new("");
    let output = run(registered(
        fixture.app(&["install", "-g", "--agent-mode", "agent"], &[]),
    ));
    assert_eq!(output.status.code(), Some(75), "the delegated status");
    assert_eq!(
        fixture.record.calls(),
        0,
        "never spawned inside a hosting agent"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("```vibe-agent-tasks").count(), 1, "{stdout}");
    assert!(
        stdout.contains("resume: vibe install -g --agent-mode agent org.example/hooked --registry"),
        "{stdout}"
    );
    assert!(stdout.contains(" --resume\n"), "{stdout}");
    let work = fixture.work();
    let state = steps_json(&work);
    assert_eq!(step(&state, "payload")["outcome"], "done");
    assert_eq!(step(&state, "fix")["outcome"], "delegated");
    assert_eq!(step(&state, "fix")["agent"]["status"], "parked");
    let outbox = fixture.user.settings.join("agentic/outbox");
    let run_dirs: Vec<PathBuf> = fs::read_dir(&outbox)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    let task = run_dirs[0].join("fix.md");
    let text = fs::read_to_string(&task).unwrap();
    assert!(text.contains("Keep $HOME untouched."), "{text}");
    assert!(
        text.contains(&work.join("steps/fix/result.json").display().to_string()),
        "{text}"
    );
    assert!(stdout.contains(&task.display().to_string()), "{stdout}");
    assert_eq!(fixture.log(), "", "nothing after the parked step ran");
    let record = newest_record(&fixture);
    let run_json: Value =
        serde_json::from_slice(&fs::read(record.join("run.json")).unwrap()).unwrap();
    assert_eq!(run_json["outcome"], "delegated");

    // Resuming without an answer parks again; --json carries the facts.
    let output = run(fixture.app(
        &["install", "-g", "--agent-mode", "agent"],
        &["--resume", "--json"],
    ));
    let (code, envelope) = refused(&output);
    assert_eq!(
        (code, envelope["code"].as_str()),
        (Some(75), Some("delegated"))
    );
    assert_eq!(envelope["tasks"].as_array().unwrap().len(), 1);
    assert!(envelope["resume"].as_str().unwrap().ends_with("--resume"));
    assert_eq!(
        envelope["run"],
        work.join("steps/fix").display().to_string()
    );

    // The hosting agent answers; --resume validates it and continues.
    fs::write(
        work.join("steps/fix/result.json"),
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done by the host"}"#,
    )
    .unwrap();
    ok(&run(fixture.app(
        &["install", "-g", "--agent-mode", "agent"],
        &["--resume"],
    )));
    assert_eq!(fixture.log(), "AFTER\n");
    assert_eq!(fixture.record.calls(), 0);
    let record = &fixture.index()["installations"][KEY];
    assert_eq!(record["steps"][1]["outcome"], "done");
    assert!(!work.exists());
}

#[test]
fn a_parked_finish_step_resumes_against_the_placed_payload() {
    let fixture = Fixture::new("phase = \"finish\"");
    let output = run(fixture.app(&["install", "-g", "--agent-mode", "agent"], &[]));
    assert_eq!(output.status.code(), Some(75));
    let payload = fixture
        .user
        .settings
        .join("opt/apps/hooked/org.example.hooked@1.0.0");
    assert!(payload.join("bin/tool").is_file(), "the payload is placed");
    assert_eq!(fixture.log(), "AFTER\n");
    let record = &fixture.index()["installations"][KEY];
    assert_eq!(record["steps"][1]["outcome"], "delegated");
    let work = fixture.work();
    fs::write(work.join("steps/fix/result.json"), r#"{"status":"ok"}"#).unwrap();
    ok(&run(fixture.app(
        &["install", "-g", "--agent-mode", "agent"],
        &["--resume"],
    )));
    let record = &fixture.index()["installations"][KEY];
    assert_eq!(record["steps"][1]["outcome"], "done");
    assert!(payload.join("bin/tool").is_file());
    assert_eq!(
        fixture.log(),
        "AFTER\n",
        "the prepare steps did not run again"
    );
    assert!(!work.exists());
}

#[test]
fn the_flag_beats_the_registration_which_beats_the_default() {
    let fixture = Fixture::new("");
    let script = fake_agent::script().display().to_string();
    fs::write(
        fixture.user.settings.join("agents.toml"),
        format!(
            "default = \"configured\"\n\n[agent.configured]\ncommand = \"sh\"\n\
             interactive = [\"{script}\", \"--interactive\", \"{{prompt_file}}\"]\n\
             unattended = [\"{script}\", \"--print\", \"{{prompt}}\"]\nextra_args = [\"--extra\"]\n\
             env = {{ FAKE_AGENT_NAME = \"configured\" }}\n"
        ),
    )
    .unwrap();
    let install = |flag: Option<&str>, registration: bool| {
        let mut extra = vec!["--assume-yes"];
        if let Some(name) = flag {
            extra.extend(["--agent", name]);
        }
        let command = fixture.app(&["install", "-g"], &extra);
        ok(&run(if registration {
            registered(command)
        } else {
            command
        }));
    };
    install(None, false);
    assert_eq!(fixture.record.env("FAKE_AGENT_NAME"), "configured");
    let out = fixture.work().join("out").display().to_string();
    assert_eq!(
        fixture.record.args(),
        [
            "--print".to_owned(),
            format!("Adapt the payload in {out}."),
            "Keep $HOME untouched.".into(),
            String::new(),
            "--extra".into(),
        ],
        "the prompt is one argument (recorded one line per line)"
    );
    install(None, true);
    assert_eq!(
        fixture.record.args()[0],
        "--hook",
        "the registration beats the default"
    );
    assert_eq!(fixture.record.env("FAKE_AGENT_NAME"), "");
    install(Some("configured"), true);
    assert_eq!(
        fixture.record.args()[0],
        "--print",
        "the flag beats the registration"
    );

    let output = run(fixture.app(&["install", "-g"], &["--agent", "ghost", "--json"]));
    let (code, envelope) = refused(&output);
    assert_eq!(
        (code, envelope["code"].as_str()),
        (Some(2), Some("agent_unknown"))
    );
    let mut bad = fixture.app(&["install", "-g"], &["--json"]);
    bad.env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "not json");
    let (code, envelope) = refused(&run(bad));
    assert_eq!(
        (code, envelope["code"].as_str()),
        (Some(2), Some("agent_registration_invalid"))
    );
}

#[test]
fn an_interactive_step_without_a_terminal_is_refused_at_planning() {
    let fixture = Fixture::new("mode = \"interactive\"");
    let output = run(registered(
        fixture.app(&["install", "-g"], &["--assume-yes", "--json"]),
    ));
    let (code, envelope) = refused(&output);
    assert_eq!(
        (code, envelope["code"].as_str()),
        (Some(2), Some("need_unmet"))
    );
    assert!(
        envelope["error"].as_str().unwrap().contains("`terminal`"),
        "{envelope}"
    );
    assert_eq!(fixture.record.calls(), 0);
}

#[test]
fn agents_are_listed_configured_and_presets_shown() {
    let fixture = Fixture::new("");
    let presets: Value =
        serde_json::from_str(&ok(&run(fixture.command(&["agent", "presets", "--json"])))).unwrap();
    assert_eq!(presets["protocol"], "vibe-agent-presets/1");
    let names: Vec<&str> = presets["presets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|preset| preset["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "claude-code",
            "codex",
            "opencode",
            "qwen-code",
            "gemini",
            "cursor"
        ]
    );
    assert_eq!(
        presets["presets"][0]["unattended"],
        serde_json::json!(["-p", "{prompt}"])
    );

    let empty: Value =
        serde_json::from_str(&ok(&run(fixture.command(&["agent", "list", "--json"])))).unwrap();
    assert_eq!(empty["protocol"], "vibe-agent-list/1");
    assert!(empty["agents"].as_array().unwrap().is_empty());
    assert!(empty.get("default").is_none());

    ok(&run(fixture.command(&["agent", "use", "codex"])));
    ok(&run(fixture.command(&[
        "agent",
        "add",
        "wrapped",
        "--preset",
        "claude-code",
        "--command",
        "/opt/bin/claude-codex",
    ])));
    let mut listing = fixture.command(&["agent", "list", "--json"]);
    listing
        .env("VIBE_AGENT_COMMAND", "gui-tool")
        .env("VIBE_AGENT_ARGS", "[\"--x\"]");
    let listed: Value = serde_json::from_str(&ok(&run(listing))).unwrap();
    assert_eq!(listed["default"], "codex");
    let wrapped = listed["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|agent| agent["name"] == "wrapped")
        .unwrap();
    assert_eq!(wrapped["command"], "/opt/bin/claude-codex");
    assert_eq!(wrapped["preset"], "claude-code");
    assert_eq!(wrapped["default"], false);
    assert_eq!(listed["external"]["command"], "gui-tool");
    let text = fs::read_to_string(fixture.user.settings.join("agents.toml")).unwrap();
    assert!(text.starts_with("default = \"codex\""), "{text}");

    let (code, envelope) =
        refused(&run(fixture.command(&[
            "agent", "add", "wrapped", "--preset", "codex", "--json",
        ])));
    assert_eq!(
        (code, envelope["code"].as_str()),
        (Some(2), Some("agent_exists"))
    );
    ok(&run(fixture.command(&["agent", "remove", "codex"])));
    let listed: Value =
        serde_json::from_str(&ok(&run(fixture.command(&["agent", "list", "--json"])))).unwrap();
    assert!(
        listed.get("default").is_none(),
        "removing the default clears it"
    );
    let errors: Value =
        serde_json::from_str(&ok(&run(fixture.command(&["app", "errors", "--json"])))).unwrap();
    let exit_of = |code: &str| {
        errors["codes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["code"] == code)
            .map(|row| row["exit"].as_u64().unwrap())
    };
    assert_eq!(exit_of("delegated"), Some(75));
    assert_eq!(exit_of("consent_required"), Some(2));
    assert_eq!(exit_of("agent_failed"), Some(1));
}
