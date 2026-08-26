use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tempfile::tempdir;
use vibe_core::manifest::LlmSection;
use vibe_core::user_config::LlmConfig;
use vibe_llm::{
    CredentialReadError, CredentialReader, CredentialSource, EffectiveLlmConfigError,
    ReqwestChatTransport, resolve_effective_config,
};

const ENV_KEY: &str = "env-key-canary";
const FILE_KEY: &str = "file-key-canary";

#[derive(Default)]
struct FakeCredentials {
    env: BTreeMap<String, String>,
    files: BTreeMap<PathBuf, String>,
    env_reads: Mutex<Vec<String>>,
    file_reads: Mutex<Vec<PathBuf>>,
}

impl CredentialReader for FakeCredentials {
    fn read_env(&self, name: &str) -> Result<Option<String>, CredentialReadError> {
        self.env_reads.lock().unwrap().push(name.to_owned());
        Ok(self.env.get(name).cloned())
    }

    fn read_file(&self, path: &Path) -> Result<String, CredentialReadError> {
        self.file_reads.lock().unwrap().push(path.to_path_buf());
        self.files
            .get(path)
            .cloned()
            .ok_or(CredentialReadError::Unavailable)
    }
}

fn user(token_file: Option<PathBuf>) -> LlmConfig {
    LlmConfig {
        provider: "openai-compatible".into(),
        model: "user-model".into(),
        endpoint: "https://api.example.invalid/v1/chat/completions".into(),
        token_file,
    }
}

fn project(api_key_env: Option<&str>) -> LlmSection {
    LlmSection {
        default_provider: "openai-compatible".into(),
        default_model: "project-model".into(),
        api_key_env: api_key_env.map(ToOwned::to_owned),
    }
}

#[test]
fn project_provider_model_and_env_override_user_fields_without_file_read() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let token_path = dir.path().join("provider.token");
    let mut credentials = FakeCredentials::default();
    credentials.env.insert("PROJECT_KEY".into(), ENV_KEY.into());
    credentials.files.insert(token_path, FILE_KEY.into());

    let effective = resolve_effective_config(
        Some(&user(Some(PathBuf::from("provider.token")))),
        &config_path,
        Some(&project(Some("PROJECT_KEY"))),
        &credentials,
    )
    .unwrap()
    .unwrap();
    assert_eq!(effective.provider(), "openai-compatible");
    assert_eq!(effective.model(), "project-model");
    assert_eq!(
        effective.endpoint().as_str(),
        "https://api.example.invalid/v1/chat/completions"
    );
    assert_eq!(
        effective.credential_source(),
        Some(&CredentialSource::Environment("PROJECT_KEY".into()))
    );
    assert_eq!(
        credentials.env_reads.lock().unwrap().as_slice(),
        ["PROJECT_KEY"]
    );
    assert!(credentials.file_reads.lock().unwrap().is_empty());
    let debug = format!("{effective:?}");
    assert!(!debug.contains(ENV_KEY));
    assert!(!debug.contains(FILE_KEY));
}

#[test]
fn relative_absolute_and_tilde_token_paths_have_literal_resolution() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("nested/config.toml");
    let config_dir = config_path.parent().unwrap();

    for (declared, expected) in [
        (
            PathBuf::from("provider.token"),
            config_dir.join("provider.token"),
        ),
        (
            PathBuf::from("~/provider.token"),
            config_dir.join("~/provider.token"),
        ),
        (
            dir.path().join("absolute.token"),
            dir.path().join("absolute.token"),
        ),
    ] {
        let mut credentials = FakeCredentials::default();
        credentials.files.insert(expected.clone(), FILE_KEY.into());
        let effective = resolve_effective_config(
            Some(&user(Some(declared))),
            &config_path,
            None,
            &credentials,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            effective.credential_source(),
            Some(&CredentialSource::TokenFile(expected.clone()))
        );
        assert_eq!(
            credentials.file_reads.lock().unwrap().as_slice(),
            [expected]
        );
    }
}

#[test]
fn empty_project_env_name_yields_to_user_token_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let expected = dir.path().join("provider.token");
    let mut credentials = FakeCredentials::default();
    credentials.files.insert(expected.clone(), FILE_KEY.into());
    let effective = resolve_effective_config(
        Some(&user(Some(PathBuf::from("provider.token")))),
        &config_path,
        Some(&project(Some("  "))),
        &credentials,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        effective.credential_source(),
        Some(&CredentialSource::TokenFile(expected))
    );
    assert!(credentials.env_reads.lock().unwrap().is_empty());
}

#[test]
fn exact_provider_id_has_no_aliases_and_no_config_means_no_provider() {
    let credentials = FakeCredentials::default();
    assert!(
        resolve_effective_config(None, Path::new("config.toml"), None, &credentials)
            .unwrap()
            .is_none()
    );

    for alias in ["openai", "openrouter", "ollama", "anthropic"] {
        let mut layer = user(None);
        layer.provider = alias.into();
        assert!(matches!(
            resolve_effective_config(
                Some(&layer),
                Path::new("config.toml"),
                None,
                &credentials
            ),
            Err(EffectiveLlmConfigError::UnsupportedProvider(value)) if value == alias
        ));
    }
    assert!(credentials.env_reads.lock().unwrap().is_empty());
    assert!(credentials.file_reads.lock().unwrap().is_empty());
}

#[test]
fn a_configured_but_missing_env_credential_does_not_fall_back_to_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let token_path = dir.path().join("provider.token");
    let mut credentials = FakeCredentials::default();
    credentials.files.insert(token_path, FILE_KEY.into());
    let error = resolve_effective_config(
        Some(&user(Some(PathBuf::from("provider.token")))),
        &config_path,
        Some(&project(Some("MISSING_PROJECT_KEY"))),
        &credentials,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        EffectiveLlmConfigError::CredentialUnavailable(CredentialSource::Environment(name))
            if name == "MISSING_PROJECT_KEY"
    ));
    assert!(credentials.file_reads.lock().unwrap().is_empty());
}

#[test]
fn blank_env_and_unreadable_or_blank_token_files_fail_without_secret_text() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let mut blank_env = FakeCredentials::default();
    blank_env.env.insert("BLANK_KEY".into(), "   ".into());
    let error = resolve_effective_config(
        Some(&user(Some(PathBuf::from("unused.token")))),
        &config_path,
        Some(&project(Some("BLANK_KEY"))),
        &blank_env,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        EffectiveLlmConfigError::InvalidCredential { .. }
    ));
    assert!(blank_env.file_reads.lock().unwrap().is_empty());

    let unreadable = FakeCredentials::default();
    let error = resolve_effective_config(
        Some(&user(Some(PathBuf::from("missing.token")))),
        &config_path,
        None,
        &unreadable,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        EffectiveLlmConfigError::CredentialUnavailable(CredentialSource::TokenFile(_))
    ));

    let blank_path = dir.path().join("blank.token");
    let mut blank_file = FakeCredentials::default();
    blank_file.files.insert(blank_path, "\n\t".into());
    let error = resolve_effective_config(
        Some(&user(Some(PathBuf::from("blank.token")))),
        &config_path,
        None,
        &blank_file,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        EffectiveLlmConfigError::InvalidCredential { .. }
    ));
}

#[test]
fn provider_precedence_is_per_field_and_project_is_the_higher_row() {
    let credentials = FakeCredentials::default();
    let mut bad_user = user(None);
    bad_user.provider = "anthropic".into();
    let effective = resolve_effective_config(
        Some(&bad_user),
        Path::new("config.toml"),
        Some(&project(None)),
        &credentials,
    )
    .unwrap()
    .unwrap();
    assert_eq!(effective.provider(), "openai-compatible");
    assert_eq!(effective.model(), "project-model");

    let mut bad_project = project(None);
    bad_project.default_provider = "openai".into();
    assert!(matches!(
        resolve_effective_config(
            Some(&user(None)),
            Path::new("config.toml"),
            Some(&bad_project),
            &credentials
        ),
        Err(EffectiveLlmConfigError::UnsupportedProvider(value)) if value == "openai"
    ));

    assert!(matches!(
        resolve_effective_config(
            None,
            Path::new("config.toml"),
            Some(&project(None)),
            &credentials
        ),
        Err(EffectiveLlmConfigError::MissingField("endpoint"))
    ));
}

#[test]
fn endpoint_query_values_are_used_but_redacted_from_all_debug_surfaces() {
    const QUERY_CANARY: &str = "query-secret-canary";
    let credentials = FakeCredentials::default();
    let mut layer = user(None);
    layer.endpoint =
        format!("https://api.example.invalid/v1/chat/completions?api-version={QUERY_CANARY}");
    let effective =
        resolve_effective_config(Some(&layer), Path::new("config.toml"), None, &credentials)
            .unwrap()
            .unwrap();
    assert!(effective.endpoint().as_str().contains(QUERY_CANARY));
    assert!(!format!("{:?}", effective.endpoint()).contains(QUERY_CANARY));
    assert!(!format!("{effective:?}").contains(QUERY_CANARY));

    let provider = effective
        .into_provider(Arc::new(ReqwestChatTransport::new().unwrap()))
        .unwrap();
    assert!(!format!("{provider:?}").contains(QUERY_CANARY));
}
