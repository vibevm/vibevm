//! The sandbox a documented command runs in
//! (PROP-057 `##PIPE-EXAMPLE-RUNNER`).
//!
//! Three rules, all of them load-bearing:
//!
//! 1. **The working directory is a sandbox, never the source tree.** The
//!    commands a page shows name no paths, so nothing absolute reaches the
//!    output that normalisation then has to chase; and a run from inside
//!    the checkout was observed to leave a file behind in it.
//! 2. **The isolation environment does not change behaviour.** The runner
//!    sets the settings directory and `NO_COLOR` and REMOVES every
//!    behavioural variable it inherited (`VIBE_OFFLINE`, `VIBE_UNATTENDED`,
//!    `VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`,
//!    `VIBEFRAME`). `--offline` and `--assume-yes` stand in the example
//!    itself or they stand nowhere: otherwise a page turns green for a
//!    reason its reader cannot see in the command line.
//! 3. **The registry is local and lives inside the fixture.** A root
//!    fixture carries a copy of the packages its examples install, and its
//!    machine-level `registry.toml` addresses that copy as `${REGISTRY}`.
//!    Nothing reaches the network, and nothing reaches the host tree
//!    (X-025).
//!
//! A fixture is built once per run and copied for each example, so two
//! examples on one fixture cannot see each other's effects. Every copy is
//! RETARGETED — the parent's absolute path is rewritten to the copy's —
//! because a lock file and a settings directory record where they were
//! written, and a copy that still points at its parent would resolve
//! through a directory the page knows nothing about.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER");

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::{DocError, Result};
use crate::examples::command::{CommandLine, Program, parse};
use crate::examples::fixture::{
    FixtureFile, HOME_DIR, OVERLAY_DIR, REGISTRY_DIR, ancestry, fixture_dir,
};
use crate::examples::normalize::{display_path, replace_path_spellings};

/// The behavioural variables the runner strips from the child's
/// environment, so an example's flags are the only thing that steers it.
pub const CLEARED: &[&str] = &[
    "VIBE_OFFLINE",
    "VIBE_UNATTENDED",
    "VIBE_INVOKED_BY",
    "VIBE_NO_DEFAULT_REGISTRY",
    "VIBETERM",
    "VIBEFRAME",
    "VIBE_REGISTRY_CACHE",
    "VIBEVM_SEARCH_CACHE_DIR",
    "VIBEVM_INSTALL_ROOT",
];

/// The sandbox subdirectory each fixture's built state lives in.
const FIXTURES: &str = "f";

/// What a child process left behind.
#[derive(Debug, Clone)]
pub struct Capture {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// What the runner needs from its caller. The library reads no ambient
/// environment of its own: the binary, the roots and the guarded paths all
/// arrive as data (the composition root above resolves them).
#[derive(Debug, Clone)]
pub struct RunnerEnv {
    /// The built `vibe` binary every example and recipe step runs.
    pub binary: PathBuf,
    /// Where sandboxes are created. Keep it SHORT: a deep sandbox path
    /// overflows the Windows path limit inside a materialised dependency
    /// tree.
    pub sandbox_root: PathBuf,
    /// The source tree, replaced by `<REPO>` in output and guarded by the
    /// tripwire.
    pub repo_root: Option<PathBuf>,
    /// The operator's own home directory, replaced by `<HOME>` in
    /// output: a product that prints `~/.vibe` prints the home it hangs
    /// off, and that is a person's account name.
    pub user_home: Option<PathBuf>,
    /// The real per-user settings directory (`~/.vibe`) the tripwire
    /// guards. A different thing from the placeholder above, and the two
    /// are never the same path: one is what must not leak into a page,
    /// the other is what must not move.
    pub settings_home: Option<PathBuf>,
    /// `cargo`, for the recipes that need a real build tree.
    pub cargo: PathBuf,
    /// How long one command may take before the runner kills it. A hung
    /// child must fail the panel, never own it.
    pub timeout_secs: u64,
}

/// One materialised fixture.
#[derive(Debug, Clone)]
pub struct Materialised {
    /// The sandbox root: `home/`, `work/` and the registry copy.
    pub root: PathBuf,
    /// Absolute working directory for this fixture's examples.
    pub cwd: PathBuf,
    /// `true` for the one fixture whose subject is the checkout itself.
    pub in_source_tree: bool,
}

/// Build every fixture in `name`'s ancestry, reusing what an earlier
/// example of this run already built.
pub fn build_fixture(
    package_dir: &Path,
    env: &RunnerEnv,
    name: &str,
    built: &mut BTreeMap<String, Materialised>,
) -> Result<Materialised> {
    if let Some(done) = built.get(name) {
        return Ok(done.clone());
    }
    let chain = ancestry(package_dir, name)?;
    let mut last: Option<Materialised> = None;
    for (step_name, decl) in chain {
        if let Some(done) = built.get(&step_name) {
            last = Some(done.clone());
            continue;
        }
        let made = materialise(package_dir, env, &step_name, &decl, last.as_ref())?;
        built.insert(step_name, made.clone());
        last = Some(made);
    }
    last.ok_or_else(|| DocError::Fixture {
        fixture: name.to_owned(),
        message: "the recipe chain is empty".into(),
    })
}

/// Build one fixture from its parent's finished state.
fn materialise(
    package_dir: &Path,
    env: &RunnerEnv,
    name: &str,
    decl: &FixtureFile,
    parent: Option<&Materialised>,
) -> Result<Materialised> {
    let root = env.sandbox_root.join(FIXTURES).join(name);
    remove_dir(&root)?;
    match parent {
        Some(p) => copy_state(&p.root, &root)?,
        None => create_dir(&root)?,
    }
    create_dir(&root.join(HOME_DIR))?;
    create_dir(&root.join("work"))?;

    let overlay = fixture_dir(package_dir, name).join(OVERLAY_DIR);
    if overlay.is_dir() {
        copy_tree(&overlay, &root)?;
        substitute_tree(&root)?;
    }

    let cwd = if decl.fixture.in_source_tree {
        env.repo_root.clone().ok_or_else(|| DocError::Fixture {
            fixture: name.to_owned(),
            message: "declares `in_source_tree`, but the caller named no source tree".into(),
        })?
    } else {
        root.join(decl.fixture.cwd.as_deref().unwrap_or("work"))
    };

    for (index, step) in decl.fixture.step.iter().enumerate() {
        let at = index + 1;
        let fail = |message: String| DocError::Sandbox {
            fixture: name.to_owned(),
            message: format!("recipe step {at}: {message}"),
        };
        match (&step.run, &step.copy, &step.append, &step.edit) {
            (Some(line), None, None, None) => {
                let step_cwd = match &step.cwd {
                    Some(rel) => root.join(rel),
                    None => cwd.clone(),
                };
                create_dir(&step_cwd)?;
                let line = substitute(line, &root);
                let cmd = parse(&line)?;
                let capture = run(env, &cmd, &step_cwd, &root)?;
                if capture.code != 0 {
                    return Err(fail(format!(
                        "`{line}` exited {}: {}",
                        capture.code,
                        first_line(&capture.stderr, &capture.stdout)
                    )));
                }
            }
            (None, Some(copy), None, None) => {
                let from = root.join(&copy.from);
                let to = root.join(&copy.to);
                copy_tree(&from, &to)?;
                retarget_tree(&to, &display_path(&from), &display_path(&to))?;
            }
            (None, None, Some(append), None) => {
                let path = root.join(&append.path);
                let mut text = std::fs::read_to_string(&path)
                    .map_err(|e| fail(format!("cannot read `{}`: {e}", path.display())))?;
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&append.text);
                std::fs::write(&path, text).map_err(|e| DocError::io("writing", &path, e))?;
            }
            (None, None, None, Some(edit)) => {
                let path = root.join(&edit.path);
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| fail(format!("cannot read `{}`: {e}", path.display())))?;
                if !text.contains(&edit.from) {
                    return Err(fail(format!(
                        "`{}` holds no `{}` to replace",
                        path.display(),
                        edit.from
                    )));
                }
                let replaced = text.replace(&edit.from, &edit.to);
                std::fs::write(&path, replaced).map_err(|e| DocError::io("writing", &path, e))?;
            }
            _ => {
                return Err(fail(
                    "a step declares exactly one of `run`, `copy`, `append`, `edit`".into(),
                ));
            }
        }
    }
    Ok(Materialised {
        root,
        cwd,
        in_source_tree: decl.fixture.in_source_tree,
    })
}

/// Copy a fixture's finished state into a fresh directory for one
/// example, so two examples on one fixture cannot see each other's work.
pub fn clone_for_example(base: &Materialised, into: PathBuf) -> Result<Materialised> {
    if base.in_source_tree {
        // The source-tree fixture has no state of its own to copy: its
        // subject IS the checkout, which is read-only here. Its settings
        // home is still the sandbox one.
        return Ok(base.clone());
    }
    remove_dir(&into)?;
    copy_state(&base.root, &into)?;
    let rel = base
        .cwd
        .strip_prefix(&base.root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| PathBuf::from("work"));
    Ok(Materialised {
        cwd: into.join(rel),
        root: into,
        in_source_tree: false,
    })
}

/// Copy a sandbox and rewrite every reference to its old location.
fn copy_state(from: &Path, to: &Path) -> Result<()> {
    copy_tree(from, to)?;
    retarget_tree(to, &display_path(from), &display_path(to))
}

/// Run one command line, captured, isolated and bounded.
pub fn run(env: &RunnerEnv, cmd: &CommandLine, cwd: &Path, sandbox: &Path) -> Result<Capture> {
    let program = match cmd.program {
        Program::Vibe => env.binary.clone(),
        Program::Cargo => env.cargo.clone(),
        Program::Cat => return cat(cmd, cwd),
    };
    let mut child = Command::new(&program);
    child
        .args(&cmd.args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for key in CLEARED {
        child.env_remove(key);
    }
    child
        .env("VIBE_SETTINGS", display_path(&sandbox.join(HOME_DIR)))
        .env("VIBE_NO_PROGRESS", "1")
        .env("NO_COLOR", "1");
    let fail = |message: String| DocError::Command {
        command: cmd.source.clone(),
        message,
    };
    let mut handle = child
        .spawn()
        .map_err(|e| fail(format!("cannot run `{}`: {e}", program.display())))?;
    let stdout = handle.stdout.take();
    let stderr = handle.stderr.take();
    let out_thread = std::thread::spawn(move || drain(stdout));
    let err_thread = std::thread::spawn(move || drain(stderr));

    let deadline = Instant::now() + Duration::from_secs(env.timeout_secs.max(1));
    let status = loop {
        match handle.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = handle.kill();
                    let _ = handle.wait();
                    return Err(fail(format!(
                        "still running after {}s — killed",
                        env.timeout_secs
                    )));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(fail(format!("waiting for the child: {e}"))),
        }
    };
    let out = out_thread.join().unwrap_or_default();
    let err = err_thread.join().unwrap_or_default();
    Ok(Capture {
        code: status.code().unwrap_or(-1),
        stdout: out,
        stderr: err,
    })
}

/// Read one captured pipe to the end. Runs on its own thread so a child
/// that fills both pipes cannot deadlock against the wait loop.
fn drain<R: Read>(pipe: Option<R>) -> String {
    let Some(mut pipe) = pipe else {
        return String::new();
    };
    let mut bytes = Vec::new();
    let _ = pipe.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// `cat <path>` — the file's bytes, as the page means them.
fn cat(cmd: &CommandLine, cwd: &Path) -> Result<Capture> {
    let Some(rel) = cmd.args.first() else {
        return Err(DocError::Command {
            command: cmd.source.clone(),
            message: "`cat` needs exactly one path".into(),
        });
    };
    match std::fs::read(cwd.join(rel)) {
        Ok(bytes) => Ok(Capture {
            code: 0,
            stdout: String::from_utf8_lossy(&bytes).into_owned(),
            stderr: String::new(),
        }),
        Err(e) => Ok(Capture {
            code: 1,
            stdout: String::new(),
            stderr: format!("cat: {rel}: {e}\n"),
        }),
    }
}

fn first_line(stderr: &str, stdout: &str) -> String {
    let source = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    source.lines().next().unwrap_or("(no output)").to_owned()
}

/// `${REGISTRY}` and `${SANDBOX}` in one string.
pub fn substitute(text: &str, sandbox: &Path) -> String {
    let registry = display_path(&sandbox.join(REGISTRY_DIR)).replace('\\', "/");
    let root = display_path(sandbox).replace('\\', "/");
    text.replace("${REGISTRY}", &registry)
        .replace("${SANDBOX}", &root)
}

/// Substitute the placeholders in every text file of a freshly laid
/// overlay. A fixture's own files are the only place they appear: the
/// command a page shows never carries one.
fn substitute_tree(root: &Path) -> Result<()> {
    rewrite_text_files(root, &|text| {
        if text.contains("${") {
            Some(substitute(text, root))
        } else {
            None
        }
    })
}

/// Rewrite every absolute reference to `old` into `new` — in the native,
/// forward-slash and JSON-escaped spellings alike.
fn retarget_tree(root: &Path, old: &str, new: &str) -> Result<()> {
    rewrite_text_files(root, &|text| {
        let replaced = replace_path_spellings(text, old, new);
        (replaced != text).then_some(replaced)
    })
}

fn rewrite_text_files(root: &Path, edit: &dyn Fn(&str) -> Option<String>) -> Result<()> {
    for entry in walkdir::WalkDir::new(root).sort_by_file_name() {
        let entry = entry.map_err(|e| walk_error(root, e))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if let Some(replaced) = edit(&text) {
            std::fs::write(path, replaced).map_err(|e| DocError::io("writing", path, e))?;
        }
    }
    Ok(())
}

fn walk_error(root: &Path, e: walkdir::Error) -> DocError {
    DocError::io(
        "walking",
        root,
        e.into_io_error()
            .unwrap_or_else(|| std::io::Error::other("walk failed")),
    )
}

fn create_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|e| DocError::io("creating", path, e))
}

fn remove_dir(path: &Path) -> Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(DocError::io("removing", path, e)),
    }
}

/// Copy a directory tree, creating the destination.
pub fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    create_dir(to)?;
    for entry in walkdir::WalkDir::new(from).sort_by_file_name() {
        let entry = entry.map_err(|e| walk_error(from, e))?;
        let Ok(rel) = entry.path().strip_prefix(from) else {
            continue;
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = to.join(rel);
        if entry.file_type().is_dir() {
            create_dir(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                create_dir(parent)?;
            }
            std::fs::copy(entry.path(), &target)
                .map_err(|e| DocError::io("copying", entry.path(), e))?;
        }
    }
    Ok(())
}
