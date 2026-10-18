//! Immutable scenario intent. This module neither resolves nor reads sources.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#HYPOTHETICAL-SCENARIOS");
use super::hypothetical::{HostCapture, digest};
use serde::{Deserialize, Serialize};
use vibe_core::{PackageRef, manifest::LinkType};
use vibe_wire::generated::consumer_configuration::e1::request::Request;

/// Explicit selections for one world. Configuration definitions are supplied
/// by the captured catalog owner, never by a transport-authored replacement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selections {
    pub features: Vec<String>,
    pub no_defaults: bool,
    pub all_features: bool,
    pub configuration: Option<Request>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    Add {
        package: String,
        loading: Option<LinkType>,
    },
    Remove {
        coordinate: String,
    },
    Toggle {
        package: String,
        enabled: bool,
        loading: Option<LinkType>,
    },
    /// Replace explicit consumer choices, including continuity-group choices.
    /// No omitted mandatory alternative is selected by this operation.
    Reconfigure {
        selections: Selections,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub baseline_selections: Selections,
    pub edits: Vec<Edit>,
}

pub struct Candidate {
    pub host: HostCapture,
    pub selections: Selections,
    pub draft_identity: String,
}
impl Draft {
    /// Derive a new host intent in memory. Captured baseline bytes and identity
    /// remain untouched; the derived identity binds the exact draft and target.
    pub fn candidate(&self, baseline: &HostCapture, target: &str) -> Result<Candidate, String> {
        if !matches!(
            target
                .parse::<vibe_core::manifest::WhenCondition>()
                .map_err(|e| e.to_string())?,
            vibe_core::manifest::WhenCondition::Os(_)
        ) {
            return Err("scenario target must be an explicit os condition".into());
        }
        let mut host = baseline.clone();
        let mut selections = self.baseline_selections.clone();
        for edit in &self.edits {
            match edit {
                Edit::Add { package, loading } => add(&mut host, package, *loading)?,
                Edit::Remove { coordinate } => remove(&mut host, coordinate)?,
                Edit::Toggle {
                    package,
                    enabled,
                    loading,
                } => {
                    let reference = qualified(package)?;
                    let coordinate = reference.qualified_name();
                    let present = host
                        .manifest
                        .requires
                        .packages
                        .iter()
                        .any(|p| p.qualified_name() == coordinate);
                    if *enabled {
                        if present {
                            let existing = host
                                .manifest
                                .requires
                                .packages
                                .iter()
                                .find(|p| p.qualified_name() == coordinate)
                                .expect("checked root");
                            if existing != &reference {
                                return Err(
                                    "toggle cannot silently change an existing root constraint"
                                        .into(),
                                );
                            }
                            if let Some(link) = loading {
                                host.manifest.requires.links.insert(coordinate, *link);
                            }
                        } else {
                            add(&mut host, package, *loading)?;
                        }
                    } else if present {
                        remove(&mut host, &coordinate)?;
                    }
                }
                Edit::Reconfigure {
                    selections: requested,
                } => selections = requested.clone(),
            }
        }
        let draft_identity = digest(
            &serde_json::to_vec(&("boot-scenario-draft/1", &baseline.id, target, self))
                .map_err(|e| e.to_string())?,
        );
        // Keep the retained source capture ID: this is derived intent over that
        // inventory, not a claim that candidate manifest bytes existed on disk.
        Ok(Candidate {
            host,
            selections,
            draft_identity,
        })
    }
}
fn qualified(raw: &str) -> Result<PackageRef, String> {
    let p = PackageRef::parse(raw).map_err(|e| e.to_string())?;
    if p.group.is_none() {
        return Err("scenario package must be group-qualified".into());
    }
    Ok(p)
}
fn add(host: &mut HostCapture, package: &str, loading: Option<LinkType>) -> Result<(), String> {
    let p = qualified(package)?;
    let coordinate = p.qualified_name();
    if host
        .manifest
        .requires
        .packages
        .iter()
        .any(|p| p.qualified_name() == coordinate)
    {
        return Err(format!("scenario root already declared: {coordinate}"));
    }
    host.manifest.requires.packages.push(p);
    if let Some(link) = loading {
        host.manifest.requires.links.insert(coordinate, link);
    }
    Ok(())
}
fn remove(host: &mut HostCapture, coordinate: &str) -> Result<(), String> {
    let p = qualified(coordinate)?;
    if p.qualified_name() != coordinate {
        return Err("remove requires a coordinate without a version constraint".into());
    }
    let roots = &mut host.manifest.requires.packages;
    let before = roots.len();
    roots.retain(|p| p.qualified_name() != coordinate);
    if roots.len() == before {
        return Err(format!("scenario root not declared: {coordinate}"));
    }
    let requires = &mut host.manifest.requires;
    requires.links.remove(coordinate);
    requires.accesses.remove(coordinate);
    requires.friend_flags.remove(coordinate);
    requires.excludes.remove(coordinate);
    Ok(())
}
