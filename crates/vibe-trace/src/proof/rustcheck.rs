use super::{
    files::{self, Capture},
    wire::*,
};
use command_group::CommandGroup;
use lean_ai_native_check::FileBinding;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(super) fn capture(
    root: &Path,
    policy: &RustPolicy,
    inputs: &mut Capture,
) -> Result<Vec<FileBinding>> {
    let manifest_path = Path::new(&policy.manifest);
    files::relative(manifest_path)?;
    if manifest_path.file_name().and_then(|p| p.to_str()) != Some("Cargo.toml") {
        return Err(Diagnostic::new(
            "rust_policy_invalid",
            "manifest must name Cargo.toml",
        ));
    }
    let crate_dir = manifest_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| {
            Diagnostic::new(
                "rust_policy_invalid",
                "selected crate must have its own subdirectory",
            )
        })?;
    let bytes = inputs.add(root, &policy.manifest)?;
    let manifest: toml::Value =
        toml::from_str(std::str::from_utf8(&bytes).map_err(files::input)?).map_err(files::input)?;
    let table = manifest
        .as_table()
        .ok_or_else(|| Diagnostic::new("rust_policy_invalid", "manifest is not a table"))?;
    if table
        .keys()
        .any(|k| !matches!(k.as_str(), "package" | "workspace"))
    {
        return Err(Diagnostic::new(
            "rust_dependency_unsupported",
            "first selected checker requires a default library without dependencies, features, build scripts or custom targets",
        ));
    }
    let package = table
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| Diagnostic::new("rust_policy_invalid", "package absent"))?;
    let allowed = [
        "name",
        "version",
        "edition",
        "rust-version",
        "authors",
        "license",
        "publish",
        "description",
    ];
    if package.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(Diagnostic::new(
            "rust_dependency_unsupported",
            "unsupported package field in selected dependency-free checker",
        ));
    }
    if let Some(workspace) = table.get("workspace") {
        if !workspace
            .as_table()
            .is_some_and(|t| t.keys().all(|k| k == "resolver"))
        {
            return Err(Diagnostic::new(
                "rust_dependency_unsupported",
                "external workspace members/dependencies are unsupported",
            ));
        }
    }
    if root.join(crate_dir).join("build.rs").exists() {
        return Err(Diagnostic::new(
            "rust_dependency_unsupported",
            "implicit build.rs is unsupported",
        ));
    }
    let lock = format!("{}/Cargo.lock", crate_dir.display());
    let lock_bytes = inputs.add(root, &lock)?;
    let lock_value: toml::Value =
        toml::from_str(std::str::from_utf8(&lock_bytes).map_err(files::input)?)
            .map_err(files::input)?;
    let packages = lock_value
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| Diagnostic::new("rust_dependency_unsupported", "lock has no package"))?;
    if packages.len() != 1
        || packages[0].get("name") != package.get("name")
        || packages[0].get("version") != package.get("version")
        || packages[0].get("source").is_some()
        || packages[0].get("dependencies").is_some()
    {
        return Err(Diagnostic::new(
            "rust_dependency_unsupported",
            "lock must bind exactly this dependency-free package",
        ));
    }
    let declared: BTreeSet<_> = policy.sources.iter().cloned().collect();
    if declared.len() != policy.sources.len()
        || !declared.contains(&format!("{}/src/lib.rs", crate_dir.display()))
    {
        return Err(Diagnostic::new(
            "rust_policy_invalid",
            "source list must be unique and contain src/lib.rs",
        ));
    }
    let found = source_paths(root, crate_dir)?;
    if found != declared {
        return Err(Diagnostic::new(
            "rust_source_closure_mismatch",
            "declared sources must equal all local Rust sources, excluding target",
        ));
    }
    let mut result = vec![
        files::binding(&policy.manifest, &bytes),
        files::binding(&lock, &lock_bytes),
    ];
    for source in &policy.sources {
        let bytes = inputs.add(root, source)?;
        let source_text = std::str::from_utf8(&bytes).map_err(files::input)?;
        syn::parse_file(source_text)
            .map_err(|e| Diagnostic::new("rust_source_invalid", e.to_string()))?;
        let tokens: proc_macro2::TokenStream =
            source_text.parse().map_err(|e: proc_macro2::LexError| {
                Diagnostic::new("rust_source_invalid", e.to_string())
            })?;
        if has_ambient_input_tokens(tokens) {
            return Err(Diagnostic::new(
                "rust_source_closure_unsupported",
                "include, compile-time environment and path attributes are outside the selected source closure",
            ));
        }
        result.push(files::binding(source, &bytes));
    }
    if policy.tests.is_empty()
        || policy.tests.iter().collect::<BTreeSet<_>>().len() != policy.tests.len()
        || policy.tests.iter().any(|s| {
            s.is_empty()
                || s.starts_with('-')
                || !s
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
        })
    {
        return Err(Diagnostic::new(
            "rust_policy_invalid",
            "select unique exact named Rust libtests",
        ));
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

// Raw token groups include macro_rules! bodies and cfg_attr. The initial
// selected checker conservatively reserves these identifiers everywhere.
fn has_ambient_input_tokens(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Group(group) => has_ambient_input_tokens(group.stream()),
        proc_macro2::TokenTree::Ident(ident) => [
            "include",
            "include_str",
            "include_bytes",
            "env",
            "option_env",
            "path",
        ]
        .contains(&ident.to_string().as_str()),
        _ => false,
    })
}

fn source_paths(root: &Path, dir: &Path) -> Result<BTreeSet<String>> {
    fn visit(root: &Path, dir: &Path, paths: &mut BTreeSet<String>) -> Result<()> {
        let full = root.join(dir);
        if fs::symlink_metadata(&full)
            .map_err(files::input)?
            .file_type()
            .is_symlink()
        {
            return Err(Diagnostic::new("invalid_path", "crate links unsupported"));
        }
        for entry in fs::read_dir(full).map_err(files::input)? {
            let entry = entry.map_err(files::input)?;
            let relative = dir.join(entry.file_name());
            if entry.file_name() == "target" || entry.file_name() == ".git" {
                continue;
            }
            let meta = entry.file_type().map_err(files::input)?;
            if meta.is_symlink() {
                return Err(Diagnostic::new("invalid_path", "crate links unsupported"));
            }
            if meta.is_dir() {
                visit(root, &relative, paths)?;
            } else if relative.extension().is_some_and(|e| e == "rs") {
                paths.insert(files::relative(&relative)?);
            }
        }
        Ok(())
    }
    let mut paths = BTreeSet::new();
    visit(root, dir, &mut paths)?;
    Ok(paths)
}

pub(super) fn check(
    context: &Context,
    snapshot: &Path,
    policy: &RustPolicy,
    symbol: &str,
    inputs: Vec<FileBinding>,
) -> Result<RustWitness> {
    if !cfg!(target_os = "linux") {
        return Err(Diagnostic::new(
            "rust_target_unsupported",
            "selected prototype currently supports a native Linux toolchain",
        ));
    }
    let root = context
        .rust_toolchain_dir
        .clone()
        .or_else(|| std::env::var_os("RUST_LEAN_RUST_TOOLCHAIN_DIR").map(PathBuf::from))
        .ok_or_else(|| {
            Diagnostic::new(
                "rust_toolchain_unavailable",
                "set trusted RUST_LEAN_RUST_TOOLCHAIN_DIR or startup toolchain directory",
            )
        })?
        .canonicalize()
        .map_err(|e| Diagnostic::new("rust_toolchain_unavailable", e.to_string()))?;
    for dir in ["home", "cargo-home", "target"] {
        fs::create_dir_all(snapshot.join(dir)).map_err(files::input)?;
    }
    let rustc = run(&root.join("bin/rustc"), &["-Vv".into()], snapshot, &root)?;
    let cargo = run(&root.join("bin/cargo"), &["-Vv".into()], snapshot, &root)?;
    if !rustc
        .lines()
        .any(|line| line == format!("commit-hash: {}", policy.rustc_commit))
        || !rustc
            .lines()
            .any(|line| line == format!("release: {}", policy.rustc_release))
        || !rustc
            .lines()
            .any(|line| line == format!("host: {}", policy.target))
    {
        return Err(Diagnostic::new(
            "rust_toolchain_mismatch",
            "rustc commit/release/host differs from selected policy",
        ));
    }
    let toolchain_hash = files::distribution_hash(&root)?;
    let linker_hash = host_linker_hash()?;
    let args = [
        "test",
        "--manifest-path",
        &policy.manifest,
        "--lib",
        "--no-run",
        "--offline",
        "--frozen",
        "--target",
        &policy.target,
        "--message-format=json",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();
    let events = run(&root.join("bin/cargo"), &args, snapshot, &root)?;
    let mut executables = Vec::new();
    for line in events.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value["reason"] == "compiler-artifact" && value["profile"]["test"] == true {
            if let Some(path) = value["executable"].as_str() {
                executables.push(PathBuf::from(path));
            }
        }
    }
    if executables.len() != 1 {
        return Err(Diagnostic::new(
            "rust_checker_artifact_invalid",
            "expected exactly one compiled libtest artifact",
        ));
    }
    let executable = executables.remove(0).canonicalize().map_err(files::input)?;
    if !executable.starts_with(snapshot.join("target")) {
        return Err(Diagnostic::new(
            "rust_checker_artifact_invalid",
            "test artifact leaves private target",
        ));
    }
    let executable_sha256 = files::file_hash(&executable)?;
    let names = run(
        &executable,
        &["--list".into(), "--format=terse".into()],
        snapshot,
        &root,
    )?;
    let mut tests = Vec::new();
    for test in &policy.tests {
        if !names.lines().any(|name| name == format!("{test}: test")) {
            return Err(Diagnostic::new(
                "rust_test_missing",
                format!("exact test {test} was not discovered"),
            ));
        }
        let events = run(
            &executable,
            &[
                "--exact".into(),
                test.clone(),
                "-Z".into(),
                "unstable-options".into(),
                "--format=json".into(),
            ],
            snapshot,
            &root,
        )?;
        let events: Vec<Value> = events
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| Diagnostic::new("rust_test_protocol_invalid", e.to_string()))?;
        let ok = events
            .iter()
            .filter(|e| e["type"] == "test" && e["event"] == "ok" && e["name"] == *test)
            .count();
        let suite = events.iter().any(|e| {
            e["type"] == "suite"
                && e["event"] == "ok"
                && e["passed"] == 1
                && e["failed"] == 0
                && e["ignored"] == 0
        });
        if ok != 1 || !suite {
            return Err(Diagnostic::new(
                "rust_test_not_passed",
                format!("exact test {test} did not report one executed pass"),
            ));
        }
        tests.push(TestOutcome {
            name: test.clone(),
            outcome: "passed".into(),
        });
    }
    if files::file_hash(&executable)? != executable_sha256
        || files::distribution_hash(&root)? != toolchain_hash
        || host_linker_hash()? != linker_hash
    {
        return Err(Diagnostic::new(
            "rust_toolchain_unstable",
            "compiler, linker or test executable changed during execution",
        ));
    }
    Ok(RustWitness {
        execution_id: policy.execution_id.clone(),
        checker: "cargo-exact-libtests/1".into(),
        outcome: "passed".into(),
        symbol: symbol.into(),
        inputs,
        dependency_policy: "closed-local-crate-no-cargo-dependencies/1".into(),
        target: policy.target.clone(),
        tests,
        toolchain: RustToolchain {
            rustc_version: rustc,
            cargo_version: cargo,
            bin_lib_sha256: toolchain_hash,
            host_linker_sha256: linker_hash,
        },
        executable_sha256,
    })
}

fn host_linker_hash() -> Result<String> {
    let paths = ["/usr/bin/cc", "/usr/bin/ld", "/usr/bin/as"];
    let bindings = paths
        .iter()
        .map(|p| files::file_hash(Path::new(p)).map(|h| (*p, h)))
        .collect::<Result<Vec<_>>>()?;
    Ok(files::hash(
        &serde_json::to_vec(&bindings).map_err(files::input)?,
    ))
}

pub(super) fn recheck_toolchain(context: &Context, witness: &RustWitness) -> Result<()> {
    let root = context
        .rust_toolchain_dir
        .clone()
        .or_else(|| std::env::var_os("RUST_LEAN_RUST_TOOLCHAIN_DIR").map(PathBuf::from))
        .ok_or_else(|| Diagnostic::new("rust_toolchain_unavailable", "Rust toolchain disappeared"))?
        .canonicalize()
        .map_err(files::input)?;
    if files::distribution_hash(&root)? != witness.toolchain.bin_lib_sha256
        || host_linker_hash()? != witness.toolchain.host_linker_sha256
    {
        return Err(Diagnostic::new(
            "rust_toolchain_unstable",
            "Rust toolchain changed while the Lean contribution ran",
        ));
    }
    Ok(())
}

fn run(program: &Path, args: &[String], cwd: &Path, toolchain: &Path) -> Result<String> {
    let output = tempfile::NamedTempFile::new().map_err(files::input)?;
    let errors = tempfile::NamedTempFile::new().map_err(files::input)?;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", toolchain.join("bin").display()),
        )
        .env("HOME", cwd.join("home"))
        .env("CARGO_HOME", cwd.join("cargo-home"))
        .env("CARGO_TARGET_DIR", cwd.join("target"))
        .env("RUSTC", toolchain.join("bin/rustc"))
        .env("RUSTDOC", toolchain.join("bin/rustdoc"))
        .env(
            "RUSTFLAGS",
            format!("--remap-path-prefix={}=/selected-proof", cwd.display()),
        )
        .env("CARGO_PROFILE_TEST_DEBUG", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_INCREMENTAL", "0")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .stdout(output.reopen().map_err(files::input)?)
        .stderr(errors.reopen().map_err(files::input)?);
    let mut child = command
        .group_spawn()
        .map_err(|e| Diagnostic::new("rust_checker_unavailable", e.to_string()))?;
    let start = Instant::now();
    let status = loop {
        let over = [output.path(), errors.path()].iter().any(|p| {
            fs::metadata(p)
                .map(|m| m.len() > 1024 * 1024)
                .unwrap_or(true)
        });
        if start.elapsed() > Duration::from_secs(120) || over {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Diagnostic::new(
                "rust_checker_limit",
                "checker exceeded 120 seconds or 1 MiB output",
            ));
        }
        if let Some(status) = child.try_wait().map_err(files::input)? {
            break status;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = fs::read_to_string(output.path()).map_err(files::input)?;
    if !status.success() {
        let stderr = fs::read_to_string(errors.path()).map_err(files::input)?;
        return Err(Diagnostic::new(
            "rust_checker_failed",
            format!(
                "{} exited {status}: {stderr}{stdout}",
                program.file_name().unwrap_or_default().to_string_lossy()
            ),
        ));
    }
    Ok(stdout)
}
