//! The prompt check, end to end, against a fake agent.
//!
//! The fake agent is this test binary, re-invoked on the one test below
//! that acts as a runner. It is not a mock: a real process is started in
//! the sandbox, the prompt reaches it on its standard input, it changes
//! the working directory the way an agent would, and the page's own
//! asserts then judge what it left. What is faked is the reasoning, which
//! is the one part this check does not own.
//!
//! A script would have been simpler on one platform and absent on the
//! other — a `.sh` needs a shell Windows has not got, and a `.cmd` needs
//! a Windows the panel does not always run on.

use super::*;

use std::fs;

use crate::examples::sandbox::RunnerEnv;

/// The fake agent. When this process was started BY the prompt check —
/// which is what the environment says — it reads the prompt, does what it
/// asks, and exits. Otherwise it is an ordinary test that asserts the two
/// halves of that contract exist.
#[test]
fn the_fake_runner_stands_in_for_an_agent() {
    let (Ok(dir), Ok(prompt_file)) = (std::env::var(ENV_DIR), std::env::var(ENV_PROMPT_FILE))
    else {
        // Not acting as a runner: assert the contract this test relies
        // on, so the names cannot drift away from it silently.
        assert_eq!(ENV_DIR, "VIBE_DOC_DIR");
        assert_eq!(ENV_PROMPT_FILE, "VIBE_DOC_PROMPT_FILE");
        return;
    };
    let mut asked = String::new();
    std::io::stdin().read_to_string(&mut asked).expect("stdin");
    // The prompt is on the standard input AND on disk; both must say the
    // same thing, which is the contract a scripted runner relies on.
    let on_disk = fs::read_to_string(&prompt_file).expect("the prompt file");
    assert_eq!(asked.trim(), on_disk.trim());

    if asked.contains("refuse") {
        panic!("the fake agent refuses this task");
    }
    // «Create a file named `made.txt`» — the file is between the first
    // pair of backticks.
    if let Some(rest) = asked.split_once('`').map(|(_, rest)| rest)
        && let Some((name, _)) = rest.split_once('`')
    {
        fs::write(PathBuf::from(&dir).join(name), "made by the fake agent\n")
            .expect("write the file the prompt asked for");
    }
}

/// The runner command that re-invokes this binary on the test above.
fn fake_runner() -> String {
    let exe = std::env::current_exe().expect("the test binary");
    format!(
        "\"{}\" prompts::tests::the_fake_runner_stands_in_for_an_agent --exact --nocapture",
        exe.display()
    )
}

struct Package {
    dir: tempfile::TempDir,
    sandbox: tempfile::TempDir,
}

impl Package {
    fn new(prompts_toml: &str) -> Package {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("vibe.toml"),
            "[package]\nname = \"m\"\ngroup = \"org.demo\"\nkind = \"doc\"\nversion = \"0.1.0\"\n",
        )
        .expect("manifest");
        fs::write(dir.path().join("prompts.toml"), prompts_toml).expect("prompts.toml");
        let fixture = dir.path().join("examples/empty");
        fs::create_dir_all(&fixture).expect("fixture dir");
        fs::write(fixture.join("example.toml"), "schema = 1\n").expect("fixture");
        Package {
            dir,
            sandbox: tempfile::tempdir().expect("sandbox dir"),
        }
    }

    fn page(&self, rel: &str, body: &str) -> &Package {
        let path = self.dir.path().join("vibevm/vibespecs").join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("page dir");
        }
        fs::write(
            &path,
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
                   <title id=\"root\">A task</title>\n{body}</spec>\n"
            ),
        )
        .expect("page");
        self
    }

    fn env(&self) -> RunnerEnv {
        RunnerEnv {
            binary: PathBuf::from("vibe"),
            sandbox_root: self.sandbox.path().to_path_buf(),
            repo_root: None,
            user_home: None,
            settings_home: None,
            cargo: PathBuf::from("cargo"),
            timeout_secs: 120,
        }
    }

    fn check(&self, opts: &Options) -> Report {
        super::check(self.dir.path(), &self.env(), opts).expect("the check runs")
    }
}

const CONFIG: &str = "schema = 1\n\n[doc.prompts]\ntimeout = 120\nfixture = \"empty\"\n";

fn with_runner() -> Options {
    Options {
        runner: Some(fake_runner()),
        ..Options::default()
    }
}

/// The whole contract in one test: a clean sandbox, the prompt handed to
/// the agent, and then the page's own asserts on what it left.
#[test]
fn the_agent_does_the_task_and_the_page_s_asserts_judge_the_result() {
    let package = Package::new(CONFIG);
    package.page(
        "howto/make-a-file.xml",
        "  <p>This page makes a file.</p>\n  \
         <prompt id=\"make\">Create a file named `made.txt` in this folder.\n    \
           <needs>nothing at all</needs>\n    \
           <outcome>a file called `made.txt` is there</outcome>\n    \
           <assert>test -f made.txt</assert>\n    \
           <assert>grep -q \"fake agent\" made.txt</assert>\n  \
         </prompt>\n",
    );
    let report = package.check(&with_runner());
    assert!(report.ok(), "{}", report.render());
    let outcome = &report.outcomes[0];
    assert_eq!(outcome.verdict, Verdict::Held);
    assert_eq!(outcome.runner_code, Some(0));
    assert_eq!(outcome.asserts.len(), 2);
    assert!(outcome.asserts.iter().all(|a| a.passed()));
}

/// An agent that finishes without doing the work is the case this check
/// exists for, and the page's assert is what catches it.
#[test]
fn an_agent_that_does_nothing_breaks_the_assert() {
    let package = Package::new(CONFIG);
    package.page(
        "howto/make-a-file.xml",
        "  <p>This page makes a file.</p>\n  \
         <prompt id=\"make\">Do nothing whatsoever in this folder.\n    \
           <assert>test -f made.txt</assert>\n  \
         </prompt>\n",
    );
    let report = package.check(&with_runner());
    assert!(!report.ok());
    assert_eq!(report.outcomes[0].verdict, Verdict::Broke);
    assert_eq!(report.outcomes[0].asserts[0].code, 1);
}

/// A runner that will not finish is a different report from a task that
/// was not done, and the two must not be confused.
#[test]
fn a_runner_that_fails_is_failed_and_not_broken() {
    let package = Package::new(CONFIG);
    package.page(
        "howto/make-a-file.xml",
        "  <p>This page makes a file.</p>\n  \
         <prompt id=\"make\">Please refuse this task outright.\n    \
           <assert>test -f made.txt</assert>\n  \
         </prompt>\n",
    );
    let report = package.check(&with_runner());
    assert!(matches!(report.outcomes[0].verdict, Verdict::Failed { .. }));
    assert!(report.outcomes[0].asserts.is_empty());
}

/// Nothing was claimed, so there is nothing to check.
#[test]
fn an_illustrative_prompt_is_skipped_with_its_reason() {
    let package = Package::new(CONFIG);
    package.page(
        "model/idea.xml",
        "  <p>This page explains an idea.</p>\n  \
         <s title=\"Asking\">\n    \
           <prompt id=\"ask\" assert=\"none\">Ask for the thing.</prompt>\n  \
         </s>\n",
    );
    let report = package.check(&with_runner());
    assert!(report.ok());
    assert!(matches!(
        &report.outcomes[0].verdict,
        Verdict::Skipped { reason } if reason.contains("illustrative")
    ));
}

/// The sandbox is fresh per prompt: the second prompt must not see what
/// the first one made.
#[test]
fn two_prompts_on_one_fixture_cannot_see_each_other_s_work() {
    let package = Package::new(CONFIG);
    package.page(
        "howto/first.xml",
        "  <p>This page makes a file.</p>\n  \
         <prompt id=\"make\">Create a file named `made.txt` in this folder.\n    \
           <assert>test -f made.txt</assert>\n  \
         </prompt>\n",
    );
    package.page(
        "howto/second.xml",
        "  <p>This page checks a folder.</p>\n  \
         <prompt id=\"look\">Do nothing whatsoever in this folder.\n    \
           <assert>test ! -e made.txt</assert>\n  \
         </prompt>\n",
    );
    let report = package.check(&with_runner());
    assert!(report.ok(), "{}", report.render());
}

/// A declared skip says what is missing; a silent one says nothing.
#[test]
fn a_declared_skip_is_honoured_and_named() {
    let package = Package::new(
        "schema = 1\n\n[doc.prompts]\nfixture = \"empty\"\n\n\
         [[doc.prompts.prompt]]\npage = \"howto/make-a-file.xml\"\nid = \"make\"\n\
         skip = \"publishes to a real registry\"\n",
    );
    package.page(
        "howto/make-a-file.xml",
        "  <p>This page makes a file.</p>\n  \
         <prompt id=\"make\">Create a file named `made.txt` in this folder.\n    \
           <assert>test -f made.txt</assert>\n  \
         </prompt>\n",
    );
    let report = package.check(&with_runner());
    assert!(report.ok());
    assert!(matches!(
        &report.outcomes[0].verdict,
        Verdict::Skipped { reason } if reason.contains("real registry")
    ));
}

/// The sample is random enough to exercise pages nobody would choose and
/// fixed enough that two runs a month apart are comparable.
#[test]
fn a_sample_is_the_same_prompts_every_time_and_not_the_first_n() {
    let corpus: Vec<Collected> = ["alpha", "beta", "gamma", "delta", "epsilon"]
        .iter()
        .map(|id| Collected {
            page: "p.xml".to_owned(),
            id: (*id).to_owned(),
            text: "x".to_owned(),
            needs: None,
            asserts: vec!["vibe check".to_owned()],
        })
        .collect();
    let config = Config::default();
    let opts = Options {
        sample: Some(2),
        ..Options::default()
    };
    let once = choose(&corpus, &config, &opts);
    assert_eq!(once.len(), 2);
    assert_eq!(once, choose(&corpus, &config, &opts));

    // Not positional: the same corpus in the opposite order yields the
    // same two prompts, which «the first N» never would. The seed is the
    // address, so a page moved up the tree changes nothing.
    let reversed: Vec<Collected> = corpus.iter().rev().cloned().collect();
    let mut a = once.clone();
    let mut b = choose(&reversed, &config, &opts);
    a.sort();
    b.sort();
    assert_eq!(a, b);

    // Zero is every prompt, and a sample larger than the corpus is too.
    let all = Options {
        sample: Some(0),
        ..Options::default()
    };
    assert_eq!(choose(&corpus, &config, &all).len(), 5);
}

/// A prompt outside the sample is reported as such, never as passed.
#[test]
fn a_prompt_outside_the_sample_is_skipped_and_says_so() {
    let package = Package::new(CONFIG);
    package.page(
        "howto/a.xml",
        "  <p>One.</p>\n  <prompt id=\"x\">Do nothing whatsoever.\n    \
         <assert>vibe check</assert>\n  </prompt>\n",
    );
    package.page(
        "howto/b.xml",
        "  <p>Two.</p>\n  <prompt id=\"y\">Do nothing whatsoever.\n    \
         <assert>vibe check</assert>\n  </prompt>\n",
    );
    let report = package.check(&Options {
        sample: Some(1),
        ..with_runner()
    });
    let skipped = report
        .outcomes
        .iter()
        .filter(|o| matches!(&o.verdict, Verdict::Skipped { reason } if reason.contains("sample")))
        .count();
    assert_eq!(skipped, 1);
}

/// Without an agent there is nothing to run, and a check that cannot run
/// must not print green.
#[test]
fn a_run_with_no_runner_anywhere_is_refused_by_name() {
    let package = Package::new("schema = 1\n\n[doc.prompts]\nfixture = \"empty\"\n");
    package.page(
        "howto/a.xml",
        "  <p>One.</p>\n  <prompt id=\"x\">Do it.\n    <assert>vibe check</assert>\n  </prompt>\n",
    );
    let e =
        super::check(package.dir.path(), &package.env(), &Options::default()).expect_err("refused");
    assert!(e.to_string().contains("--runner"), "{e}");
    assert!(e.to_string().contains("STYLE-PROMPT-FIRST"), "{e}");
}
