//! Strict native scaffold intent and exact partial observation journal.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD-RECOVERY");
use super::*;
use serde::{Deserialize, Serialize};
const CAP: usize = 32 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum State {
    Pending,
    Verified,
    Uncertain,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct File {
    pub name: String,
    pub desired: Vec<u8>,
    pub sha256: String,
    pub identity: Option<String>,
    pub unix_mode: Option<u32>,
    pub state: State,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Journal {
    pub schema: u32,
    pub recipe: String,
    pub id: String,
    pub intent_sha256: String,
    pub root_binding: String,
    pub capture: String,
    pub source_capture: String,
    pub node_path: String,
    pub node_before: Vec<u8>,
    pub node_sha256: String,
    pub node_identity: String,
    pub node_mode: Option<u32>,
    pub directory: String,
    pub directory_identity: Option<String>,
    pub directory_mode: Option<u32>,
    pub directory_uncertain: bool,
    pub created_parents: Vec<String>,
    pub created_parent_identities: BTreeMap<String, String>,
    pub uncertain_directories: Vec<String>,
    pub completed: bool,
    pub files: Vec<File>,
}
fn hash(raw: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(raw))
}
fn directory_key(root: &str, directory: &str) -> String {
    let mut out = b"change-scaffold-directory-key/1".to_vec();
    for value in [root, directory] {
        out.extend_from_slice(&(value.len() as u64).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    hash(&out)
}
pub(super) fn intent_id(j: &Journal) -> String {
    fn field(out: &mut Vec<u8>, raw: &[u8]) {
        out.extend_from_slice(&(raw.len() as u64).to_be_bytes());
        out.extend_from_slice(raw);
    }
    let mut out = b"change-scaffold-journal/1".to_vec();
    for value in [
        &j.recipe,
        &j.root_binding,
        &j.capture,
        &j.source_capture,
        &j.node_path,
        &j.node_sha256,
        &j.node_identity,
        &j.directory,
    ] {
        field(&mut out, value.as_bytes());
    }
    match j.node_mode {
        None => out.push(0),
        Some(mode) => {
            out.push(1);
            out.extend_from_slice(&mode.to_be_bytes());
        }
    }
    out.extend_from_slice(&(j.files.len() as u64).to_be_bytes());
    for f in &j.files {
        field(&mut out, f.name.as_bytes());
        field(&mut out, f.sha256.as_bytes());
    }
    hash(&out)
}
pub(super) fn from_plan(plan: &PreparedScaffold) -> Journal {
    let mut j = Journal {
        schema: 1,
        recipe: "change-scaffold-journal/1".into(),
        id: String::new(),
        intent_sha256: String::new(),
        root_binding: plan.authority.root_binding.clone(),
        capture: plan.editor_capture.clone(),
        source_capture: plan.output.capture_id.clone(),
        node_path: plan.node_path.clone(),
        node_before: plan.node_before.bytes.clone(),
        node_sha256: plan.node_before.sha256.clone(),
        node_identity: plan.node_before.identity.observation_correlation_token(),
        node_mode: plan.node_before.unix_mode,
        directory: plan.authority.directory.to_str().unwrap().into(),
        directory_identity: None,
        directory_mode: None,
        directory_uncertain: false,
        created_parents: vec![],
        created_parent_identities: BTreeMap::new(),
        uncertain_directories: vec![],
        completed: false,
        files: plan
            .output
            .files
            .iter()
            .map(|(name, bytes)| File {
                name: name.clone(),
                desired: bytes.clone(),
                sha256: hash(bytes),
                identity: None,
                unix_mode: None,
                state: State::Pending,
            })
            .collect(),
    };
    j.id = directory_key(&j.root_binding, &j.directory);
    j.intent_sha256 = intent_id(&j);
    j
}
pub(super) fn path(id: &str) -> Result<String, ScaffoldRefusal> {
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ScaffoldRefusal::Journal);
    }
    Ok(format!(".vibe/change-scaffolds/{id}.toml"))
}
pub(super) fn decode(
    raw: &[u8],
    authority: &ScaffoldAuthority,
) -> Result<Journal, ScaffoldRefusal> {
    let j = decode_contract(raw)?;
    if j.root_binding != authority.root_binding
        || Path::new(&j.directory) != authority.directory
        || authority.host.resource(&j.node_path) != Some(authority.node)
    {
        return Err(ScaffoldRefusal::Journal);
    }
    Ok(j)
}
pub(super) fn decode_contract(raw: &[u8]) -> Result<Journal, ScaffoldRefusal> {
    if raw.len() > CAP {
        return Err(ScaffoldRefusal::Journal);
    }
    let text = std::str::from_utf8(raw).map_err(|_| ScaffoldRefusal::Journal)?;
    let j: Journal = toml::from_str(text).map_err(|_| ScaffoldRefusal::Journal)?;
    path(&j.id)?;
    if j.schema != 1
        || j.recipe != "change-scaffold-journal/1"
        || j.id != directory_key(&j.root_binding, &j.directory)
        || j.intent_sha256 != intent_id(&j)
        || j.files.len() != 4
    {
        return Err(ScaffoldRefusal::Journal);
    }
    let mut names = std::collections::BTreeSet::new();
    if hash(&j.node_before) != j.node_sha256
        || j.node_identity.is_empty()
        || j.node_identity.len() > 256
        || j.node_before.len() > 4 * 1024 * 1024
    {
        return Err(ScaffoldRefusal::Journal);
    }
    for f in &j.files {
        if !["acceptance.md", "change.toml", "intent.md", "refinement.md"]
            .contains(&f.name.as_str())
            || !names.insert(&f.name)
            || hash(&f.desired) != f.sha256
            || f.desired.len() > 4 * 1024 * 1024
            || (f.state == State::Verified && f.identity.is_none())
            || f.identity
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 256)
        {
            return Err(ScaffoldRefusal::Journal);
        }
    }
    if j.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
        != ["acceptance.md", "change.toml", "intent.md", "refinement.md"]
        || (j.completed
            && (j.directory_identity.is_none()
                || j.files.iter().any(|f| f.state != State::Verified)))
    {
        return Err(ScaffoldRefusal::Journal);
    }
    if j.directory_identity
        .as_ref()
        .is_some_and(|s| s.is_empty() || s.len() > 256)
    {
        return Err(ScaffoldRefusal::Journal);
    }
    for parent in j.created_parents.iter().chain(&j.uncertain_directories) {
        if !Path::new(&j.directory).starts_with(parent)
            || parent == &j.directory
            || vibe_safefs::split_relative(parent).is_err()
        {
            return Err(ScaffoldRefusal::Journal);
        }
    }
    let parents: std::collections::BTreeSet<_> = j.created_parents.iter().collect();
    if parents.len() != j.created_parents.len()
        || parents != j.created_parent_identities.keys().collect()
        || j.created_parent_identities
            .values()
            .any(|s| s.is_empty() || s.len() > 256)
        || (j.directory_uncertain && j.directory_identity.is_some())
        || (j.completed && (j.directory_uncertain || !j.uncertain_directories.is_empty()))
        || (j.directory_identity.is_none() && j.files.iter().any(|f| f.state != State::Pending))
    {
        return Err(ScaffoldRefusal::Journal);
    }
    if j.capture.is_empty()
        || j.capture.len() > 256
        || j.source_capture.is_empty()
        || j.source_capture.len() > 256
        || j.root_binding.is_empty()
        || j.root_binding.len() > 256
        || vibe_safefs::split_relative(&j.node_path).is_err()
        || vibe_safefs::split_relative(&j.directory).is_err()
    {
        return Err(ScaffoldRefusal::Journal);
    }
    let node_text = std::str::from_utf8(&j.node_before).map_err(|_| ScaffoldRefusal::Journal)?;
    let node = vibe_core::manifest::Manifest::parse_str(node_text)
        .map_err(|_| ScaffoldRefusal::Journal)?;
    let node_parent = Path::new(&j.node_path)
        .parent()
        .ok_or(ScaffoldRefusal::Journal)?;
    let relative = Path::new(&j.directory)
        .strip_prefix(node_parent)
        .map_err(|_| ScaffoldRefusal::Journal)?;
    let spec_root = relative
        .parent()
        .and_then(Path::parent)
        .ok_or(ScaffoldRefusal::Journal)?;
    let selected = if node_parent.as_os_str().is_empty() {
        "."
    } else {
        node_parent.to_str().ok_or(ScaffoldRefusal::Journal)?
    };
    let context = ChangeContext::for_node(&node, selected, Some(spec_root))
        .map_err(|_| ScaffoldRefusal::Journal)?;
    let change = j.files.iter().find(|f| f.name == "change.toml").unwrap();
    let text = std::str::from_utf8(&change.desired).map_err(|_| ScaffoldRefusal::Journal)?;
    let inputs: Vec<_> = j
        .files
        .iter()
        .filter(|f| f.name != "change.toml")
        .map(|f| (relative.join(&f.name), f.desired.as_slice()))
        .collect();
    let borrowed: Vec<_> = inputs.iter().map(|(p, b)| (p.as_path(), *b)).collect();
    let parsed =
        vibe_requirements::change::analyze_change_bytes(text, &context, relative, &borrowed)
            .map_err(|_| ScaffoldRefusal::Journal)?;
    if parsed.manifest.documents().len() != 3
        || parsed
            .manifest
            .origins()
            .iter()
            .any(|o| o.selected_content_hash.is_none())
    {
        return Err(ScaffoldRefusal::Journal);
    }
    Ok(j)
}
pub(super) fn checkpoint(
    project: &vibe_safefs::Project,
    j: &Journal,
) -> Result<(), ScaffoldRefusal> {
    let raw = toml::to_string(j)
        .map_err(|_| ScaffoldRefusal::Journal)?
        .into_bytes();
    if raw.len() > CAP {
        return Err(ScaffoldRefusal::Journal);
    }
    let path = path(&j.id)?;
    let prior = project
        .read_file_snapshot_bounded(&path, CAP)
        .map_err(infra)?;
    if let Some(prior) = prior {
        let written = project
            .write_snapshot_guarded(&path, &prior, &raw)
            .map_err(|error| infra(format!("{error:?}")))?;
        if !written.file_synced
            || matches!(written.parent, vibe_safefs::DirectoryDurability::Failed(_))
        {
            return Err(ScaffoldRefusal::Journal);
        }
    } else {
        let dir = project
            .dir(&[".vibe", "change-scaffolds"], true)
            .map_err(infra)?;
        project
            .publish_new_in(&dir, &format!("{}.toml", j.id), &raw)
            .map_err(infra)?;
    }
    let visible = project
        .read_file_snapshot_bounded(&path, CAP)
        .map_err(infra)?;
    if visible.as_ref().map(|s| s.bytes.as_slice()) != Some(raw.as_slice()) {
        return Err(ScaffoldRefusal::Journal);
    }
    Ok(())
}
pub(super) fn load(
    project: &vibe_safefs::Project,
    authority: &ScaffoldAuthority,
    id: &str,
) -> Result<Journal, ScaffoldRefusal> {
    let raw = project
        .read_file_snapshot_bounded(&path(id)?, CAP)
        .map_err(infra)?
        .ok_or(ScaffoldRefusal::Journal)?;
    let j = decode(&raw.bytes, authority)?;
    if j.id != id {
        return Err(ScaffoldRefusal::Journal);
    }
    Ok(j)
}
