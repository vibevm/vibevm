use super::text::*;
use super::*;

fn cargo_workspace_alias_map<'a>(
    manifests: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Result<BTreeMap<String, String>, ScrapeError> {
    let mut aliases = BTreeMap::new();
    for (path, bytes) in manifests {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| fail(format!("Cargo manifest `{path}` is not UTF-8")))?;
        let document = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(|error| fail(format!("cannot parse Cargo manifest `{path}`: {error}")))?;
        let Some(dependencies) = document
            .get("workspace")
            .and_then(toml_edit::Item::as_table)
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(toml_edit::Item::as_table)
        else {
            continue;
        };
        for (alias, item) in dependencies {
            if is_workspace_inherited(item) {
                return Err(fail(format!(
                    "workspace dependency `{alias}` in `{path}` recursively inherits workspace identity"
                )));
            }
            let identity = dependency_identity(alias, item)
                .ok_or_else(|| fail(format!("workspace dependency `{alias}` has no identity")))?;
            if let Some(prior) = aliases.insert(alias.to_owned(), identity.to_owned())
                && prior != identity
            {
                return Err(fail(format!(
                    "Cargo workspace alias `{alias}` resolves to both `{prior}` and `{identity}`; workspace ownership is ambiguous"
                )));
            }
        }
    }
    Ok(aliases)
}

#[derive(Debug)]
pub(super) struct CargoManifestNode {
    pub(super) path: String,
    dir: String,
    bytes: Vec<u8>,
    has_package: bool,
    has_workspace: bool,
    workspace_members: Vec<String>,
    workspace_exclude: Vec<String>,
}

#[derive(Debug)]
pub(super) struct CargoTopology {
    manifests: BTreeMap<String, CargoManifestNode>,
    locks: BTreeSet<String>,
}

impl CargoTopology {
    pub(super) fn build<'a>(
        manifests: impl IntoIterator<Item = (&'a str, &'a [u8])>,
        locks: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, ScrapeError> {
        let mut nodes = BTreeMap::new();
        for (path, bytes) in manifests {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| fail(format!("Cargo manifest `{path}` is not UTF-8")))?;
            let document = text
                .parse::<toml_edit::DocumentMut>()
                .map_err(|error| fail(format!("cannot parse Cargo manifest `{path}`: {error}")))?;
            let dir = cargo_parent(path, "Cargo.toml")?;
            let has_package = document
                .get("package")
                .and_then(toml_edit::Item::as_table)
                .is_some();
            let workspace = document
                .get("workspace")
                .and_then(toml_edit::Item::as_table);
            let has_workspace = workspace.is_some();
            let workspace_members = cargo_workspace_paths(
                workspace.and_then(|table| table.get("members")),
                path,
                &dir,
                "members",
            )?;
            let workspace_exclude = cargo_workspace_paths(
                workspace.and_then(|table| table.get("exclude")),
                path,
                &dir,
                "exclude",
            )?;
            let node = CargoManifestNode {
                path: path.to_owned(),
                dir,
                bytes: bytes.to_vec(),
                has_package,
                has_workspace,
                workspace_members,
                workspace_exclude,
            };
            if nodes.insert(path.to_owned(), node).is_some() {
                return Err(fail(format!("duplicate Cargo manifest `{path}`")));
            }
        }
        Ok(Self {
            manifests: nodes,
            locks: locks.into_iter().map(str::to_owned).collect(),
        })
    }

    pub(super) fn workspace_root_for(
        &self,
        manifest: &str,
    ) -> Result<Option<&CargoManifestNode>, ScrapeError> {
        let node = self.manifests.get(manifest).ok_or_else(|| {
            fail(format!(
                "Cargo manifest `{manifest}` is absent from topology"
            ))
        })?;
        if node.has_workspace {
            return Ok(Some(node));
        }
        let roots = self
            .manifests
            .values()
            .filter(|candidate| candidate.has_workspace)
            .filter_map(
                |candidate| match cargo_workspace_contains(candidate, manifest) {
                    Ok(true) => Some(Ok(candidate)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
            )
            .collect::<Result<Vec<_>, ScrapeError>>()?;
        match roots.as_slice() {
            [] => Ok(None),
            [root] => Ok(Some(*root)),
            _ => Err(ScrapeError::blocked(format!(
                "Cargo manifest `{manifest}` is selected by multiple workspace roots"
            ))),
        }
    }

    pub(super) fn workspace_aliases_for(
        &self,
        manifest: &str,
    ) -> Result<BTreeMap<String, String>, ScrapeError> {
        let Some(root) = self.workspace_root_for(manifest)? else {
            return Ok(BTreeMap::new());
        };
        cargo_workspace_alias_map(std::iter::once((root.path.as_str(), root.bytes.as_slice())))
    }

    pub(super) fn owned_lock_for(&self, manifest: &str) -> Result<Option<String>, ScrapeError> {
        let node = self.manifests.get(manifest).ok_or_else(|| {
            fail(format!(
                "Cargo manifest `{manifest}` is absent from topology"
            ))
        })?;
        let owner = self.workspace_root_for(manifest)?.unwrap_or(node);
        let lock = cargo_child(&owner.dir, "Cargo.lock");
        Ok(self.locks.contains(&lock).then_some(lock))
    }

    pub(super) fn owned_locks<'a>(
        &self,
        manifests: impl IntoIterator<Item = &'a str>,
    ) -> Result<BTreeSet<String>, ScrapeError> {
        let mut result = BTreeSet::new();
        for manifest in manifests {
            if let Some(lock) = self.owned_lock_for(manifest)? {
                result.insert(lock);
            }
        }
        Ok(result)
    }

    pub(super) fn source_manifest(&self, source: &str) -> Result<&CargoManifestNode, ScrapeError> {
        let owners = self
            .manifests
            .values()
            .filter(|manifest| manifest.has_package && cargo_dir_contains(&manifest.dir, source))
            .collect::<Vec<_>>();
        let Some(longest) = owners.iter().map(|owner| owner.dir.len()).max() else {
            return Err(ScrapeError::blocked(format!(
                "Rust source `{source}` has no owning package Cargo.toml"
            )));
        };
        let nearest = owners
            .into_iter()
            .filter(|owner| owner.dir.len() == longest)
            .collect::<Vec<_>>();
        match nearest.as_slice() {
            [owner] => Ok(*owner),
            _ => Err(ScrapeError::blocked(format!(
                "Rust source `{source}` has ambiguous owning Cargo manifests"
            ))),
        }
    }
}

fn cargo_parent(path: &str, leaf: &str) -> Result<String, ScrapeError> {
    if path == leaf {
        return Ok(String::new());
    }
    path.strip_suffix(&format!("/{leaf}"))
        .map(str::to_owned)
        .ok_or_else(|| fail(format!("Cargo path `{path}` is not a `{leaf}` path")))
}

fn cargo_child(dir: &str, leaf: &str) -> String {
    if dir.is_empty() {
        leaf.to_owned()
    } else {
        format!("{dir}/{leaf}")
    }
}

fn cargo_dir_contains(dir: &str, path: &str) -> bool {
    dir.is_empty() || path.starts_with(&(dir.to_owned() + "/"))
}

fn cargo_workspace_paths(
    item: Option<&toml_edit::Item>,
    manifest: &str,
    root: &str,
    field: &str,
) -> Result<Vec<String>, ScrapeError> {
    let Some(item) = item else {
        return Ok(Vec::new());
    };
    let values = item.as_array().ok_or_else(|| {
        ScrapeError::blocked(format!(
            "Cargo workspace `{manifest}` has non-array `{field}`"
        ))
    })?;
    let mut result = Vec::new();
    for value in values {
        let relative = value.as_str().ok_or_else(|| {
            ScrapeError::blocked(format!(
                "Cargo workspace `{manifest}` has non-string `{field}` member"
            ))
        })?;
        if relative.is_empty()
            || relative.starts_with('/')
            || relative.contains('\\')
            || relative.split('/').any(|part| part == "." || part == "..")
        {
            return Err(ScrapeError::blocked(format!(
                "Cargo workspace `{manifest}` has unsupported `{field}` path `{relative}`"
            )));
        }
        let joined = cargo_child(root, relative.trim_end_matches('/'));
        let pattern = cargo_child(&joined, "Cargo.toml");
        Glob::parse(&pattern).map_err(|error| {
            ScrapeError::blocked(format!(
                "Cargo workspace `{manifest}` has unsupported `{field}` pattern `{relative}`: {error}"
            ))
        })?;
        result.push(pattern);
    }
    result.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    result.dedup();
    Ok(result)
}

fn cargo_workspace_contains(
    workspace: &CargoManifestNode,
    manifest: &str,
) -> Result<bool, ScrapeError> {
    let included = workspace
        .workspace_members
        .iter()
        .map(|pattern| Glob::parse(pattern))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|pattern| pattern.matches(manifest));
    let excluded = workspace
        .workspace_exclude
        .iter()
        .map(|pattern| Glob::parse(pattern))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|pattern| pattern.matches(manifest));
    Ok(included && !excluded)
}

pub(super) fn cargo_rule_selects_manifest(
    manifests: &[String],
    manifest: &str,
) -> Result<bool, ScrapeError> {
    manifests
        .iter()
        .map(|pattern| Glob::parse(pattern))
        .collect::<Result<Vec<_>, _>>()
        .map(|patterns| patterns.iter().any(|pattern| pattern.matches(manifest)))
}

pub(super) fn cargo_topology_from_current(
    current: &BTreeMap<String, Vec<u8>>,
    files: &BTreeSet<String>,
) -> Result<CargoTopology, ScrapeError> {
    CargoTopology::build(
        current.iter().filter_map(|(path, bytes)| {
            path.ends_with("Cargo.toml")
                .then_some((path.as_str(), bytes.as_slice()))
        }),
        files
            .iter()
            .filter(|path| path.ends_with("Cargo.lock"))
            .map(String::as_str),
    )
}

pub(super) fn observed_specmark_aliases_for_source(
    contract: &Contract,
    topology: &CargoTopology,
    source: &str,
) -> Result<BTreeSet<String>, ScrapeError> {
    let owner = topology.source_manifest(source)?;
    let matching = contract
        .rewrite
        .iter()
        .filter_map(|rule| {
            let RewriteRule::CargoPackageRemoveV1 {
                id,
                manifests,
                package,
                aliases,
                ..
            } = rule
            else {
                return None;
            };
            (package == "core-ai-native-specmark").then_some((id, manifests, package, aliases))
        })
        .filter_map(
            |row| match cargo_rule_selects_manifest(row.1, &owner.path) {
                Ok(true) => Some(Ok(row)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .collect::<Result<Vec<_>, ScrapeError>>()?;
    let (rule_id, _, package, aliases) = match matching.as_slice() {
        [only] => *only,
        [] => return Ok(BTreeSet::new()),
        _ => {
            return Err(ScrapeError::blocked(format!(
                "Rust source `{source}` is owned by `{}` but multiple Specmark Cargo removal rules claim it",
                owner.path
            )));
        }
    };
    let workspace_aliases = topology.workspace_aliases_for(&owner.path)?;
    let (_, _, _, observed, _) =
        prepare_cargo_resolved(&owner.bytes, package, aliases, &workspace_aliases).map_err(
            |error| {
                ScrapeError::blocked(format!(
                    "Cargo rule `{rule_id}` cannot prove Specmark ownership for Rust source `{source}` through `{}`: {error}",
                    owner.path
                ))
            },
        )?;
    Ok(observed
        .into_iter()
        .map(|alias| alias.replace('-', "_"))
        .collect())
}
