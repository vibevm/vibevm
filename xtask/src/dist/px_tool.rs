//! `cargo xtask dist px` — stage the pinned px release as `tools/px/`
//! (`PROP-068#proxy`): fetch (or take) the asset `xtask/dist/px.toml` pins
//! for one target, verify its SHA-256, unpack it, and add px's MIT LICENSE
//! and a NOTICE line. px is never committed; this runs at distribution
//! build time.
//!
//! The licence is pinned and staged with LF line ends, because upstream
//! ships the same text LF in the repository and CRLF in the Windows zip
//! (see [`lf_line_ends`]).

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use vibe_publish::release_manifest::{CompanionAsset, CompanionTool};

/// The manifest's location under the repository root.
pub(crate) const MANIFEST: &str = "xtask/dist/px.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PxManifest {
    pub(crate) schema: u32,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) homepage: String,
    pub(crate) license: String,
    pub(crate) license_url: String,
    pub(crate) license_sha256: String,
    pub(crate) notice: String,
    #[serde(rename = "asset")]
    pub(crate) assets: Vec<PxAsset>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PxAsset {
    pub(crate) target: String,
    pub(crate) url: String,
    pub(crate) sha256: String,
    pub(crate) archive: String,
    pub(crate) executable: String,
}

impl PxManifest {
    pub(crate) fn parse(text: &str) -> Result<Self> {
        let manifest: Self = toml::from_str(text).context("parsing xtask/dist/px.toml")?;
        if manifest.schema != 1 {
            bail!("xtask/dist/px.toml schema {} is not 1", manifest.schema);
        }
        for asset in &manifest.assets {
            if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                bail!("px asset for `{}` has no SHA-256 pin", asset.target);
            }
            if !matches!(asset.archive.as_str(), "zip" | "tar.gz") {
                bail!(
                    "px asset for `{}`: unknown archive `{}`",
                    asset.target,
                    asset.archive
                );
            }
        }
        Ok(manifest)
    }

    pub(crate) fn asset(&self, target: &str) -> Result<&PxAsset> {
        self.assets.iter().find(|asset| asset.target == target).with_context(|| {
            format!(
                "px {} has no asset for `{target}` (pinned: {}); a distribution for this target \
                 carries no px — its users install px themselves (`pip install px-proxy`) and \
                 set `[proxy] px_path`",
                self.version,
                self.assets
                    .iter()
                    .map(|asset| asset.target.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
}

pub(crate) struct Inputs<'a> {
    pub(crate) target: &'a str,
    pub(crate) out_dir: &'a Path,
    /// A local copy of the release asset (no download).
    pub(crate) archive: Option<&'a Path>,
    /// A local copy of the LICENSE (no download; else the archive's own).
    pub(crate) license: Option<&'a Path>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The licence text with every CRLF folded to a bare LF.
///
/// Upstream publishes one licence in two spellings: the repository's LF
/// form, which `license_url` serves, and the CRLF form the Windows zip
/// carries. `license_sha256` pins the LF form, so this is both the form the
/// digest is measured on **and** the form written into `tools/px/`: the
/// licence vibe ships is then byte for byte the pinned text whichever copy
/// the build read, and its digest is exactly the pin. Only the two bytes of
/// a line end are folded — a single changed, added or removed character of
/// licence text still changes the digest and is still refused.
fn lf_line_ends(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some(index) = rest.iter().position(|byte| *byte == b'\r') {
        if rest.get(index + 1) == Some(&b'\n') {
            out.extend_from_slice(&rest[..index]);
        } else {
            out.extend_from_slice(&rest[..=index]);
        }
        rest = &rest[index + 1..];
    }
    out.extend_from_slice(rest);
    out
}

fn fetch(url: &str) -> Result<Vec<u8>> {
    let client = vibe_net::http::route(
        reqwest::blocking::Client::builder()
            .user_agent(concat!("vibevm-xtask/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(600)),
        vibe_net::http::Ambient::Inherit,
    )?
    .build()?;
    let response = client
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .with_context(|| format!("downloading `{url}`"))?;
    Ok(response.bytes()?.to_vec())
}

/// A relative, normal archive path; `./` prefixes dropped. Anything that
/// could escape the destination is refused.
fn contained(name: &str) -> Result<Option<PathBuf>> {
    let mut out = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => bail!("px archive entry `{name}` escapes its directory"),
        }
    }
    Ok((!out.as_os_str().is_empty()).then_some(out))
}

fn unpack(asset: &PxAsset, bytes: &[u8], dest: &Path) -> Result<()> {
    match asset.archive.as_str() {
        "zip" => {
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index)?;
                let Some(relative) = contained(entry.name())? else {
                    continue;
                };
                let path = dest.join(relative);
                if entry.is_dir() {
                    std::fs::create_dir_all(&path)?;
                    continue;
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                std::fs::write(&path, data)?;
            }
        }
        _ => {
            let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
            for entry in archive.entries()? {
                let mut entry = entry?;
                let name = entry.path()?.to_string_lossy().into_owned();
                let Some(relative) = contained(&name)? else {
                    continue;
                };
                let kind = entry.header().entry_type();
                if !(kind.is_file() || kind.is_dir()) {
                    bail!("px archive entry `{name}` is not a file or directory");
                }
                entry.unpack(dest.join(relative))?;
            }
        }
    }
    Ok(())
}

/// Stage `tools/px/` for `target` under `out_dir`; returns the staged
/// directory and the verified upstream archive bytes.
pub(crate) fn stage_with_archive(
    manifest: &PxManifest,
    inputs: &Inputs<'_>,
) -> Result<(PathBuf, Vec<u8>)> {
    let asset = manifest.asset(inputs.target)?;
    let bytes = match inputs.archive {
        Some(path) => {
            std::fs::read(path).with_context(|| format!("reading `{}`", path.display()))?
        }
        None => fetch(&asset.url)?,
    };
    let digest = sha256_hex(&bytes);
    if digest != asset.sha256 {
        bail!(
            "px asset for `{}` has SHA-256 {digest}, xtask/dist/px.toml pins {} — refusing it",
            asset.target,
            asset.sha256
        );
    }
    let dest = inputs.out_dir.join("tools").join("px");
    if dest.exists() {
        std::fs::remove_dir_all(&dest).with_context(|| format!("clearing `{}`", dest.display()))?;
    }
    std::fs::create_dir_all(&dest)?;
    unpack(asset, &bytes, &dest)?;
    let executable = dest.join(&asset.executable);
    if !executable.is_file() {
        bail!(
            "px asset for `{}` holds no `{}`",
            asset.target,
            asset.executable
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))?;
    }
    let license = lf_line_ends(&match inputs.license {
        Some(path) => std::fs::read(path)?,
        None if dest.join("LICENSE.txt").is_file() => std::fs::read(dest.join("LICENSE.txt"))?,
        None => fetch(&manifest.license_url)?,
    });
    let license_digest = sha256_hex(&license);
    if license_digest != manifest.license_sha256 {
        bail!(
            "px LICENSE has SHA-256 {license_digest} with LF line ends, xtask/dist/px.toml pins \
             {} — refusing it",
            manifest.license_sha256
        );
    }
    std::fs::write(dest.join("LICENSE.txt"), &license)?;
    std::fs::write(dest.join("NOTICE"), format!("{}\n", manifest.notice))?;
    Ok((dest, bytes))
}

/// The companion release asset of px for `asset`
/// (`px-<version>-<target>.<zip|tar.gz>`, PROP-019 `#companion-tools`): the
/// pinned upstream archive byte for byte, so its digest is the pin vibe
/// embeds. The same naming as `vibe_net::px_store::PxPin::release_asset_name`.
pub(crate) fn asset_name(version: &str, asset: &PxAsset) -> String {
    format!("px-{version}-{}.{}", asset.target, asset.archive)
}

/// The per-target listing the release assembly folds into `TOOLS.json`.
pub(crate) fn fragment_name(version: &str, target: &str) -> String {
    format!("px-{version}-{target}.tool.json")
}

/// Write the verified upstream `archive` of `target` as the companion
/// asset and its listing into `out_dir`. Returns the listing.
pub(crate) fn package(
    manifest: &PxManifest,
    archive: &[u8],
    target: &str,
    out_dir: &Path,
) -> Result<CompanionTool> {
    let asset = manifest.asset(target)?;
    let digest = sha256_hex(archive);
    if digest != asset.sha256 {
        bail!("px archive for `{target}` is not the pinned one ({digest})");
    }
    let name = asset_name(&manifest.version, asset);
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join(&name), archive)?;
    let tool = CompanionTool {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        target: target.to_owned(),
        executable: asset.executable.clone(),
        asset: CompanionAsset {
            name,
            size: archive.len() as u64,
            digest: format!("sha256:{digest}"),
        },
    };
    std::fs::write(
        out_dir.join(fragment_name(&manifest.version, target)),
        serde_json::to_vec_pretty(&tool)?,
    )?;
    Ok(tool)
}

pub(crate) fn read_manifest(repo_root: &Path) -> Result<PxManifest> {
    let text = std::fs::read_to_string(repo_root.join(MANIFEST))
        .with_context(|| format!("reading {MANIFEST}"))?;
    PxManifest::parse(&text)
}

pub(crate) fn run(repo_root: &Path, inputs: &Inputs<'_>) -> Result<()> {
    let manifest = read_manifest(repo_root)?;
    let (dest, archive) = stage_with_archive(&manifest, inputs)?;
    let tool = package(&manifest, &archive, inputs.target, inputs.out_dir)?;
    println!(
        "staged {} {} ({}, {}) for {} at {}; companion asset `{}` ({} bytes, {})",
        manifest.name,
        manifest.version,
        manifest.license,
        manifest.homepage,
        inputs.target,
        dest.display(),
        tool.asset.name,
        tool.asset.size,
        tool.asset.digest
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut out);
        for (name, data) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, data).unwrap();
        }
        writer.finish().unwrap();
        out.into_inner()
    }

    fn manifest(sha: &str, license_sha: &str) -> PxManifest {
        PxManifest::parse(&format!(
            "schema = 1\nname = 'px'\nversion = '0.0.1'\nhomepage = 'https://example.invalid'\n\
             license = 'MIT'\nlicense_url = 'https://example.invalid/LICENSE.txt'\n\
             license_sha256 = '{license_sha}'\nnotice = 'px test notice'\n\
             [[asset]]\ntarget = 'x86_64-pc-windows-msvc'\nurl = 'https://example.invalid/px.zip'\n\
             sha256 = '{sha}'\narchive = 'zip'\nexecutable = 'px.exe'\n"
        ))
        .unwrap()
    }

    #[test]
    fn the_pinned_manifest_parses_and_covers_the_bundled_targets() {
        let text =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/px.toml"))
                .unwrap();
        let manifest = PxManifest::parse(&text).unwrap();
        for target in [
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "x86_64-unknown-linux-musl",
            "aarch64-apple-darwin",
        ] {
            assert!(manifest.asset(target).is_ok(), "{target}");
        }
        let missing = manifest
            .asset("x86_64-apple-darwin")
            .unwrap_err()
            .to_string();
        assert!(missing.contains("px_path"), "{missing}");
    }

    #[test]
    fn a_verified_asset_is_staged_with_license_and_notice_and_a_wrong_digest_is_refused() {
        let license = b"MIT License\n";
        let archive = zip_bytes(&[
            ("px.exe", b"MZ fake"),
            ("LICENSE.txt", license),
            ("Lib/x.py", b""),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("px.zip");
        std::fs::write(&archive_path, &archive).unwrap();
        let good = manifest(&sha256_hex(&archive), &sha256_hex(license));
        let inputs = Inputs {
            target: "x86_64-pc-windows-msvc",
            out_dir: dir.path(),
            archive: Some(&archive_path),
            license: None,
        };
        let (staged, _) = stage_with_archive(&good, &inputs).unwrap();
        assert_eq!(std::fs::read(staged.join("px.exe")).unwrap(), b"MZ fake");
        assert_eq!(std::fs::read(staged.join("LICENSE.txt")).unwrap(), license);
        assert_eq!(
            std::fs::read_to_string(staged.join("NOTICE")).unwrap(),
            "px test notice\n"
        );
        assert!(staged.join("Lib/x.py").is_file());

        let wrong = manifest(&"0".repeat(64), &sha256_hex(license));
        let error = stage_with_archive(&wrong, &inputs).unwrap_err().to_string();
        assert!(error.contains("refusing"), "{error}");

        let escaping = zip_bytes(&[("../evil", b"x")]);
        std::fs::write(&archive_path, &escaping).unwrap();
        let evil = manifest(&sha256_hex(&escaping), &sha256_hex(license));
        assert!(
            stage_with_archive(&evil, &inputs)
                .unwrap_err()
                .to_string()
                .contains("escapes")
        );
    }

    #[test]
    fn the_crlf_spelling_of_the_pinned_licence_is_staged_as_the_pinned_lf_text() {
        let lf: &[u8] = b"MIT License\n\nCopyright (c) 2017-2026 Someone\n";
        let crlf: &[u8] = b"MIT License\r\n\r\nCopyright (c) 2017-2026 Someone\r\n";
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("px.zip");

        // The Windows zip's own CRLF copy of the pinned text is accepted,
        // and `tools/px/LICENSE.txt` is the pinned LF form byte for byte.
        let archive = zip_bytes(&[("px.exe", b"MZ fake"), ("LICENSE.txt", crlf)]);
        std::fs::write(&archive_path, &archive).unwrap();
        let inputs = Inputs {
            target: "x86_64-pc-windows-msvc",
            out_dir: dir.path(),
            archive: Some(&archive_path),
            license: None,
        };
        let pinned = manifest(&sha256_hex(&archive), &sha256_hex(lf));
        let (staged, _) = stage_with_archive(&pinned, &inputs).unwrap();
        assert_eq!(std::fs::read(staged.join("LICENSE.txt")).unwrap(), lf);
        assert_eq!(
            sha256_hex(&std::fs::read(staged.join("LICENSE.txt")).unwrap()),
            pinned.license_sha256,
            "the staged licence must hash to the pin"
        );

        // A single byte of licence *content* changed — `2026` to `2027`, the
        // length untouched — is still refused, in the CRLF and the LF
        // spelling alike.
        for tampered in [
            b"MIT License\r\n\r\nCopyright (c) 2017-2027 Someone\r\n".as_slice(),
            b"MIT License\n\nCopyright (c) 2017-2027 Someone\n".as_slice(),
        ] {
            let archive = zip_bytes(&[("px.exe", b"MZ fake"), ("LICENSE.txt", tampered)]);
            std::fs::write(&archive_path, &archive).unwrap();
            let still_pinned = manifest(&sha256_hex(&archive), &sha256_hex(lf));
            let error = stage_with_archive(&still_pinned, &inputs)
                .unwrap_err()
                .to_string();
            assert!(error.contains("px LICENSE has SHA-256"), "{error}");
            assert!(error.contains("refusing it"), "{error}");
        }
    }

    #[test]
    fn folding_line_ends_touches_nothing_but_a_crlf_pair() {
        assert_eq!(lf_line_ends(b"a\r\nb\n"), b"a\nb\n");
        assert_eq!(lf_line_ends(b"a\r\n\r\nb"), b"a\n\nb");
        // A lone CR is content, not a line end, and survives.
        assert_eq!(lf_line_ends(b"a\rb"), b"a\rb");
        assert_eq!(lf_line_ends(b"a\r\r\nb"), b"a\r\nb");
        assert_eq!(lf_line_ends(b"trailing\r"), b"trailing\r");
        assert_eq!(lf_line_ends(b""), b"");
    }

    #[test]
    fn the_companion_asset_is_the_pinned_archive_with_its_listing() {
        let root = tempfile::tempdir().unwrap();
        let archive = zip_bytes(&[("px.exe", b"px".as_slice())]);
        let manifest = manifest(&sha256_hex(&archive), &"0".repeat(64));
        let out = root.path().join("out");
        let tool = package(&manifest, &archive, "x86_64-pc-windows-msvc", &out).unwrap();
        assert_eq!(tool.asset.name, "px-0.0.1-x86_64-pc-windows-msvc.zip");
        assert_eq!(std::fs::read(out.join(&tool.asset.name)).unwrap(), archive);
        assert_eq!(
            tool.asset.digest,
            format!("sha256:{}", sha256_hex(&archive))
        );
        let listed: CompanionTool = serde_json::from_slice(
            &std::fs::read(out.join(fragment_name("0.0.1", "x86_64-pc-windows-msvc"))).unwrap(),
        )
        .unwrap();
        assert_eq!(listed, tool);
        // Another archive than the pinned one is refused.
        assert!(package(&manifest, b"other", "x86_64-pc-windows-msvc", &out).is_err());
    }

    #[test]
    fn the_companion_name_and_digest_are_the_ones_vibe_embeds() {
        let pin = vibe_net::px_store::PxPin::embedded();
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/px.toml"),
        )
        .unwrap();
        let manifest = PxManifest::parse(&text).unwrap();
        for asset in &manifest.assets {
            let embedded = pin.asset(&asset.target).unwrap();
            assert_eq!(
                asset_name(&manifest.version, asset),
                pin.release_asset_name(embedded)
            );
            assert_eq!(asset.sha256, embedded.sha256);
        }
    }
}
