//! The pinned px and its versioned store (PROP-019 `#companion-tools`,
//! PROP-068 `#proxy-px`).
//!
//! `xtask/dist/px.toml` is the one pin: it is embedded in every vibe build,
//! so a release and a source build alike know the px version, the asset of
//! their target and its SHA-256. The release companion asset
//! `px-<version>-<target>.<zip|tar.gz>` is the pinned upstream archive
//! byte for byte. px is installed once per px version into the store
//! `<install root>/opt/vibevm/tools/px/<version>/`, shared by every vibe
//! instance of that installation; the px lookup searches it after
//! `px_path` and the files beside the running binary, and before PATH.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools");

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The pin, as committed (`xtask/dist/px.toml`).
const PINNED: &str = include_str!("../../../xtask/dist/px.toml");

/// The pinned px release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PxPin {
    pub schema: u32,
    pub name: String,
    pub version: String,
    pub homepage: String,
    pub license: String,
    pub license_url: String,
    pub license_sha256: String,
    pub notice: String,
    pub assets: Vec<PinnedAsset>,
}

/// One target's pinned upstream archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedAsset {
    pub target: String,
    pub url: String,
    /// The digest the pin file records for the upstream archive, or empty.
    /// Information only — nothing vibe installs is refused over it.
    pub sha256: String,
    pub archive: String,
    pub executable: String,
}

impl PxPin {
    /// Parse a pin; `None` when it is not one (schema 1, each asset
    /// naming its target, url, archive kind and executable).
    ///
    /// `sha256` is read when the file carries it and is pure
    /// information: vibe does not check px against it, here or anywhere
    /// (owner decision 2026-11-09, `PROP-019#companion-tools`), so a pin
    /// that drops the digests is still a pin.
    pub fn parse(text: &str) -> Option<Self> {
        let table: toml::Table = toml::from_str(text).ok()?;
        let text = |table: &toml::Table, key: &str| -> Option<String> {
            table.get(key)?.as_str().map(str::to_owned)
        };
        let mut assets = Vec::new();
        for asset in table.get("asset")?.as_array()? {
            let asset = asset.as_table()?;
            assets.push(PinnedAsset {
                target: text(asset, "target")?,
                url: text(asset, "url")?,
                sha256: text(asset, "sha256").unwrap_or_default(),
                archive: text(asset, "archive")?,
                executable: text(asset, "executable")?,
            });
        }
        let pin = Self {
            schema: u32::try_from(table.get("schema")?.as_integer()?).ok()?,
            name: text(&table, "name")?,
            version: text(&table, "version")?,
            homepage: text(&table, "homepage")?,
            license: text(&table, "license")?,
            license_url: text(&table, "license_url")?,
            license_sha256: text(&table, "license_sha256").unwrap_or_default(),
            notice: text(&table, "notice")?,
            assets,
        };
        (pin.schema == 1
            && pin
                .assets
                .iter()
                .all(|asset| matches!(asset.archive.as_str(), "zip" | "tar.gz")))
        .then_some(pin)
    }

    /// The pin this vibe was built with.
    ///
    /// ```
    /// let pin = vibe_net::px_store::PxPin::embedded();
    /// assert_eq!(pin.name, "px");
    /// assert!(pin.asset("x86_64-pc-windows-msvc").is_some());
    /// ```
    pub fn embedded() -> &'static PxPin {
        static PIN: OnceLock<PxPin> = OnceLock::new();
        PIN.get_or_init(|| {
            // The committed file is checked by this crate's tests; an
            // unparsable pin only means "no pinned px".
            PxPin::parse(PINNED).unwrap_or(PxPin {
                schema: 1,
                name: "px".into(),
                version: "unpinned".into(),
                homepage: String::new(),
                license: String::new(),
                license_url: String::new(),
                license_sha256: String::new(),
                notice: String::new(),
                assets: Vec::new(),
            })
        })
    }

    pub fn asset(&self, target: &str) -> Option<&PinnedAsset> {
        self.assets.iter().find(|asset| asset.target == target)
    }

    /// The release companion asset name of `asset`
    /// (`px-0.11.0-x86_64-pc-windows-msvc.zip`).
    pub fn release_asset_name(&self, asset: &PinnedAsset) -> String {
        format!("px-{}-{}.{}", self.version, asset.target, asset.archive)
    }
}

/// The distribution target of this build, as release assets name it.
pub fn running_target() -> Option<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "windows") => Some("x86_64-pc-windows-msvc"),
        ("x86_64", "linux") if cfg!(target_env = "musl") => Some("x86_64-unknown-linux-musl"),
        ("x86_64", "linux") => Some("x86_64-unknown-linux-gnu"),
        ("x86_64", "macos") => Some("x86_64-apple-darwin"),
        ("aarch64", "macos") => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

static STORE: OnceLock<PathBuf> = OnceLock::new();

/// Name the store root (`<install root>/opt/vibevm/tools/px`) once, at the
/// composition root. Later calls are ignored.
pub fn register_store(root: PathBuf) {
    let _ = STORE.set(root);
}

/// The registered store root.
pub fn store_root() -> Option<&'static Path> {
    STORE.get().map(PathBuf::as_path)
}

/// The directory the px of `pin`'s version occupies in `root`.
pub fn stored_dir(root: &Path, pin: &PxPin) -> PathBuf {
    root.join(&pin.version)
}

/// The store directory of a px whose version nothing names — a bare
/// executable, or an archive not spelled `px-<version>-<target>.<ext>`.
pub const CUSTOM_VERSION: &str = "custom";

/// The provenance record vibe writes beside every px it installs.
pub const RECORD_FILE: &str = "PX.toml";

/// One px in the store, with what vibe knows about where it came from.
///
/// px is the operator's choice and vibe does not pin its content (owner
/// decision 2026-11-09, `PROP-019#companion-tools`), so the record is
/// provenance, never a gate: the file vibe read, the SHA-256 it measured
/// while reading it, when that was, and what the program says its version
/// is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPx {
    /// The px executable, absolute.
    pub program: PathBuf,
    /// The store directory holding it; its name is the version.
    pub directory: PathBuf,
    /// The file or directory vibe read, as it was named.
    pub source: Option<String>,
    /// The SHA-256 vibe measured of those bytes. Information only.
    pub sha256: Option<String>,
    /// When it was installed (RFC 3339).
    pub installed_at: Option<String>,
    /// `px --version`, when the program answered.
    pub version: Option<String>,
}

/// The record's text, for the writer that installs px.
pub fn record_text(
    executable: &str,
    source: &str,
    sha256: &str,
    installed_at: &str,
    version: Option<&str>,
) -> String {
    let quote = |value: &str| value.replace('\\', "\\\\").replace('"', "\\\"");
    let mut out = String::from("# Written by vibe; px is not checked against any digest.\n");
    out.push_str("schema = 1\n");
    out.push_str(&format!("executable = \"{}\"\n", quote(executable)));
    out.push_str(&format!("source = \"{}\"\n", quote(source)));
    out.push_str(&format!("sha256 = \"{}\"\n", quote(sha256)));
    out.push_str(&format!("installed_at = \"{}\"\n", quote(installed_at)));
    if let Some(version) = version {
        out.push_str(&format!("version = \"{}\"\n", quote(version)));
    }
    out
}

/// The px in one store directory: its record when vibe wrote one, else the
/// executable found by name (a store filled by an older vibe).
pub fn stored_in(directory: &Path) -> Option<InstalledPx> {
    let record = std::fs::read_to_string(directory.join(RECORD_FILE))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok());
    let field = |key: &str| -> Option<String> {
        record
            .as_ref()?
            .get(key)?
            .as_str()
            .map(str::to_owned)
            .filter(|value| !value.is_empty())
    };
    let named = field("executable").map(|relative| directory.join(relative));
    let program = named
        .into_iter()
        .chain([
            directory.join(crate::PX_EXE),
            directory.join("px"),
            directory.join("px.exe"),
        ])
        .find(|path| path.is_file())?;
    Some(InstalledPx {
        program,
        directory: directory.to_path_buf(),
        source: field("source"),
        sha256: field("sha256"),
        installed_at: field("installed_at"),
        version: field("version"),
    })
}

/// Every px in `root`, the pinned version's first, then `custom`, then the
/// rest newest-looking first — so the order a lookup takes is the same on
/// every machine.
pub fn stored_all(root: &Path) -> Vec<InstalledPx> {
    let pin = PxPin::embedded();
    let mut names: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names.reverse();
    let rank = |name: &String| match name.as_str() {
        version if version == pin.version => 0,
        CUSTOM_VERSION => 1,
        _ => 2,
    };
    names.sort_by_key(rank);
    names
        .iter()
        .filter_map(|name| stored_in(&root.join(name)))
        .collect()
}

/// The px the store offers, if it holds one.
pub fn stored(root: &Path) -> Option<InstalledPx> {
    stored_all(root).into_iter().next()
}

/// What the store records about the px at `program` — whichever version
/// directory holds it, and however deep inside. `None` for a px the store
/// did not install (`px_path`, PATH, `tools/px/` beside the binary).
pub fn stored_record(root: &Path, program: &Path) -> Option<InstalledPx> {
    stored_all(root)
        .into_iter()
        .find(|found| found.program == program)
}

/// Where the px executable of the registered store lives, if it holds one.
pub fn stored_executable() -> Option<PathBuf> {
    stored(store_root()?).map(|installed| installed.program)
}

/// What a missing px is told to the user: the store directory, the asset
/// file name a drop or release must carry for this target, and the repair.
pub fn missing_hint() -> String {
    let pin = PxPin::embedded();
    let target = running_target().unwrap_or("this platform");
    let asset = running_target()
        .and_then(|target| pin.asset(target))
        .map(|asset| pin.release_asset_name(asset));
    let store = store_root()
        .map(|root| stored_dir(root, pin).display().to_string())
        .unwrap_or_else(|| "the vibe data directory".to_owned());
    match asset {
        Some(asset) => format!(
            "run `vibe self px install` to put px {} into `{store}` — it reads `{asset}` from \
             the release source or from the release drop (`[artifacts] vibe`), or \
             `vibe self px install --from <archive|directory|executable>` to install a px of \
             your own",
            pin.version
        ),
        None => format!(
            "px has no default build for `{target}`; install one of your own with \
             `vibe self px install --from <archive|directory|executable>`"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
    fn the_embedded_pin_is_the_committed_one_and_names_each_asset() {
        let pin = PxPin::embedded();
        assert_eq!(PxPin::parse(PINNED).as_ref(), Some(pin));
        assert_ne!(pin.version, "unpinned");
        let windows = pin.asset("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(windows.executable, "px.exe");
        assert_eq!(
            pin.release_asset_name(windows),
            format!("px-{}-x86_64-pc-windows-msvc.zip", pin.version)
        );
        let linux = pin.asset("x86_64-unknown-linux-gnu").unwrap();
        assert!(pin.release_asset_name(linux).ends_with(".tar.gz"));
        assert!(PxPin::parse("schema = 2").is_none());
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
    fn the_px_lookup_finds_the_store_copy_and_a_miss_names_the_store_and_the_asset() {
        let root = tempfile::tempdir().unwrap();
        register_store(root.path().join("tools/px"));
        let config: vibe_core::user_config::ProxyConfig =
            toml::from_str("mode = 'px'\nupstream = 'proxy.example.invalid:8080'").unwrap();
        let route = vibe_core::net::ProxyRoute::from_config(&config).unwrap();
        let settings = route.px().unwrap().clone();
        let pin = PxPin::embedded();
        let Some(target) = running_target() else {
            // No default asset for this platform: nothing to name.
            return;
        };
        // The store is empty, so the miss names the store, the default
        // asset and the repair.
        assert_eq!(stored_executable(), None);
        if crate::px::locate(&settings).is_ok() {
            // A px on PATH or beside the test binary wins; not this
            // test's case.
            return;
        }
        let missing = crate::px::locate(&settings).unwrap_err().to_string();
        assert!(missing.contains("vibe self px install"), "{missing}");
        if let Some(asset) = pin.asset(target) {
            assert!(
                missing.contains(&pin.release_asset_name(asset)),
                "{missing}"
            );
            assert!(
                missing.contains(&stored_dir(store_root().unwrap(), pin).display().to_string()),
                "{missing}"
            );
        }

        // A px in the store is then what the lookup finds — whichever
        // version directory holds it, and whatever its name inside
        // (`PROP-019#companion-tools`).
        let directory = stored_dir(store_root().unwrap(), pin);
        let program = directory.join(crate::PX_EXE);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(&program, b"px").unwrap();
        assert_eq!(stored_executable(), Some(program.clone()));
        assert_eq!(crate::px::locate(&settings).unwrap(), program);

        // A px of the operator's own, recorded with its provenance, is
        // found the same way — the record names the executable inside the
        // directory, and nothing says anything about a digest.
        std::fs::remove_dir_all(&directory).unwrap();
        let custom = store_root().unwrap().join(CUSTOM_VERSION);
        std::fs::create_dir_all(custom.join("bin")).unwrap();
        let mine = custom.join("bin").join(crate::PX_EXE);
        std::fs::write(&mine, b"my px").unwrap();
        std::fs::write(
            custom.join(RECORD_FILE),
            record_text(
                &format!("bin/{}", crate::PX_EXE),
                "/home/me/px.zip",
                "abc123",
                "2026-11-09T00:00:00Z",
                Some("px 9.9.9"),
            ),
        )
        .unwrap();
        let found = stored(store_root().unwrap()).expect("the store offers px");
        assert_eq!(found.program, mine);
        assert_eq!(found.source.as_deref(), Some("/home/me/px.zip"));
        assert_eq!(found.sha256.as_deref(), Some("abc123"));
        assert_eq!(found.version.as_deref(), Some("px 9.9.9"));
        assert_eq!(crate::px::locate(&settings).unwrap(), mine);
    }
}
