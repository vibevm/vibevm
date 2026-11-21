//! Which installed instance a selector names (PROP-019 §2.3, §2.11): the
//! newest instance of an id, the highest release tag for `stable`, and the
//! branch > tag > commit precedence of a bare name. Split from the `vvm` hub
//! for the file-length budget; every verb still reaches it as
//! `super::resolve_installed`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#surface");

use super::error::VvmError;
use super::model::{self, InstallRecord, State, VersionId};

/// Map a selector onto the newest *installed* instance of its id (PROP-019
/// §2.3, §2.11).
pub(super) fn resolve_installed(
    state: &State,
    selector: &model::Selector,
    raw: &str,
) -> Result<InstallRecord, VvmError> {
    use model::{Kind, Selector, VersionId};
    match selector {
        Selector::Latest => {
            latest_of(state, &VersionId::new(Kind::Branch, "main")).ok_or_else(|| {
                VvmError::NotInstalled {
                    detail: "`latest` is not installed".to_string(),
                }
            })
        }
        Selector::Explicit(id) => latest_of(state, id).ok_or_else(|| VvmError::NotInstalled {
            detail: format!("`{id}` is not installed (try `vibe self install {raw}`)"),
        }),
        Selector::Exact(exact) => state
            .installs
            .iter()
            .find(|record| {
                record.version_id() == exact.version && record.instance == exact.instance
            })
            .cloned()
            .ok_or_else(|| VvmError::NotInstalled {
                detail: format!("exact local instance `{exact}` is not installed"),
            }),
        Selector::Stable => highest_tag_record(state).ok_or_else(|| VvmError::NotInstalled {
            detail: "no installed release tag satisfies `stable`".to_string(),
        }),
        Selector::Ambiguous(name) => {
            by_precedence_record(state, name).ok_or_else(|| VvmError::NotInstalled {
                detail: format!("no installed version named `{name}`"),
            })
        }
    }
}

/// The newest instance of a version id.
fn latest_of(state: &State, id: &VersionId) -> Option<InstallRecord> {
    state
        .installs
        .iter()
        .filter(|r| &r.version_id() == id)
        .max_by_key(|r| r.instance)
        .cloned()
}

/// The newest instance of the highest installed semver tag (PROP-019 §2.3).
fn highest_tag_record(state: &State) -> Option<InstallRecord> {
    state
        .installs
        .iter()
        .filter_map(|r| {
            (r.kind == model::Kind::Tag)
                .then(|| semver::Version::parse(r.id.strip_prefix('v').unwrap_or(&r.id)).ok())
                .flatten()
                .map(|v| (v, r.instance, r))
        })
        .max_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, _, r)| r.clone())
}

/// The newest instance of a bare name, by precedence branch > tag > commit
/// (PROP-019 §2.3).
fn by_precedence_record(state: &State, name: &str) -> Option<InstallRecord> {
    for kind in [model::Kind::Branch, model::Kind::Tag, model::Kind::Commit] {
        if let Some(r) = state
            .installs
            .iter()
            .filter(|r| r.kind == kind && r.id == name)
            .max_by_key(|r| r.instance)
        {
            return Some(r.clone());
        }
    }
    None
}
