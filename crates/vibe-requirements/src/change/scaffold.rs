//! Pure authored output from one complete retained fact selection.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD-PREPARATION");
use crate::complete::FactSelection;
use std::{collections::BTreeMap, path::PathBuf};
use vibe_core::manifest::change::{ChangeContext, ChangeId};

#[derive(Debug)]
pub struct ScaffoldOutput {
    pub directory: PathBuf,
    pub files: BTreeMap<String, Vec<u8>>,
    pub analysis: super::ParsedCapturedChange,
    pub capture_id: String,
    pub filter_id: String,
    pub selected_addresses: Vec<String>,
    pub source_strengths: BTreeMap<String, String>,
}
#[derive(Debug, thiserror::Error)]
pub enum ScaffoldError {
    #[error("selected fact scope is incomplete or empty")]
    Incomplete,
    #[error("invalid authored scaffold: {0:?}")]
    Authored(super::SavedChangeError),
}
fn quoted(value: &str) -> String {
    toml::Value::String(value.into()).to_string()
}
pub fn prepare_scaffold(
    context: &ChangeContext,
    id: &ChangeId,
    slug: &str,
    title: &str,
    selection: &FactSelection<'_>,
) -> Result<ScaffoldOutput, ScaffoldError> {
    if !selection.is_complete() || selection.observed().is_empty() {
        return Err(ScaffoldError::Incomplete);
    }
    let inventory = selection.inventory();
    let rows = selection.observed();
    let mut source_strengths = BTreeMap::new();
    for row in &rows {
        let source = inventory
            .documents()
            .get(&row.document.path)
            .ok_or(ScaffoldError::Incomplete)?;
        if row.document.digest != format!("sha256:{}", source.snapshot().sha256) {
            return Err(ScaffoldError::Incomplete);
        }
        use vibe_workspace::source_query::SourceStrength;
        let grade = match source.provenance().strength {
            SourceStrength::Observed => "observed",
            SourceStrength::RecordConsistent => "record-consistent",
            SourceStrength::SourceProven => "source-proven",
        };
        source_strengths.insert(row.document.path.clone(), grade.into());
    }
    let mut text = format!(
        "# Prepared local change; package facts remain references.\nschema = 1\n\n[change]\nid = {}\nselected_node = {}\nslug = {}\ntitle = {}\n\n[[origins]]\npackage = {}\nselected_content_hash = {}\nmanifest_sha256 = {}\n",
        quoted(id.as_str()),
        quoted(context.selected_node()),
        quoted(slug),
        quoted(title),
        quoted(&inventory.pin().to_string()),
        quoted(inventory.content_hash().as_str()),
        quoted(&format!("sha256:{}", inventory.manifest().sha256))
    );
    let addresses: Vec<_> = rows
        .iter()
        .map(|row| row.authored.address.clone())
        .collect();
    for row in rows {
        text.push_str(&format!(
            "\n[[origins.documents]]\naddress = {}\nsha256 = {}\n",
            quoted(&row.authored.address),
            quoted(&row.document.digest)
        ));
    }
    let mut files = BTreeMap::new();
    for (role, name, body) in [
        (
            "intent",
            "intent.md",
            "# Intent\n\n@fact:LOCAL-INTENT Implement the referenced origin in this project. @status:spec\n",
        ),
        (
            "refinement",
            "refinement.md",
            "# Refinement\n\n@fact:LOCAL-REFINEMENT Record project-specific implementation constraints here. @status:spec\n",
        ),
        (
            "acceptance",
            "acceptance.md",
            "# Acceptance\n\n@fact:LOCAL-ACCEPTANCE Supply local implementation and verification evidence. @status:spec\n",
        ),
    ] {
        text.push_str(&format!(
            "\n[[documents]]\nrole = {}\npath = {}\nnormative = true\ninput = true\n",
            quoted(role),
            quoted(name)
        ));
        let mut body = body.to_owned();
        if role == "intent" {
            for address in &addresses {
                body.push_str(&format!("\nOrigin reference: @{address}\n"));
            }
        }
        files.insert(name.into(), body.into_bytes());
    }
    let directory = context.directory(id);
    let inputs: Vec<_> = files
        .iter()
        .map(|(path, bytes)| (directory.join(path), bytes.as_slice()))
        .collect();
    let borrowed: Vec<_> = inputs
        .iter()
        .map(|(path, bytes)| (path.as_path(), *bytes))
        .collect();
    let analysis = super::analyze_change_bytes(&text, context, &directory, &borrowed)
        .map_err(ScaffoldError::Authored)?;
    files.insert("change.toml".into(), text.into_bytes());
    Ok(ScaffoldOutput {
        directory,
        files,
        analysis,
        capture_id: selection.capture_id().into(),
        filter_id: selection.filter_id().into(),
        selected_addresses: addresses,
        source_strengths,
    })
}
