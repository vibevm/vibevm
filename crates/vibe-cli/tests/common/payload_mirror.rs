//! The payload-projection fixture of `vibe registry mirror --config`
//! (PROP-075 `#MIRROR-PAYLOAD-PROJECTION`, `#MIRROR-PAYLOAD-BUDGET`).
//!
//! The template-snapshot suite drives the production `org.vibevm.node/node`
//! manifest against upstream's own frozen documents, which is what makes
//! its render fidelity worth something. Payload projection needs the
//! opposite: **the bytes themselves**, and a digest document that
//! describes them. Upstream's recorded checksums describe real
//! multi-megabyte release tarballs, so this fixture declares its own
//! template over its own four-kilobyte payloads and computes the digest
//! documents from them — the digest a run verifies is a real digest of
//! real bytes, and the suite still opens no socket.
//!
//! The template declares `size` on one installer option and not on the
//! other, which is the two halves of `#MIRROR-DRY-RUN-OUTPUT`: a declared
//! size is weighed against the budget, an undeclared one makes the total a
//! lower bound and is counted as such.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use sha2::{Digest, Sha256};

use super::UserScratch;

pub const PACKAGE: &str = "org.corp/tool";
pub const REVISION: &str = "1.0.0";
/// Every payload file is exactly this many bytes, so the one literal
/// `size` the template declares is the truth for every version it
/// produces.
pub const PAYLOAD_BYTES: u64 = 4096;
const ORIGIN: &str = "https://downloads.example/tool/";

/// The versions the fixture's index offers, newest first.
pub const VERSIONS: [&str; 3] = ["3.1.0", "2.4.0", "1.9.0"];

/// The archive options, and the payload file each declares per version.
/// `linux-x64` declares its `size`; `win-x64` does not, which is what
/// makes a projected total a lower bound.
const OPTIONS: [(&str, &str, &str); 2] = [
    ("linux-x64", "tar.gz", "linux-x64"),
    ("win-x64", "zip", "win-x64"),
];

/// The file name of the `src` option's `sources` input — the one a client
/// inside the closed network really installs, because a `sources` input is
/// copied rather than extracted, so the fixture's payload bytes need not
/// be a real archive.
fn source_input(version: &str) -> String {
    format!("tool-{version}-src.bin")
}

fn template_toml() -> String {
    let mut toml = format!(
        "[package]\nname = \"tool\"\ngroup = \"org.corp\"\nkind = \"app\"\n\
         version = \"{REVISION}\"\nepoch = 1\ntemplate = true\n\
         description = \"The quokkaword tool {{version}}, as a portable Vibe application\"\n\
         keywords = [\"quokkaword\", \"tool\"]\n\n\
         [application]\nid = \"tool\"\nkeywords = [\"quokkaword\"]\n\n\
         [template.versions]\nkind = \"json\"\nurl = \"{ORIGIN}index.json\"\n\
         select = \"\"\nversion_field = \"version\"\nstrip_prefix = \"v\"\n\n\
         [template.digests]\nkind = \"sha256sums\"\n\
         url = \"{ORIGIN}v{{version}}/SHA256SUMS\"\n\n\
         [[template.channel]]\nrule = \"truthy\"\nname = \"lts\"\nfrom = \"lts\"\n"
    );
    toml.push_str(&format!(
        "\n[[installer_options]]\nid = \"src\"\nname = \"Tool {{version}} from source\"\n\
         platform = \"linux\"\nplatform_name = \"Linux (glibc)\"\narch = \"{arch}\"\n\n\
         [installer_options.installation]\ntype = \"source_build\"\n\n\
         [[installer_options.installation.sources]]\nname = \"payload\"\n\
         url = \"{ORIGIN}v{{version}}/tool-{{version}}-src.bin\"\n\
         sha256 = \"{{sha256:tool-{{version}}-src.bin}}\"\n\
         size = {PAYLOAD_BYTES}\n\n\
         [installer_options.installation.recipe]\ninterpreter = \"posix_sh\"\n\
         script = \"build.sh\"\n\n\
         [[installer_options.launchers]]\nid = \"tool\"\nname = \"Tool\"\n\
         executable = \"$app_bin/tool\"\nkind = \"command\"\ndefault = true\n",
        arch = std::env::consts::ARCH,
    ));
    for (id, extension, slug) in OPTIONS {
        let platform = if id.starts_with("win") {
            ("windows", "Windows")
        } else {
            ("linux", "Linux (glibc)")
        };
        let format = if extension == "zip" { "zip" } else { "tar_gz" };
        let size = if id == "linux-x64" {
            format!("size = {PAYLOAD_BYTES}\n")
        } else {
            String::new()
        };
        toml.push_str(&format!(
            "\n[[installer_options]]\nid = \"{id}\"\nname = \"Tool {{version}}\"\n\
             platform = \"{}\"\nplatform_name = \"{}\"\narch = \"x86_64\"\n\
             path_dirs = [\"bin\"]\n\n\
             [[installer_options.steps]]\nid = \"tool\"\n\
             title = \"Download and unpack Tool {{version}}\"\nkind = \"archive\"\n\
             format = \"{format}\"\n\
             source = \"{ORIGIN}v{{version}}/tool-{{version}}-{slug}.{extension}\"\n\
             sha256 = \"{{sha256:tool-{{version}}-{slug}.{extension}}}\"\n{size}\
             payload_root = \"tool-{{version}}\"\n",
            platform.0, platform.1
        ));
    }
    toml
}

/// A `localdir` source carrying the template, a `localdir` target to
/// mirror into, the fixture's own release host on disk, and the drop
/// payloads are projected into.
pub struct Fixture {
    pub root: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let fixture = Fixture {
            root: tempfile::tempdir().unwrap(),
        };
        let dir = fixture.source().join(PACKAGE).join(format!("v{REVISION}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), template_toml()).unwrap();
        // The recipe rides the template's own tree, so a client that
        // renders the manifest also has the script it names.
        fs::write(
            dir.join("build.sh"),
            "set -e\nmkdir -p \"$VIBE_OUT/bin\"\n\
             cp \"$VIBE_SOURCES\"/payload/* \"$VIBE_OUT/bin/tool\"\n\
             chmod 755 \"$VIBE_OUT/bin/tool\"\n",
        )
        .unwrap();
        fs::create_dir_all(fixture.target()).unwrap();
        fixture.write_upstream();
        fs::write(
            fixture.root.path().join("remap.toml"),
            format!(
                "[[remap]]\nprefix = \"{ORIGIN}\"\npath = \"{}\"\n\
                 reason = \"the suite's own copy of the tool release host\"\n",
                fixture.dist().display()
            ),
        )
        .unwrap();
        fixture
    }

    pub fn source(&self) -> PathBuf {
        self.root.path().join("source")
    }

    pub fn target(&self) -> PathBuf {
        self.root.path().join("target")
    }

    pub fn dist(&self) -> PathBuf {
        self.root.path().join("dist")
    }

    /// The package drop `[payloads] into` names.
    pub fn drop(&self) -> PathBuf {
        self.root.path().join("drop")
    }

    /// The projected path of one version's payload — the first rung of the
    /// drop lookup, which is where a client reads it.
    pub fn projected(&self, version: &str, option: &str) -> PathBuf {
        let file = match option {
            "src" => source_input(version),
            "win-x64" => format!("tool-{version}-win-x64.zip"),
            other => format!("tool-{version}-{other}.tar.gz"),
        };
        self.drop().join(PACKAGE).join(version).join(file)
    }

    /// A project that can see **only** the mirrored target, with the
    /// projected drop configured and no route to the release host — the
    /// closed network's own view.
    pub fn closed_project(&self) -> PathBuf {
        let project = self.root.path().join("closed");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("vibe.toml"),
            format!(
                "[project]\nname='inside'\nversion='0.1.0'\n\
                 [[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
                self.target().display()
            ),
        )
        .unwrap();
        project
    }

    /// Declare this fixture's drop as the machine's `[artifacts] packages`
    /// drop for `user`.
    pub fn configure_drop(&self, user: &UserScratch) {
        fs::write(
            user.settings.join("config.toml"),
            format!("[artifacts]\npackages = \"{}\"\n", self.drop().display()),
        )
        .unwrap();
    }

    /// `index.json`, the payload files, and a `SHA256SUMS` computed over
    /// them.
    fn write_upstream(&self) {
        let dist = self.dist();
        fs::create_dir_all(&dist).unwrap();
        let records: Vec<String> = VERSIONS
            .iter()
            .enumerate()
            .map(|(position, version)| {
                let lts = if position == 1 { "\"Tin\"" } else { "false" };
                format!("{{\"version\":\"v{version}\",\"lts\":{lts}}}")
            })
            .collect();
        fs::write(
            dist.join("index.json"),
            format!("[\n{}\n]\n", records.join(",\n")),
        )
        .unwrap();
        for version in VERSIONS {
            let release = dist.join(format!("v{version}"));
            fs::create_dir_all(&release).unwrap();
            let mut document = String::new();
            let names = OPTIONS
                .iter()
                .map(|(_, extension, slug)| format!("tool-{version}-{slug}.{extension}"))
                .chain(std::iter::once(source_input(version)));
            for name in names {
                let body = payload(&name);
                fs::write(release.join(&name), &body).unwrap();
                document.push_str(&format!("{:x}  {name}\n", Sha256::digest(&body)));
            }
            fs::write(release.join("SHA256SUMS"), document).unwrap();
        }
    }

    /// The project whose registry layers name the source and the target.
    pub fn project(&self) -> PathBuf {
        let project = self.root.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("vibe.toml"),
            format!(
                "[project]\nname='consumer'\nversion='0.1.0'\n\
                 [[registry]]\nname='src'\ntype='localdir'\nurl='file://{}'\n\
                 [[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
                self.source().display(),
                self.target().display()
            ),
        )
        .unwrap();
        project
    }

    /// A `mirror.toml` selecting `versions`, with `payloads = true` into
    /// this fixture's drop and the given budget and installer ids.
    pub fn config(&self, body: &str) -> PathBuf {
        let path = self.root.path().join("mirror.toml");
        fs::write(&path, body).unwrap();
        path
    }

    /// The `[[template]]` row this suite mostly uses: one version, the
    /// `linux-x64` option, payloads into the drop.
    pub fn one_version(&self, version: &str, budget: Option<&str>, installers: &str) -> PathBuf {
        let budget = budget
            .map(|figure| format!("max_total = \"{figure}\"\n"))
            .unwrap_or_default();
        self.config(&format!(
            "schema = 1\n[[template]]\npackage = '{PACKAGE}'\nversions = ['={version}']\n\
             installers = [{installers}]\npayloads = true\n\
             [payloads]\ninto = '{}'\n{budget}",
            self.drop().display()
        ))
    }
}

/// Deterministic payload bytes of exactly [`PAYLOAD_BYTES`] length, so one
/// declared `size` is honest for every version.
fn payload(name: &str) -> Vec<u8> {
    let mut body = format!("quokkaword payload of {name}\n").into_bytes();
    body.resize(PAYLOAD_BYTES as usize, b'.');
    body
}

pub fn vibe(user: &UserScratch, project: &Path, args: &[&str]) -> Output {
    user.vibe()
        .current_dir(project)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .output()
        .unwrap()
}

pub fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "expected success:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn refusal(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "expected a refusal:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Mirror with `--config`, through the remap row that reads the release
/// host off disk.
pub fn mirror(user: &UserScratch, fixture: &Fixture, config: &Path, extra: &[&str]) -> Output {
    let remap = fixture.root.path().join("remap.toml");
    let project = fixture.project();
    let mut args: Vec<&str> = vec![
        "--remap",
        remap.to_str().unwrap(),
        "--json",
        "registry",
        "mirror",
        "src",
        "--into",
        "dst",
        "--config",
        config.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    vibe(user, &project, &args)
}

/// Every file under `dir`, recursively.
pub fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            found.extend(walk(&entry.path()));
        } else {
            found.push(entry.path());
        }
    }
    found
}
