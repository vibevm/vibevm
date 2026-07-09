//! Provider profiles (plan D6): `<fractality-home>/profiles.toml`.
//!
//! Mission-control loads profiles for validation and limits; pods load
//! them to construct worker environments. `token_file` is a **path
//! reference** — this module never reads token contents (secrets
//! hygiene: only the pod reads the file, at spawn time, and never
//! echoes it).

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The whole profiles file, `schema = 1`.
///
/// ```
/// use fractality_core::profile::ProfilesFile;
///
/// let profiles = ProfilesFile::from_toml_str(
///     "schema = 1\n[profile.glm]\nbackend = \"claude-code\"\nbase_url = \"http://gw\"\ntoken_file = \"t\"\n[profile.glm.models]\nbig = \"m1\"\nsmall = \"m2\"\nhaiku_slot = \"m2\"\n",
/// )
/// .expect("parses");
/// assert_eq!(profiles.get("glm").expect("glm").models.big, "m1");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilesFile {
    /// File schema; this build writes and reads `1`.
    pub schema: u32,
    /// Profiles by name (`[profile.<name>]` tables).
    pub profile: BTreeMap<String, Profile>,
}

/// One provider profile (D6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Worker backend id; this build knows `claude-code`.
    pub backend: String,
    /// Anthropic-compatible gateway base URL.
    pub base_url: String,
    /// Path to the bearer token — by reference, never inlined.
    pub token_file: Utf8PathBuf,
    /// Worker binary name or absolute path.
    #[serde(default = "default_claude_binary")]
    pub claude_binary: String,
    /// `auto` = a fresh per-run CLAUDE_CONFIG_DIR inside the run dir
    /// (F4/R5: onboards headless); anything else is used verbatim.
    #[serde(default = "default_config_dir")]
    pub config_dir: String,
    pub models: Models,
    /// Extra provider env, injected verbatim (D5 layer 3).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub pricing: Pricing,
}

/// The model slots a packet's `routing.model` resolves through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Models {
    /// The `big` routing slot (also the worker's opus/sonnet mapping).
    pub big: String,
    /// The `small` routing slot.
    pub small: String,
    /// CC-internal small-model traffic (the haiku env slot).
    pub haiku_slot: String,
}

/// Concurrency limits (admission control lands in Phase 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_concurrent: default_max_concurrent(),
        }
    }
}

/// Worker permission posture (RP4: no yolo; D18 is the way of life).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Permissions {
    /// Exact Claude Code `--permission-mode` value.
    #[serde(default = "default_permission_mode")]
    pub mode: String,
    /// Tools denied outright (tariff hygiene, D12).
    #[serde(default)]
    pub deny_tools: Vec<String>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            mode: default_permission_mode(),
            deny_tools: Vec::new(),
        }
    }
}

/// Pricing metadata — metrics only, never enforcement (D6/D12).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pricing {
    /// True for flat-rate subscription plans.
    #[serde(default)]
    pub flat: bool,
    /// Plan name feeding the quota metric (e.g. `max`).
    #[serde(default)]
    pub plan: Option<String>,
}

fn default_claude_binary() -> String {
    "claude".to_owned()
}
fn default_config_dir() -> String {
    "auto".to_owned()
}
fn default_max_concurrent() -> u32 {
    4
}
fn default_permission_mode() -> String {
    "acceptEdits".to_owned()
}

impl ProfilesFile {
    /// File name inside the fractality home.
    pub const FILE_NAME: &'static str = "profiles.toml";

    /// Parses and validates the authored TOML form.
    pub fn from_toml_str(text: &str) -> Result<Self, CoreError> {
        let file: ProfilesFile = toml::from_str(text)?;
        file.validate()?;
        Ok(file)
    }

    /// Structural validation: schema pin and non-empty anchors.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema != 1 {
            return Err(CoreError::ProfilesSchema { found: self.schema });
        }
        for (name, profile) in &self.profile {
            let non_empty: [(&'static str, &str, &'static str); 5] = [
                (
                    "backend",
                    &profile.backend,
                    "name a worker backend (claude-code)",
                ),
                (
                    "base_url",
                    &profile.base_url,
                    "the provider's Anthropic-compatible gateway URL",
                ),
                (
                    "models.big",
                    &profile.models.big,
                    "the big routing slot's model id",
                ),
                (
                    "models.small",
                    &profile.models.small,
                    "the small routing slot's model id",
                ),
                (
                    "models.haiku_slot",
                    &profile.models.haiku_slot,
                    "the model for CC-internal small-model traffic",
                ),
            ];
            for (field, value, hint) in non_empty {
                if value.trim().is_empty() {
                    return Err(CoreError::ProfileField {
                        profile: name.clone(),
                        field,
                        hint,
                    });
                }
            }
        }
        Ok(())
    }

    /// Loads `<home>/profiles.toml`.
    pub fn load(home: &Utf8Path) -> Result<Self, CoreError> {
        let path = home.join(Self::FILE_NAME);
        let text = match std::fs::read_to_string(path.as_std_path()) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CoreError::ProfilesMissing { path });
            }
            Err(e) => {
                return Err(CoreError::ProfilesIo {
                    path,
                    message: e.to_string(),
                });
            }
        };
        Self::from_toml_str(&text)
    }

    /// Looks a profile up by name; the error lists what exists.
    pub fn get(&self, name: &str) -> Result<&Profile, CoreError> {
        self.profile
            .get(name)
            .ok_or_else(|| CoreError::ProfileUnknown {
                name: name.to_owned(),
                available: self.profile.keys().cloned().collect::<Vec<_>>().join(", "),
            })
    }
}

impl Profile {
    /// Resolves a packet's routing slot (`big` / `small`) to a model id.
    pub fn resolve_model(&self, slot: &str) -> Result<&str, CoreError> {
        match slot {
            "big" => Ok(&self.models.big),
            "small" => Ok(&self.models.small),
            other => Err(CoreError::ModelSlot {
                slot: other.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D6_SAMPLE: &str = r#"
        schema = 1
        [profile.glm]
        backend = "claude-code"
        base_url = "https://api.z.ai/api/anthropic"
        token_file = "~/.vibevm/zai.api.token"
        [profile.glm.models]
        big = "glm-5.2[1m]"
        small = "glm-5-turbo"
        haiku_slot = "glm-5-turbo"
        [profile.glm.env]
        API_TIMEOUT_MS = "3000000"
    "#;

    #[test]
    fn d6_sample_parses_with_documented_defaults() {
        let file = ProfilesFile::from_toml_str(D6_SAMPLE).expect("parses");
        let glm = file.get("glm").expect("glm exists");
        assert_eq!(glm.backend, "claude-code");
        assert_eq!(glm.base_url, "https://api.z.ai/api/anthropic");
        assert_eq!(glm.token_file, Utf8PathBuf::from("~/.vibevm/zai.api.token"));
        assert_eq!(glm.claude_binary, "claude");
        assert_eq!(glm.config_dir, "auto");
        assert_eq!(glm.models.big, "glm-5.2[1m]");
        assert_eq!(
            glm.env.get("API_TIMEOUT_MS").map(String::as_str),
            Some("3000000")
        );
        assert_eq!(glm.limits.max_concurrent, 4);
        assert_eq!(glm.permissions.mode, "acceptEdits");
        assert!(glm.permissions.deny_tools.is_empty());
        assert!(!glm.pricing.flat);
        assert_eq!(glm.pricing.plan, None);
    }

    #[test]
    fn foreign_schema_is_refused() {
        let text = D6_SAMPLE.replace("schema = 1", "schema = 2");
        let err = ProfilesFile::from_toml_str(&text).expect_err("schema 2 must fail");
        assert!(matches!(err, CoreError::ProfilesSchema { found: 2 }));
    }

    #[test]
    fn unknown_fields_are_rejected_loudly() {
        let text = format!("{D6_SAMPLE}\n[profile.glm.limits]\nmax_concurent = 4\n");
        assert!(
            ProfilesFile::from_toml_str(&text).is_err(),
            "typo must not pass"
        );
    }

    #[test]
    fn empty_anchor_field_is_refused_with_the_field_name() {
        let text = D6_SAMPLE.replace("big = \"glm-5.2[1m]\"", "big = \"  \"");
        let err = ProfilesFile::from_toml_str(&text).expect_err("blank model must fail");
        assert!(err.to_string().contains("models.big"), "{err}");
    }

    #[test]
    fn unknown_profile_lists_what_exists() {
        let file = ProfilesFile::from_toml_str(D6_SAMPLE).expect("parses");
        let err = file.get("nope").expect_err("unknown must fail");
        assert!(err.to_string().contains("available: glm"), "{err}");
    }

    #[test]
    fn model_slots_resolve_and_refuse() {
        let file = ProfilesFile::from_toml_str(D6_SAMPLE).expect("parses");
        let glm = file.get("glm").expect("glm");
        assert_eq!(glm.resolve_model("big").expect("big"), "glm-5.2[1m]");
        assert_eq!(glm.resolve_model("small").expect("small"), "glm-5-turbo");
        let err = glm
            .resolve_model("haiku")
            .expect_err("haiku is not a routing slot");
        assert!(matches!(err, CoreError::ModelSlot { .. }));
    }

    #[test]
    fn missing_file_is_a_distinct_helpful_error() {
        let dir = std::env::temp_dir().join(format!("fractality-profiles-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let home = Utf8PathBuf::from_path_buf(dir.clone()).expect("utf-8 temp dir");
        let err = ProfilesFile::load(&home).expect_err("no file yet");
        assert!(matches!(err, CoreError::ProfilesMissing { .. }));
        assert!(err.to_string().contains("profiles.toml"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
