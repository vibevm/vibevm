//! Fixed Pi models.json sink; credentials never cross this module's API.
use super::*;
use serde_json::{Value, json};

#[cfg(all(test, unix))]
mod tests;

const PROVIDER: &str = "vibevm";
pub(super) const NATIVE_LIMIT: usize = 4 * 1024 * 1024;

/// Opaque operation subject. Debug/serialization cannot expose native contents.
pub struct PiNativePreview {
    kind: super::Kind,
    home: PathBuf,
    settings_scope: PathBuf,
    connection: SecretId,
    reference_digest: String,
    key: SecretId,
    expected: Expected,
    native_digest: Option<String>,
    auth_digest: Option<String>,
    native_identity: vibe_safefs::FileIdentity,
    token: String,
}
impl std::fmt::Debug for PiNativePreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PiNativePreview([REDACTED])")
    }
}
impl PiNativePreview {
    pub fn preview_id(&self) -> &str {
        &self.token
    }
}

fn native(home: &Path, kind: super::Kind) -> Result<(Project, Pinned), Error> {
    if !home.is_absolute() || home.canonicalize().map_err(|_| Error::Unavailable)? != home {
        return Err(Error::Unavailable);
    }
    let project = Project::open(home).map_err(|_| Error::Unavailable)?;
    let home_dir = project.root_dir().map_err(|_| Error::Unavailable)?;
    home_dir
        .verify_user_owned_parent()
        .map_err(|_| Error::Unavailable)?;
    let agent = match kind {
        super::Kind::Pi => home_dir
            .private_child(".pi", false)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?
            .private_child("agent", false)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?,
        super::Kind::Qwen => home_dir
            .private_child(".qwen", false)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?,
    };
    Ok((project, agent))
}
fn config(dir: &Pinned, kind: super::Kind) -> Result<Option<Vec<u8>>, Error> {
    dir.read_private_metadata(kind.file(), NATIVE_LIMIT)
        .map_err(|_| Error::Unavailable)
}
fn auth_witness(dir: &Pinned, kind: super::Kind) -> Result<Option<String>, Error> {
    if kind == super::Kind::Qwen {
        return Ok(None);
    }
    let bytes = dir
        .read_private_metadata("auth.json", NATIVE_LIMIT)
        .map_err(|_| Error::Unavailable)?;
    if let Some(bytes) = &bytes {
        let value: Value = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        let object = value.as_object().ok_or(Error::Invalid)?;
        // Native stored credentials take precedence over models.json keys.
        // Never report success while another credential shadows the new pair.
        if object.contains_key(PROVIDER) {
            return Err(Error::Invalid);
        }
    }
    Ok(witness(&bytes))
}
fn witness(bytes: &Option<Vec<u8>>) -> Option<String> {
    bytes.as_ref().map(|b| digest(b))
}
fn endpoint_model(snapshot: &ReferenceSnapshot, id: &SecretId) -> Result<(String, String), Error> {
    let bytes = snapshot.metadata.get(id).ok_or(Error::NotFound)?;
    let metadata: toml::Value =
        toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Invalid)?)
            .map_err(|_| Error::Invalid)?;
    if metadata.get("api_kind").and_then(toml::Value::as_str) != Some("openai-chat-completions") {
        return Err(Error::Invalid);
    }
    let endpoint = metadata
        .get("endpoint")
        .and_then(toml::Value::as_str)
        .ok_or(Error::Invalid)?;
    let model = metadata
        .get("default_model")
        .and_then(toml::Value::as_str)
        .ok_or(Error::Invalid)?;
    let url = url::Url::parse(endpoint).map_err(|_| Error::Invalid)?;
    let local = match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    if endpoint.len() > 4096
        || endpoint.chars().any(char::is_control)
        || model.is_empty()
        || model.len() > 256
        || model.chars().any(char::is_control)
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https" || url.scheme() == "http" && local)
    {
        return Err(Error::Invalid);
    }
    Ok((endpoint.into(), model.into()))
}
fn document(bytes: &Option<Vec<u8>>) -> Result<Value, Error> {
    let value = match bytes {
        Some(bytes) => serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?,
        None => json!({}),
    };
    if !value.is_object() {
        return Err(Error::Invalid);
    }
    if let Some(providers) = value.get("providers") {
        let providers = providers.as_object().ok_or(Error::Invalid)?;
        if let Some(provider) = providers.get(PROVIDER) {
            let provider = provider.as_object().ok_or(Error::Invalid)?;
            // An existing OAuth provider is never silently converted to an API key.
            if provider.contains_key("oauth") {
                return Err(Error::Invalid);
            }
            if let Some(models) = provider.get("models") {
                let models = models.as_array().ok_or(Error::Invalid)?;
                if models
                    .iter()
                    .any(|m| !m.is_object() || m.get("id").and_then(Value::as_str).is_none())
                {
                    return Err(Error::Invalid);
                }
            }
        }
    }
    Ok(value)
}

impl Store {
    /// Metadata-only preview for an existing protected default Pi directory.
    /// Explicit HOME injection, never discovery or an arbitrary secret file path.
    pub fn preview_pi_native_settings(
        &self,
        home: &Path,
        connection: &SecretId,
    ) -> Result<PiNativePreview, Error> {
        self.preview_native_settings(home, connection, super::Kind::Pi)
    }
    pub(super) fn preview_native_settings(
        &self,
        home: &Path,
        connection: &SecretId,
        kind: super::Kind,
    ) -> Result<PiNativePreview, Error> {
        let settings_scope = self.root.canonicalize().map_err(|_| Error::Unavailable)?;
        if settings_scope != self.root {
            return Err(Error::Unavailable);
        }
        let snapshot = self.reference_snapshot("ai")?;
        endpoint_model(&snapshot, connection)?;
        let row = snapshot
            .index
            .items
            .get(connection)
            .ok_or(Error::NotFound)?;
        let keys = self.snapshot()?;
        let item = keys.items.get(&row.secret_ref).ok_or(Error::NotFound)?;
        let (_, dir) = native(home, kind)?;
        let bytes = config(&dir, kind)?;
        match kind {
            super::Kind::Pi => {
                document(&bytes)?;
            }
            super::Kind::Qwen => super::qwen::validate(&bytes)?,
        }
        let native_digest = witness(&bytes);
        let auth_digest = auth_witness(&dir, kind)?;
        let mut hash = Sha256::new();
        hash.update(kind.domain());
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let path = home.as_os_str().as_bytes();
            hash.update((path.len() as u64).to_be_bytes());
            hash.update(path);
            let scope = settings_scope.as_os_str().as_bytes();
            hash.update((scope.len() as u64).to_be_bytes());
            hash.update(scope);
        }
        for text in [
            snapshot.index_sha256.as_str(),
            connection.as_str(),
            row.secret_ref.as_str(),
            item.current_revision.as_str(),
            native_digest.as_deref().unwrap_or("absent"),
            auth_digest.as_deref().unwrap_or("absent"),
        ] {
            hash.update((text.len() as u64).to_be_bytes());
            hash.update(text.as_bytes());
        }
        hash.update(keys.store_revision.to_be_bytes());
        // FileIdentity exposes Hash while retaining its opaque representation.
        // Feed it into the operation digest, not Debug text or ambient stat.
        struct IdentityHash<'a>(&'a mut Sha256);
        impl std::hash::Hasher for IdentityHash<'_> {
            fn finish(&self) -> u64 {
                0
            }
            fn write(&mut self, bytes: &[u8]) {
                self.0.update((bytes.len() as u64).to_be_bytes());
                self.0.update(bytes);
            }
        }
        std::hash::Hash::hash(
            &dir.identity().map_err(|_| Error::Unavailable)?,
            &mut IdentityHash(&mut hash),
        );
        let token = format!(
            "{}-native-settings/1:sha256:{:x}",
            kind.id(),
            hash.finalize()
        );
        Ok(PiNativePreview {
            kind,
            home: home.into(),
            settings_scope,
            connection: connection.clone(),
            reference_digest: snapshot.index_sha256,
            key: row.secret_ref.clone(),
            expected: Expected {
                store_revision: keys.store_revision,
                current_revision: item.current_revision.clone(),
            },
            native_digest,
            auth_digest,
            native_identity: dir.identity().map_err(|_| Error::Unavailable)?,
            token,
        })
    }

    /// Confirmed concrete sink. No raw getter, callback, export or caller-chosen file/key path.
    /// Returns whether the single protected native file changed.
    pub fn apply_pi_native_settings(
        &self,
        preview: &PiNativePreview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        if preview.kind != super::Kind::Pi {
            return Err(Error::Invalid);
        }
        self.apply_native_settings(preview, confirmed)
    }
    pub(super) fn apply_native_settings(
        &self,
        preview: &PiNativePreview,
        confirmed: &str,
    ) -> Result<bool, Error> {
        if confirmed != preview.token
            || self.root != preview.settings_scope
            || self.root.canonicalize().map_err(|_| Error::Unavailable)? != preview.settings_scope
        {
            return Err(Error::Conflict);
        }
        let (project, root) = self.open(false)?.ok_or(Error::NotFound)?;
        let _inventory = root
            .private_lock("inventory.lock")
            .map_err(|_| Error::Unavailable)?;
        let _store = root
            .private_lock("store.lock")
            .map_err(|_| Error::Unavailable)?;
        let keys = Self::load(&project, &root)?;
        let item = keys.items.get(&preview.key).ok_or(Error::NotFound)?;
        if keys.store_revision != preview.expected.store_revision
            || item.current_revision != preview.expected.current_revision
        {
            return Err(Error::Conflict);
        }
        let ai = root
            .private_child("ai", false)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        let snapshot = read(&ai)?;
        if snapshot.index_sha256 != preview.reference_digest
            || snapshot
                .index
                .items
                .get(&preview.connection)
                .ok_or(Error::NotFound)?
                .secret_ref
                != preview.key
        {
            return Err(Error::Conflict);
        }
        let (endpoint, model) = endpoint_model(&snapshot, &preview.connection)?;
        let (native_project, dir) = native(&preview.home, preview.kind)?;
        if dir.identity().map_err(|_| Error::Unavailable)? != preview.native_identity {
            return Err(Error::Conflict);
        }
        let _native = dir
            .private_lock("vibevm-native-settings.lock")
            .map_err(|_| Error::Unavailable)?;
        let before = config(&dir, preview.kind)?;
        if witness(&before) != preview.native_digest
            || auth_witness(&dir, preview.kind)? != preview.auth_digest
        {
            return Err(Error::Conflict);
        }
        match preview.kind {
            super::Kind::Pi => {
                document(&before)?;
            }
            super::Kind::Qwen => super::qwen::validate(&before)?,
        }
        // Resolve only after all operation, revision, target and protection checks.
        let revision = open_revision(&root, &preview.key, &item.current_revision, false)?;
        let secret = revision
            .read_private_metadata("value", VALUE_LIMIT)
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        let secret = std::str::from_utf8(&secret).map_err(|_| Error::Invalid)?;
        if secret.is_empty() || secret.chars().any(char::is_control) {
            return Err(Error::Invalid);
        }
        let bytes = match preview.kind {
            super::Kind::Pi => merge_pi(&before, &endpoint, &model, secret)?,
            super::Kind::Qwen => super::qwen::merge(&before, &endpoint, &model, secret)?,
        };
        let Some(bytes) = bytes else {
            return Ok(false);
        };
        if bytes.len() > NATIVE_LIMIT {
            return Err(Error::Invalid);
        }
        if config(&dir, preview.kind)? != before
            || auth_witness(&dir, preview.kind)? != preview.auth_digest
        {
            return Err(Error::Conflict);
        }
        native_project
            .write_atomic_in_with_mode(&dir, preview.kind.file(), &bytes, Some(0o600))
            .map_err(|_| Error::Publication)?;
        Ok(true)
    }
}

fn merge_pi(
    before: &Option<Vec<u8>>,
    endpoint: &str,
    model: &str,
    secret: &str,
) -> Result<Option<Vec<u8>>, Error> {
    let mut value = document(before)?;
    let mut literal = String::new();
    for c in secret.chars() {
        match c {
            '$' => literal.push_str("$$"),
            '!' => literal.push_str("$!"),
            _ => literal.push(c),
        }
    }
    let providers = value
        .as_object_mut()
        .ok_or(Error::Invalid)?
        .entry("providers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(Error::Invalid)?;
    let provider = providers
        .entry(PROVIDER)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(Error::Invalid)?;
    provider.insert("baseUrl".into(), json!(endpoint));
    provider.insert("api".into(), json!("openai-completions"));
    provider.insert("apiKey".into(), json!(literal));
    let models = provider
        .entry("models")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or(Error::Invalid)?;
    if let Some(selected) = models
        .iter_mut()
        .find(|m| m.get("id").and_then(Value::as_str) == Some(model))
    {
        let selected = selected.as_object_mut().ok_or(Error::Invalid)?;
        // A selected model's old endpoint/API overrides must not defeat the
        // explicitly confirmed provider pairing. Other model fields survive.
        selected.insert("baseUrl".into(), json!(endpoint));
        selected.insert("api".into(), json!("openai-completions"));
    } else {
        models.push(json!({"id": model, "baseUrl": endpoint, "api": "openai-completions"}));
    }
    if value == document(before)? {
        return Ok(None);
    }
    Ok(Some(
        serde_json::to_vec_pretty(&value).map_err(|_| Error::Invalid)?,
    ))
}
