//! Reviewed npm runtime binding references an existing acquisition input, never a URL.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#npm-runtime");

use super::{InstallerOptionDecl, StepKind, steps_of};
use serde::{Deserialize, Serialize};

/// The `[installer_options.npm_runtime]` table: one exact npm package and
/// version, the step input that carries its bytes, and the Node range it needs.
///
/// ```
/// use vibe_core::manifest::NpmRuntimeDecl;
///
/// let runtime: NpmRuntimeDecl = toml::from_str(
///     "step = 'fetch'\ninput = 'npm'\npackage = '@scope/tool'\nversion = '1.2.0'\nnode_range = '>=22.0.0'",
/// )
/// .unwrap();
/// assert_eq!(runtime.package, "@scope/tool");
/// assert_eq!(runtime.minimum_node().unwrap(), semver::Version::new(22, 0, 0));
/// assert!(runtime.accepts_node(&semver::Version::new(24, 1, 0)).unwrap());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpmRuntimeDecl {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    pub package: String,
    pub version: String,
    pub node_range: String,
}

impl NpmRuntimeDecl {
    /// The first supported npm range subset is exactly `>=major.minor.patch`.
    pub fn minimum_node(&self) -> Result<semver::Version, String> {
        let raw = self
            .node_range
            .strip_prefix(">=")
            .ok_or("node_range must use >=major.minor.patch")?;
        let version =
            semver::Version::parse(raw).map_err(|_| "node_range must use >=major.minor.patch")?;
        if !version.pre.is_empty() || !version.build.is_empty() || version.to_string() != raw {
            return Err("node_range must name a stable exact lower bound".into());
        }
        Ok(version)
    }

    pub fn accepts_node(&self, version: &semver::Version) -> Result<bool, String> {
        Ok(
            version.pre.is_empty()
                && version.build.is_empty()
                && version >= &self.minimum_node()?,
        )
    }

    /// Return only the digest already declared by the addressed source owner.
    pub fn input_digest(&self, option: &InstallerOptionDecl) -> Result<String, String> {
        let steps = steps_of(option);
        let step = steps
            .iter()
            .find(|step| step.id == self.step)
            .ok_or("npm_runtime.step names no existing step")?;
        let digest = match &step.kind {
            StepKind::Sources { sources } => {
                let name = self
                    .input
                    .as_deref()
                    .ok_or("npm_runtime.input is required for a sources step")?;
                let source = sources
                    .iter()
                    .find(|s| s.name == name)
                    .ok_or("npm_runtime.input names no source in the selected step")?;
                &source.sha256
            }
            StepKind::Archive { sha256, .. } if self.input.is_none() => sha256,
            StepKind::Archive { .. } => {
                return Err("npm_runtime.input is not allowed for an archive step".into());
            }
            _ => return Err("npm_runtime.step must name a sources or archive step".into()),
        };
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("npm_runtime requires an existing concrete sha256-bound input".into());
        }
        Ok(digest.clone())
    }

    /// Compare against package/package.json read from those same verified bytes.
    pub fn verify_package_json(&self, bytes: &[u8]) -> Result<(), String> {
        let metadata: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| "npm package metadata is invalid JSON")?;
        if metadata.get("name").and_then(|v| v.as_str()) != Some(self.package.as_str())
            || metadata.get("version").and_then(|v| v.as_str()) != Some(self.version.as_str())
            || metadata.pointer("/engines/node").and_then(|v| v.as_str())
                != Some(self.node_range.as_str())
        {
            return Err(
                "npm runtime binding does not match verified package identity or engines.node"
                    .into(),
            );
        }
        self.minimum_node()?;
        Ok(())
    }

    pub(super) fn validate(&self, option: &InstallerOptionDecl) -> Result<(), String> {
        let valid_piece = |s: &str| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b))
        };
        let name = self.package.strip_prefix('@').unwrap_or(&self.package);
        let parts: Vec<_> = name.split('/').collect();
        let valid_name = if self.package.starts_with('@') {
            parts.len() == 2
        } else {
            parts.len() == 1
        };
        if !valid_name || !parts.iter().all(|p| valid_piece(p)) {
            return Err("npm_runtime.package must be an exact npm package name".into());
        }
        let version = semver::Version::parse(&self.version)
            .map_err(|_| "npm_runtime.version must be an exact stable version")?;
        if !version.pre.is_empty() || !version.build.is_empty() {
            return Err("npm_runtime.version must be stable".into());
        }
        self.minimum_node()?;
        self.input_digest(option)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding(range: &str) -> NpmRuntimeDecl {
        NpmRuntimeDecl {
            step: "fetch".into(),
            input: Some("npm".into()),
            package: "@qwen-code/qwen-code".into(),
            version: "0.28.0".into(),
            node_range: range.into(),
        }
    }
    fn option(binding: &str, steps: &str) -> Result<InstallerOptionDecl, String> {
        let text = format!(
            "[package]\nname='binding'\ngroup='org.example'\nkind='app'\nversion='1.0.0'\n[application]\nid='binding'\n[[installer_options]]\nid='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\n{binding}\n{steps}\n"
        );
        crate::manifest::Manifest::parse_str(&text)
            .map(|m| m.installer_options[0].clone())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn runtime_binding_references_exact_existing_input_and_round_trips() {
        let declared = "[installer_options.npm_runtime]\nstep='fetch'\ninput='npm'\npackage='@qwen-code/qwen-code'\nversion='0.28.0'\nnode_range='>=22.0.0'";
        let sources = format!(
            "[[installer_options.steps]]\nid='fetch'\ntitle='Fetch'\nkind='sources'\n[[installer_options.steps.sources]]\nname='npm'\nurl='https://example.invalid/package.tgz'\nsha256='{}'\n[[installer_options.steps]]\nid='copy'\ntitle='Copy'\nkind='copy'\nfrom='vibe.toml'\nto='manifest'",
            "ab".repeat(32)
        );
        let selected = option(declared, &sources).unwrap();
        let runtime = selected.npm_runtime.as_ref().unwrap();
        assert_eq!(runtime.input_digest(&selected).unwrap(), "ab".repeat(32));
        let encoded = toml::to_string(&selected).unwrap();
        assert_eq!(
            toml::from_str::<InstallerOptionDecl>(&encoded).unwrap(),
            selected
        );
        let legacy = option("", &sources).unwrap();
        assert!(legacy.npm_runtime.is_none());
        assert!(!toml::to_string(&legacy).unwrap().contains("npm_runtime"));
        for bad in [
            declared.replace("step='fetch'", "step='other'"),
            declared.replace("input='npm'", "input='elsewhere'"),
            declared.replace("input='npm'\n", ""),
            declared.replace("node_range='>=22.0.0'", "node_range='lts'"),
        ] {
            assert!(option(&bad, &sources).is_err());
        }
        let archive = format!(
            "[[installer_options.steps]]\nid='fetch'\ntitle='Fetch'\nkind='archive'\nformat='tar_gz'\nsource='package.tgz'\nsha256='{}'",
            "ab".repeat(32)
        );
        assert!(option(declared, &archive).is_err());
        assert!(option(&declared.replace("input='npm'\n", ""), &archive).is_ok());
        assert!(option(declared, &sources.replace(&"ab".repeat(32), "unpinned")).is_err());
    }

    #[test]
    fn runtime_range_is_explicit_stable_subset_and_metadata_is_exact() {
        let b = binding(">=22.0.0");
        for (v, expected) in [
            ("21.9.0", false),
            ("22.0.0", true),
            ("24.1.0", true),
            ("24.0.0-beta.1", false),
        ] {
            assert_eq!(
                b.accepts_node(&semver::Version::parse(v).unwrap()).unwrap(),
                expected
            );
        }
        for invalid in [
            ">=22",
            ">=22.0.0 || >=24.0.0",
            "lts",
            "^22.0.0",
            ">=22.0.0-beta.1",
        ] {
            assert!(binding(invalid).minimum_node().is_err());
        }
        let good =
            br#"{"name":"@qwen-code/qwen-code","version":"0.28.0","engines":{"node":">=22.0.0"}}"#;
        b.verify_package_json(good).unwrap();
        assert!(binding(">=24.0.0").verify_package_json(good).is_err());
        assert!(b.verify_package_json(br#"{"name":"other"}"#).is_err());
    }
}
