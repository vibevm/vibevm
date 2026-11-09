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
    pub sha256: String,
    pub archive: String,
    pub executable: String,
}

impl PxPin {
    /// Parse a pin; `None` when it is not one (schema 1, digests present).
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
                sha256: text(asset, "sha256")?,
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
            license_sha256: text(&table, "license_sha256")?,
            notice: text(&table, "notice")?,
            assets,
        };
        (pin.schema == 1
            && pin.assets.iter().all(|asset| {
                asset.sha256.len() == 64
                    && asset.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                    && matches!(asset.archive.as_str(), "zip" | "tar.gz")
            }))
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

/// The directory the pinned px for `asset` occupies in `root`.
pub fn stored_dir(root: &Path, pin: &PxPin) -> PathBuf {
    root.join(&pin.version)
}

/// Where the pinned px executable for this build lives in the store, if
/// both the store and the pin for this target are known.
pub fn stored_executable() -> Option<PathBuf> {
    let pin = PxPin::embedded();
    let asset = pin.asset(running_target()?)?;
    Some(stored_dir(store_root()?, pin).join(&asset.executable))
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
             the release source or from the release drop (`[artifacts] vibe`)",
            pin.version
        ),
        None => format!("px {} has no release asset for `{target}`", pin.version),
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
        let Some(program) = stored_executable() else {
            // No pinned px for this platform: nothing to find.
            return;
        };
        if crate::px::locate(&settings).is_ok_and(|found| found != program) {
            // A px on PATH or beside the test binary wins; not this test's case.
            return;
        }
        let missing = crate::px::locate(&settings).unwrap_err().to_string();
        let pin = PxPin::embedded();
        let asset = pin.asset(running_target().unwrap()).unwrap();
        assert!(missing.contains("vibe self px install"), "{missing}");
        assert!(
            missing.contains(&pin.release_asset_name(asset)),
            "{missing}"
        );
        assert!(
            missing.contains(&stored_dir(store_root().unwrap(), pin).display().to_string()),
            "{missing}"
        );
        std::fs::create_dir_all(program.parent().unwrap()).unwrap();
        std::fs::write(&program, b"px").unwrap();
        assert_eq!(crate::px::locate(&settings).unwrap(), program);
    }
}
