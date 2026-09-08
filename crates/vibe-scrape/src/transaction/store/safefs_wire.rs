#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerWire {
    schema: u32,
    project_key: String,
    transaction_id: String,
    journal_intent_sha256: String,
    ownership_token: String,
    directory_identity: String,
    entry_identity: String,
    workspace_directory_identity: String,
    workspace_entry_identity: String,
    workspace_project_identity: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetirementWire {
    schema: u32,
    project_key: String,
    transaction_id: String,
    stable_report_sha256: String,
    ownership_token: String,
    directory_identity: String,
    manifest: SafefsManifestWire,
    completed: Vec<String>,
    active: Option<CleanupIntentWire>,
    tree_removed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SafefsManifestWire {
    digest: String,
    entries: Vec<SafefsTreeEntryWire>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SafefsTreeEntryWire {
    path: String,
    state: SafefsEntryStateWire,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SafefsEntryStateWire {
    kind: SafefsEntryKindWire,
    sha256: Option<String>,
    bytes: Option<u64>,
    unix_mode: Option<u32>,
    identity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SafefsEntryKindWire {
    File,
    Directory,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanupIntentWire {
    intent_token: String,
    progress_key: String,
    path: String,
    expected: SafefsEntryStateWire,
    root: bool,
}

impl From<&EntryState> for SafefsEntryStateWire {
    fn from(value: &EntryState) -> Self {
        Self {
            kind: match value.kind {
                EntryStateKind::File => SafefsEntryKindWire::File,
                EntryStateKind::Directory => SafefsEntryKindWire::Directory,
            },
            sha256: value.sha256.clone(),
            bytes: value.bytes,
            unix_mode: value.unix_mode,
            identity: value.identity.as_str().to_owned(),
        }
    }
}

impl TryFrom<SafefsEntryStateWire> for EntryState {
    type Error = TransactionError;

    fn try_from(value: SafefsEntryStateWire) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: match value.kind {
                SafefsEntryKindWire::File => EntryStateKind::File,
                SafefsEntryKindWire::Directory => EntryStateKind::Directory,
            },
            sha256: value.sha256,
            bytes: value.bytes,
            unix_mode: value.unix_mode,
            identity: EntryIdentity::from_token(&value.identity).map_err(|error| {
                store_error(format!("invalid retirement entry identity: {error:#}"))
            })?,
        })
    }
}

impl From<&SafefsTreeManifest> for SafefsManifestWire {
    fn from(value: &SafefsTreeManifest) -> Self {
        Self {
            digest: value.digest.clone(),
            entries: value
                .entries
                .iter()
                .map(|entry| SafefsTreeEntryWire {
                    path: entry.path.clone(),
                    state: SafefsEntryStateWire::from(&entry.state),
                })
                .collect(),
        }
    }
}

impl TryFrom<SafefsManifestWire> for SafefsTreeManifest {
    type Error = TransactionError;

    fn try_from(value: SafefsManifestWire) -> Result<Self, Self::Error> {
        if value.entries.len() > MAX_TREE_ENTRIES {
            return Err(store_error("retirement manifest exceeds entry bound"));
        }
        let manifest = Self {
            digest: value.digest,
            entries: value
                .entries
                .into_iter()
                .map(|entry| {
                    Ok(SafefsTreeEntry {
                        path: entry.path,
                        state: entry.state.try_into()?,
                    })
                })
                .collect::<Result<_, TransactionError>>()?,
        };
        validate_safefs_manifest(&manifest)?;
        Ok(manifest)
    }
}

impl From<&CleanupIntent> for CleanupIntentWire {
    fn from(value: &CleanupIntent) -> Self {
        Self {
            intent_token: value.intent_token.clone(),
            progress_key: value.progress_key.clone(),
            path: value.path.clone(),
            expected: SafefsEntryStateWire::from(&value.expected),
            root: value.root,
        }
    }
}

impl TryFrom<CleanupIntentWire> for CleanupIntent {
    type Error = TransactionError;

    fn try_from(value: CleanupIntentWire) -> Result<Self, Self::Error> {
        Ok(Self {
            intent_token: value.intent_token,
            progress_key: value.progress_key,
            path: value.path,
            expected: value.expected.try_into()?,
            root: value.root,
        })
    }
}
