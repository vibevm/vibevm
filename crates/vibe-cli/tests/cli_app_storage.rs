//! Public Linux fixture for storage libraries (PROP-059 `#storage-libraries`,
//! `#availability`, `#portability`, `#move`, `#acceptance`): install into a
//! second library, detach it and observe `unavailable` through `app list`,
//! `app run`, the PATH proxy and a refused uninstall, attach it again, move
//! the installation to the default library, sweep a simulated interruption
//! with `storage gc`, and refuse moving non-portable and external
//! installations.

#![cfg(target_os = "linux")]

mod common;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};
use vibe_wire::generated::application_storage_gc::ApplicationStorageGc;
use vibe_wire::generated::application_storage_list::{ApplicationStorageList, Status};
use vibe_wire::generated::application_storage_move::ApplicationStorageMove;

const TOOL: &str = "org.example/tool@1.0.0";

struct Registry {
    root: tempfile::TempDir,
}

impl Registry {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self) -> PathBuf {
        self.root.path().join("registry")
    }

    fn package_dir(&self, name: &str) -> PathBuf {
        let dir = self.path().join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `org.example/<name>@1.0.0` in slot `<name>` with public command
    /// `<name>`; `application` extends the `[application]` table and
    /// `installation` is the whole installation table body.
    fn manifest(&self, name: &str, application: &str, installation: &str) {
        fs::write(
            self.package_dir(name).join("vibe.toml"),
            format!(
                "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
                 [application]\nid = \"{name}\"\n{application}\n\
                 [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\n\
                 [installer_options.installation]\n{installation}\n\
                 [[installer_options.launchers]]\nid = \"main\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\nargs = [\"fixed\"]\ndefault = true\n\n\
                 [[installer_options.public_commands]]\nname = \"{name}\"\nlauncher = \"main\"\n",
                arch = std::env::consts::ARCH,
            ),
        )
        .unwrap();
    }

    /// A zip package whose `bin/tool` prints `<NAME> <args>`.
    fn zip_package(&self, name: &str, application: &str) {
        let dir = self.package_dir(name);
        let mut zip = zip::ZipWriter::new(fs::File::create(dir.join("tool.zip")).unwrap());
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file("bin/tool", options).unwrap();
        let marker = name.to_uppercase();
        zip.write_all(format!("#!/bin/sh\necho \"{marker} $*\"\n").as_bytes())
            .unwrap();
        zip.finish().unwrap();
        let sha = format!(
            "{:x}",
            Sha256::digest(fs::read(dir.join("tool.zip")).unwrap())
        );
        self.manifest(
            name,
            application,
            &format!("type = \"zip\"\nsource = \"tool.zip\"\nsha256 = \"{sha}\"\n"),
        );
    }

    /// A shell package whose install writes `bin/tool` under an external root.
    fn shell_package(&self, name: &str, app_root: &Path) {
        let dir = self.package_dir(name);
        let script = dir.join("install.sh");
        fs::write(
            &script,
            "#!/bin/sh\nset -e\nmkdir -p \"$VIBE_APP_ROOT/bin\"\nprintf '#!/bin/sh\\necho EXT \"$@\"\\n' > \"$VIBE_APP_ROOT/bin/tool\"\nchmod 755 \"$VIBE_APP_ROOT/bin/tool\"\n",
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let run = format!("sh '{}'", script.display());
        self.manifest(
            name,
            "",
            &format!(
                "type = \"shell\"\napp_root = \"{root}\"\ninstall = \"{run}\"\nforce_install = \"{run}\"\nuninstall = \"true\"\nforce_uninstall = \"true\"\ninterpreter = \"posix_sh\"\n",
                root = app_root.display()
            ),
        );
    }

    fn vibe(&self, user: &UserScratch, args: &[&str]) -> Output {
        user.vibe()
            .current_dir(self.root.path())
            .args(args)
            .output()
            .unwrap()
    }

    fn install(&self, user: &UserScratch, package: &str, extra: &[&str]) -> Output {
        let registry = self.path();
        let mut args = vec!["install", "-g", package, "--registry"];
        args.push(registry.to_str().unwrap());
        args.extend_from_slice(extra);
        self.vibe(user, &args)
    }
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn failed(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> Value {
    serde_json::from_str(&ok(output)).unwrap()
}

/// The `app list` row of `key`.
fn listed(registry: &Registry, user: &UserScratch, key: &str) -> Value {
    let listing = json(&registry.vibe(user, &["app", "list", "--json"]));
    listing["installations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["key"] == key)
        .cloned()
        .unwrap()
}

/// `tool` resolved through PATH with `opt/bin` first, as a shell would.
fn proxy(user: &UserScratch) -> Output {
    let mut path = OsString::from(user.settings.join("opt/bin"));
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    Command::new("tool")
        .arg("arg")
        .env("PATH", path)
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap()
}

#[test]
fn two_libraries_detach_attach_move_and_gc() {
    let registry = Registry::new();
    let user = UserScratch::new();
    registry.zip_package("tool", "");
    let usb = registry.root.path().join("usb");
    let usb_text = usb.to_str().unwrap();

    let added = json(&registry.vibe(
        &user,
        &["storage", "add", usb_text, "--id", "usb", "--json"],
    ));
    let typed: ApplicationStorageList = serde_json::from_value(added).unwrap();
    let ids: Vec<&str> = typed.libraries.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["default", "usb"]);
    assert!(usb.join("vibe-library.toml").is_file());

    ok(&registry.install(&user, "org.example/tool", &["--library", "usb"]));
    let installed = usb.join("apps/tool/org.example.tool@1.0.0");
    assert!(installed.join("bin/tool").is_file());
    let row = listed(&registry, &user, TOOL);
    assert_eq!(row["library"], "usb");
    assert_eq!(row["status"], "ready");
    let text = ok(&registry.vibe(&user, &["storage", "list"]));
    assert!(text.contains(&format!("usb: {usb_text} (ready)")), "{text}");
    assert!(text.contains(&format!("tool: {TOOL} (ready)")), "{text}");

    // Detach: every surface reports the same reason; nothing is removed.
    let text = ok(&registry.vibe(&user, &["storage", "detach", "usb"]));
    assert!(text.contains("unlisted"), "{text}");
    assert!(installed.join("bin/tool").is_file(), "detach keeps files");
    let row = listed(&registry, &user, TOOL);
    assert_eq!(row["status"], "unavailable");
    assert_eq!(row["library"], "usb");
    let reason = row["reason"].as_str().unwrap().to_owned();
    assert!(
        reason.contains("library `usb` is not configured"),
        "{reason}"
    );
    let refused = failed(&registry.vibe(&user, &["app", "run", TOOL, "main"]));
    assert!(refused.contains(&reason), "{refused}");
    let through_proxy = proxy(&user);
    assert_eq!(through_proxy.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&through_proxy.stderr);
    assert!(stderr.contains(&reason), "{stderr}");
    let refused = failed(&registry.vibe(&user, &["uninstall", "-g", TOOL]));
    assert!(refused.contains("is unavailable"), "{refused}");
    let listing: ApplicationStorageList =
        serde_json::from_value(json(&registry.vibe(&user, &["storage", "list", "--json"])))
            .unwrap();
    let detached = listing
        .libraries
        .iter()
        .find(|row| row.id == "usb")
        .unwrap();
    assert_eq!(detached.path, None);
    assert_eq!(detached.status, Status::Unavailable);
    assert_eq!(detached.installations[0].key, TOOL);
    let refused = failed(&registry.vibe(&user, &["storage", "detach", "default"]));
    assert!(refused.contains("cannot be detached"), "{refused}");

    // Attach: ready again with no other command.
    ok(&registry.vibe(&user, &["storage", "attach", usb_text]));
    assert_eq!(listed(&registry, &user, TOOL)["status"], "ready");
    assert_eq!(
        ok(&registry.vibe(&user, &["app", "run", TOOL, "main", "--", "x"])),
        "TOOL fixed x\n"
    );

    // Move to the default library.
    let moved: ApplicationStorageMove = serde_json::from_value(json(&registry.vibe(
        &user,
        &["storage", "move", TOOL, "--to", "default", "--json"],
    )))
    .unwrap();
    let target = user.settings.join("opt/apps/tool/org.example.tool@1.0.0");
    assert_eq!(moved.from, "usb");
    assert_eq!(moved.to, "default");
    assert_eq!(Path::new(&moved.old_app_root), installed);
    assert_eq!(Path::new(&moved.new_app_root), target);
    assert!(!installed.exists(), "the usb copy is gone");
    assert!(target.join("bin/tool").is_file());
    let row = listed(&registry, &user, TOOL);
    assert_eq!(row["library"], "default");
    assert_eq!(row["app_root"], target.display().to_string());
    assert_eq!(
        ok(&registry.vibe(&user, &["app", "run", TOOL, "main"])),
        "TOOL fixed\n"
    );
    assert_eq!(ok(&proxy(&user)), "TOOL fixed arg\n");

    // An interrupted move and a stray directory are explained by gc.
    let leftovers = [
        user.settings.join("opt/apps/tool/.moving-x-123"),
        user.settings.join("opt/apps/tool/orphan"),
    ];
    for leftover in &leftovers {
        fs::create_dir_all(leftover.join("bin")).unwrap();
    }
    let report: ApplicationStorageGc =
        serde_json::from_value(json(&registry.vibe(&user, &["storage", "gc", "--json"]))).unwrap();
    let found: Vec<PathBuf> = report
        .orphans
        .iter()
        .map(|row| PathBuf::from(&row.path))
        .collect();
    assert_eq!(found, leftovers);
    assert!(report.orphans.iter().all(|row| row.library == "default"));
    assert!(!report.deleted);
    assert!(leftovers.iter().all(|leftover| leftover.is_dir()));
    let text = ok(&registry.vibe(&user, &["storage", "gc", "--delete"]));
    assert!(
        text.contains("removed 2 unreferenced directories"),
        "{text}"
    );
    assert!(leftovers.iter().all(|leftover| !leftover.exists()));
    assert!(
        target.join("bin/tool").is_file(),
        "the installed directory stays"
    );
    let text = ok(&registry.vibe(&user, &["storage", "gc"]));
    assert!(text.contains("found 0 unreferenced directories"), "{text}");
}

#[test]
fn non_portable_and_external_installations_refuse_to_move() {
    let registry = Registry::new();
    let user = UserScratch::new();
    registry.zip_package("pinned", "portable = false\n");
    let external_root = registry.root.path().join("external-root");
    registry.shell_package("ext", &external_root);
    let usb = registry.root.path().join("usb");
    ok(&registry.vibe(&user, &["storage", "add", usb.to_str().unwrap()]));

    ok(&registry.install(&user, "org.example/pinned", &["--library", "usb"]));
    let pinned = "org.example/pinned@1.0.0";
    assert_eq!(listed(&registry, &user, pinned)["portable"], false);
    let refused = failed(&registry.vibe(&user, &["storage", "move", pinned, "--to", "default"]));
    assert!(refused.contains("is not portable"), "{refused}");
    assert!(refused.contains("portable = false"), "{refused}");
    assert!(!user.settings.join("opt/apps/pinned").exists(), "no copy");
    assert!(
        usb.join("apps/pinned/org.example.pinned@1.0.0/bin/tool")
            .is_file()
    );

    ok(&registry.install(&user, "org.example/ext", &[]));
    let ext = "org.example/ext@1.0.0";
    let refused = failed(&registry.vibe(&user, &["storage", "move", ext, "--to", "usb"]));
    assert!(
        refused.contains("external installations are never portable"),
        "{refused}"
    );
    assert!(!usb.join("apps/ext").exists(), "no copy");
    assert!(external_root.join("bin/tool").is_file());
}

/// PROP-059 `#hidden-installations`: a hidden installation is a whole
/// installation (payload, record, movable) that exposes nothing, and it
/// stays hidden through a move and an update.
#[test]
fn a_hidden_installation_moves_and_updates_without_being_exposed() {
    let registry = Registry::new();
    let user = UserScratch::new();
    registry.zip_package("tool", "");
    let usb = registry.root.path().join("usb");
    ok(&registry.vibe(
        &user,
        &["storage", "add", usb.to_str().unwrap(), "--id", "usb"],
    ));

    let report = json(&registry.install(&user, "org.example/tool", &["--hidden", "--json"]));
    assert_eq!(
        report["slotDefault"],
        Value::Null,
        "a hidden installation is never the default"
    );
    assert!(report["message"].as_str().unwrap().contains("hidden"));
    let row = listed(&registry, &user, TOOL);
    assert_eq!(row["hidden"], true);
    assert_eq!(row["default"], false);
    assert!(!user.settings.join("opt/bin/tool").exists(), "no proxy");
    let refused = failed(&registry.vibe(&user, &["app", "alternatives", "tool"]));
    assert!(refused.contains("has no installed provider"), "{refused}");
    let text = ok(&registry.vibe(&user, &["app", "list"]));
    assert!(
        text.contains("hidden installation(s):\n  tool: org.example/tool@1.0.0"),
        "{text}"
    );

    ok(&registry.vibe(&user, &["storage", "move", TOOL, "--to", "usb"]));
    let row = listed(&registry, &user, TOOL);
    assert_eq!(
        (row["library"].as_str(), row["hidden"].as_bool()),
        (Some("usb"), Some(true))
    );
    let registry_path = registry.path();
    ok(&registry.vibe(
        &user,
        &[
            "update",
            "-g",
            "org.example/tool",
            "--registry",
            registry_path.to_str().unwrap(),
        ],
    ));
    assert_eq!(
        listed(&registry, &user, TOOL)["hidden"],
        true,
        "update keeps the hidden state"
    );
    assert!(!user.settings.join("opt/bin/tool").exists());

    let exposed = json(&registry.vibe(&user, &["app", "expose", "org.example/tool", "--json"]));
    assert_eq!(exposed["command"], "expose");
    assert_eq!(
        exposed["slotDefault"], TOOL,
        "an empty slot gains it as its default"
    );
    assert!(user.settings.join("opt/bin/tool").is_file());
    assert_ne!(listed(&registry, &user, TOOL)["hidden"], true);
    let hidden = json(&registry.vibe(&user, &["app", "hide", "org.example/tool", "--json"]));
    assert_eq!(
        hidden["slotDefault"],
        Value::Null,
        "the default is released"
    );
    assert!(!user.settings.join("opt/bin/tool").exists());
}
