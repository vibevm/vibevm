//! §10's vibe-bin gate, end to end and for real:
//!
//! > "deploy a VibeVM tool through an isolated `~/.vibe/bin`, run its
//! > version-free launcher, update it and roll it back"
//!
//! Everything below is real. The payload is a genuine executable Cargo
//! produced from a dependency-free fixture crate in a temp directory (the
//! same posture, and the same accepted cost, as the build phase's one real
//! end-to-end next door); the launcher is written by the shipped provider
//! through the shipped executor; and it is RUN as a child process, so what
//! the assertions read is the payload's own stdout rather than a digest
//! this suite computed. ONE `cargo build` produces BOTH generations — the
//! fixture crate declares two `[[bin]]` targets — so the real-build cost is
//! paid once.
//!
//! The isolation is by construction, not by convention: the settings root,
//! the project and the deployment state home are three temp trees named as
//! DATA, and no cell under test resolves a home. `PATH` is never touched;
//! the launcher is invoked by its absolute path.
//!
//! ## Why the order is deploy → update → re-deploy → saga → undeploy
//!
//! The saga rolls back the generation the FAILING run applied, restoring
//! the pointer that generation displaced. So for the rolled-back launcher
//! to answer with the ORIGINAL payload again (§7.1.0 ruling 6's own
//! sentence), the failing run has to be the one that moved the pointer OFF
//! the original — which is step 4, after the update has already been
//! observed answering with the new output in step 2. Step 3 is what makes
//! that possible AND proves ruling 4's write-once store: re-deploying the
//! original writes no second payload. Step 5 re-establishes a live receipt,
//! because a rolled-back receipt owns nothing by the engine's own law
//! (§7.0's ratification 5) and `undeploy` may only remove what a receipt
//! still owns.

use std::path::{Path, PathBuf};
use std::process::Command;

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactKind, ArtifactOutput, DeployTarget, MechanismRoutes,
};
use vibe_wire::generated::artifact_record::ArtifactShape;

use super::launcher::LauncherFlavour;
use super::store;
use crate::mechanism::contain::forward_slashed;
use crate::mechanism::deploy::{
    DeployError, DeployExecution, DeploySelection, deploy_state_home, execute_deploy_targets,
    list_deployments, undeploy_targets,
};
use crate::mechanism::package::support::{config, empty_world, key, registry, temp};
use crate::mechanism::record::{RecordFreshness, RecordInputs, build_record, write_record};
use crate::mechanism::{BUILTIN_VIBE_BIN_PIN, BuildExecution, execute_build_targets};

/// The command alias the deployment installs under.
const COMMAND: &str = "vibe-e2e-tool";

/// The one Cargo package, with the two `[[bin]]` targets one build
/// produces. Its own `[workspace]` so a workspace above the temp directory
/// cannot absorb it.
const MANIFEST: &str = concat!(
    "[package]\nname = \"vibe-r8-vibebin\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n",
    "[[bin]]\nname = \"vbnorig\"\npath = \"src/original.rs\"\n\n",
    "[[bin]]\nname = \"vbnupd\"\npath = \"src/updated.rs\"\n\n",
    "[workspace]\n",
);

const ORIGINAL_SOURCE: &str = "fn main() {\n    println!(\"vibe-bin payload ORIGINAL\");\n}\n";
const UPDATED_SOURCE: &str = "fn main() {\n    println!(\"vibe-bin payload UPDATED\");\n}\n";
const ORIGINAL_OUTPUT: &str = "vibe-bin payload ORIGINAL";
const UPDATED_OUTPUT: &str = "vibe-bin payload UPDATED";

/// The three temp roots this gate runs inside.
struct World {
    project: TempDir,
    settings: TempDir,
    state_home: PathBuf,
}

impl World {
    /// The absolute path of one settings-relative identity.
    fn at(&self, relative: &str) -> PathBuf {
        store::join(self.settings.path(), relative)
    }

    /// The launcher's own path on this host.
    fn launcher(&self) -> PathBuf {
        self.at(&format!(
            "bin/{COMMAND}{}",
            LauncherFlavour::NATIVE.launcher_suffix()
        ))
    }

    /// The active-payload pointer's path.
    fn pointer(&self) -> PathBuf {
        self.at(&format!("bin/{COMMAND}.current"))
    }
}

/// Deploy the named target ids through the SHIPPED executor.
fn deploy(world: &World, targets: &[DeployTarget]) -> Result<(), DeployError> {
    let ids: Vec<&str> = targets.iter().map(|row| row.id.as_str()).collect();
    let selection = selection(&ids);
    let plane = registry(&empty_world());
    let routes = MechanismRoutes::default();
    execute_deploy_targets(&DeployExecution {
        project_root: world.project.path(),
        targets,
        selection: &selection,
        registry: &plane,
        routes: &routes,
        state_home: &world.state_home,
        settings_root: world.settings.path(),
        project: "org.example/vibebin-e2e",
        package: None,
        created_at: "2026-08-30T12:00:00Z",
    })
    .map(|_| ())
}

/// One profile selection over the given target ids.
fn selection(ids: &[&str]) -> DeploySelection {
    DeploySelection {
        profile: "local".to_owned(),
        targets: ids.iter().map(|id| (*id).to_owned()).collect(),
    }
}

/// The deploy target that installs one artifact under [`COMMAND`].
///
/// The id is stable across generations while the artifact it names is not:
/// a deployment is keyed by project/package/TARGET, so two rows with one id
/// and different artifacts are two generations of one deployment — which is
/// exactly what "the artifact this target deploys was rebuilt" looks like.
fn tool(artifact: &str) -> DeployTarget {
    DeployTarget {
        id: "vibe-e2e".to_owned(),
        artifact: artifact.to_owned(),
        mechanism: key("deploy:vibe-bin"),
        provider: None,
        depends_on: None,
        config: Some(config(&format!("command = \"{COMMAND}\""))),
    }
}

/// The second target of the failing profile: a vibe-bin row over a
/// `file`-kind artifact, which §7.1.0 ruling 7 refuses by name. A realistic
/// failure rather than an injected one — the saga is driven by a law this
/// atom landed.
fn refuser() -> DeployTarget {
    DeployTarget {
        id: "vibe-e2e-refuser".to_owned(),
        artifact: "notes.md".to_owned(),
        mechanism: key("deploy:vibe-bin"),
        provider: None,
        depends_on: None,
        config: Some(config("command = \"vibe-e2e-notes\"")),
    }
}

/// RUN the launcher as a child process and read the payload's own output.
///
/// On Windows a `.cmd` is executed by the command interpreter, exactly as a
/// shell on `PATH` would; elsewhere the `#!/bin/sh` twin is executed
/// directly. `PATH` itself is never modified: the launcher is invoked by
/// its absolute path.
fn run_launcher(path: &Path) -> (String, i32) {
    let mut command = if cfg!(windows) {
        let mut interpreter = Command::new("cmd");
        interpreter.arg("/c").arg(path);
        interpreter
    } else {
        Command::new(path)
    };
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("the launcher `{}` runs: {error}", path.display()));
    (
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// Build the fixture crate's two executables through the SHIPPED build
/// executor, so both artifacts arrive with the engine's own A2 records.
fn build_payloads(root: &Path) {
    write(root, "Cargo.toml", MANIFEST);
    write(root, "src/original.rs", ORIGINAL_SOURCE);
    write(root, "src/updated.rs", UPDATED_SOURCE);
    let target = ArtifactBuildTarget {
        id: "tools".to_owned(),
        mechanism: key("build:cargo"),
        provider: None,
        workdir: ".".to_owned(),
        inputs: None,
        outputs: vec![
            output("original.exe", "vbnorig"),
            output("updated.exe", "vbnupd"),
        ],
        config: Some(config("offline = true\n")),
    };
    let plane = registry(&empty_world());
    let routes = MechanismRoutes::default();
    let targets = [target];
    if let Err(error) = execute_build_targets(&BuildExecution {
        project_root: root,
        targets: &targets,
        registry: &plane,
        routes: &routes,
        build_root: BuildExecution::default_build_root(),
        offline: true,
        created_at: "2026-08-30T11:00:00Z",
    }) {
        panic!("the fixture crate builds: {error}");
    }
}

/// One declared executable output selecting one `[[bin]]` target.
fn output(id: &str, bin: &str) -> ArtifactOutput {
    ArtifactOutput {
        id: id.to_owned(),
        kind: ArtifactKind::Executable,
        select: Some(config(&format!(
            "package = \"vibe-r8-vibebin\"\nbin = \"{bin}\""
        ))),
    }
}

/// A `file`-kind artifact and its A2 record — what the refusing target of
/// the failing profile names.
fn write_notes(root: &Path) {
    let relative = "docs/notes.md";
    write(root, relative, "not a program\n");
    let absolute = root.join(relative);
    let digest = match crate::mechanism::contain::digest_file(&absolute) {
        Ok((digest, _)) => digest,
        Err(fault) => panic!("the fixture note digests: {}", fault.reason()),
    };
    let record = build_record(&RecordInputs {
        target: "notes",
        mechanism: &key("package:static-skill"),
        provider_key: "org.vibevm/vibe#static-skill",
        provider_version: None,
        provider_hash: None,
        output_id: "notes.md",
        kind: ArtifactKind::File,
        shape: ArtifactShape::File,
        digest: &digest,
        path_absolute: &forward_slashed(&absolute),
        path_relative: relative,
        freshness: RecordFreshness::default(),
        platform: None,
        media_type: None,
        created_at: "2026-08-30T11:00:00Z",
        evidence: "fixture note".to_owned(),
    })
    .expect("the fixture record builds");
    write_record(root, &record).expect("the fixture record writes");
}

/// Write one fixture file, creating its parents.
fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        panic!("the fixture directory creates: {error}");
    }
    if let Err(error) = std::fs::write(&path, contents) {
        panic!("the fixture file writes: {error}");
    }
}

/// How many entries one directory holds; zero when it is not there.
fn count(directory: &Path) -> usize {
    std::fs::read_dir(directory).map_or(0, |entries| entries.flatten().count())
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn a_real_tool_deploys_runs_updates_rolls_back_lists_and_undeploys() {
    let world = {
        let project = temp();
        let settings = temp();
        let state_home = deploy_state_home(settings.path());
        World {
            project,
            settings,
            state_home,
        }
    };
    build_payloads(world.project.path());
    write_notes(world.project.path());
    let original = tool("original.exe");
    let updated = tool("updated.exe");

    // ---- 1. deploy, and RUN the version-free launcher -------------------
    deploy(&world, std::slice::from_ref(&original)).expect("the first generation deploys");
    let launcher_bytes = std::fs::read(world.launcher()).expect("the launcher was written");
    let first_pointer = std::fs::read(world.pointer()).expect("the pointer was written");
    assert_eq!(
        run_launcher(&world.launcher()),
        (ORIGINAL_OUTPUT.to_owned(), 0),
        "the version-free launcher really runs the deployed payload",
    );
    assert_eq!(count(&world.at("store")), 1, "one payload after one deploy");
    assert_eq!(
        count(&world.at("bin")),
        2,
        "and exactly the launcher and its pointer",
    );

    // ---- 2. update: the POINTER moves, the launcher does not ------------
    deploy(&world, std::slice::from_ref(&updated)).expect("the second generation deploys");
    assert_eq!(
        run_launcher(&world.launcher()),
        (UPDATED_OUTPUT.to_owned(), 0),
        "the same launcher now answers with the new payload's output",
    );
    assert_eq!(
        std::fs::read(world.launcher()).expect("the launcher is still there"),
        launcher_bytes,
        "an update rewrites the POINTER, never the launcher (§7.1.0 ruling 3)",
    );
    let second_pointer = std::fs::read(world.pointer()).expect("the pointer is still there");
    assert_ne!(
        second_pointer, first_pointer,
        "and the pointer is what moved"
    );
    assert_eq!(count(&world.at("store")), 2, "both payloads are kept");

    // ---- 3. back to the original: the CAS write is idempotent -----------
    deploy(&world, std::slice::from_ref(&original)).expect("the third generation deploys");
    assert_eq!(run_launcher(&world.launcher()).0, ORIGINAL_OUTPUT);
    assert_eq!(
        count(&world.at("store")),
        2,
        "a payload already at its own address is written once and never again",
    );

    // ---- 4. the saga: a two-target profile whose second target fails ----
    let failing = [updated.clone(), refuser()];
    let error = deploy(&world, &failing).expect_err("the second target refuses by artifact kind");
    let DeployError::Saga {
        target,
        rolled_back,
        retained,
        ..
    } = &error
    else {
        panic!("expected the saga refusal, got: {error}");
    };
    assert_eq!(target, "vibe-e2e-refuser");
    assert_eq!(rolled_back, "vibe-e2e");
    assert_eq!(retained, "none");
    assert_eq!(
        run_launcher(&world.launcher()),
        (ORIGINAL_OUTPUT.to_owned(), 0),
        "the rolled-back launcher runs the ORIGINAL payload again (§7.1.0 ruling 6)",
    );
    assert_eq!(
        std::fs::read(world.launcher()).expect("the launcher survived the rollback"),
        launcher_bytes,
        "a rollback restores the pointer and leaves the version-free launcher",
    );
    assert_eq!(
        count(&world.at("store")),
        2,
        "and a rollback never touches a content-addressed payload",
    );

    // ---- 5. a live generation again, and the listing --------------------
    deploy(&world, std::slice::from_ref(&original)).expect("the deployment is live again");
    let rows = list_deployments(&world.state_home).expect("the state home lists");
    assert_eq!(rows.len(), 1, "one deployment, five generations");
    let row = &rows[0];
    assert_eq!(row.target, "vibe-e2e");
    assert_eq!(row.profile, "local");
    assert_eq!(row.provider, BUILTIN_VIBE_BIN_PIN);
    assert_eq!(row.status.as_str(), "verified");
    assert_eq!(row.scope, "user");
    assert_eq!(
        row.resources, 2,
        "the launcher and the pointer, never the payload"
    );
    assert!(row.reversible);

    // ---- 6. undeploy: the two owned files go, the payloads stay ---------
    let removals = undeploy_targets(&DeployExecution {
        project_root: world.project.path(),
        targets: std::slice::from_ref(&original),
        selection: &selection(&["vibe-e2e"]),
        registry: &registry(&empty_world()),
        routes: &MechanismRoutes::default(),
        state_home: &world.state_home,
        settings_root: world.settings.path(),
        project: "org.example/vibebin-e2e",
        package: None,
        created_at: "2026-08-30T12:00:00Z",
    })
    .expect("the inverse deployment runs");

    assert_eq!(removals.len(), 1);
    assert_eq!(removals[0].removed.len(), 2, "launcher and pointer");
    assert!(!world.launcher().exists(), "the launcher is gone");
    assert!(!world.pointer().exists(), "the pointer is gone");
    assert_eq!(
        count(&world.at("bin")),
        0,
        "the destination directory holds nothing owned afterwards",
    );
    assert_eq!(
        count(&world.at("store")),
        2,
        "and both payloads remain as §7.1.0 ruling 4's disclosed store garbage",
    );
}
