//! One authority boundary for every normalized scanner and standalone consumer.
use crate::{
    config::Config,
    generated::{specmap::Edge, specmap_authority::SpecmapAuthority},
};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};
const FILE: &str = ".vibe/specmap-authority.json";
const CAP: u64 = 4 * 1024 * 1024;
fn relative(path: &str) -> bool {
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
/// Existing roots are resolved without permitting a symlink or reparse escape.
fn safe_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut current = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                current.pop();
            }
            _ => current.push(part.as_os_str()),
        }
        match std::fs::symlink_metadata(&current) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    bail!("authority path contains a link; restore ordinary selected roots");
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        bail!("authority path contains a reparse point");
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(current)
}
fn same_file(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    if a.len() != b.len()
        || a.modified().ok() != b.modified().ok()
        || a.created().ok() != b.created().ok()
    {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (
            a.dev(),
            a.ino(),
            a.nlink(),
            a.ctime(),
            a.ctime_nsec(),
            a.mtime_nsec(),
        ) != (
            b.dev(),
            b.ino(),
            b.nlink(),
            b.ctime(),
            b.ctime_nsec(),
            b.mtime_nsec(),
        ) {
            return false;
        }
    }
    true
}
fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    use std::io::{Read, Seek, SeekFrom};
    let path = safe_path(path)?;
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if !meta.is_file() || meta.len() > CAP {
        bail!("authority input is not a bounded ordinary file");
    }
    let mut file = std::fs::File::open(&path)?;
    if !same_file(&file.metadata()?, &meta) {
        bail!("authority input changed during admission");
    }
    let mut bytes = Vec::new();
    file.by_ref().take(CAP + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > CAP || !same_file(&file.metadata()?, &meta) {
        bail!("authority input changed or exceeds budget");
    }
    file.seek(SeekFrom::Start(0))?;
    let mut again = Vec::new();
    file.by_ref().take(CAP + 1).read_to_end(&mut again)?;
    if again != bytes || !same_file(&file.metadata()?, &meta) {
        bail!("authority input changed between bounded reads");
    }
    safe_path(&path)?;
    if !same_file(&std::fs::symlink_metadata(&path)?, &meta) {
        bail!("authority input name was replaced during admission");
    }
    Ok(Some(bytes))
}
fn vibe_project(root: &Path) -> Result<bool> {
    let root = safe_path(root)?;
    for ancestor in root.ancestors() {
        match std::fs::symlink_metadata(ancestor.join("vibe.toml")) {
            Ok(_) => return Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(false)
}
pub fn context(root: &Path) -> Result<Option<SpecmapAuthority>> {
    if !vibe_project(root)? {
        return Ok(None);
    }
    let bytes=read(&root.join(FILE))?.context("Vibe specification authority is missing; fix: run `vibe install --path <project>` to regenerate it")?;
    // Foreign-owned schemas retain the generator's public derives. Validate the
    // closed wire shape before deserializing its one canonical generated DTO.
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    fn keys(value: &serde_json::Value, allowed: &[&str]) -> Result<()> {
        let map = value.as_object().context("authority object required")?;
        if map.len() != allowed.len() || map.keys().any(|k| !allowed.contains(&k.as_str())) {
            bail!("unknown or missing authority wire member; regenerate authority");
        }
        Ok(())
    }
    keys(
        &value,
        &[
            "schema",
            "workspace_root",
            "selected_node",
            "self_coordinate",
            "self_namespace",
            "node_roots",
            "inputs",
            "dependencies",
        ],
    )?;
    for input in value["inputs"]
        .as_array()
        .context("authority inputs required")?
    {
        keys(input, &["root", "path", "sha256"])?;
    }
    for dep in value["dependencies"]
        .as_array()
        .context("authority dependencies required")?
    {
        keys(dep, &["coordinate", "version", "root", "spec_roots"])?;
    }
    let ctx: SpecmapAuthority = serde_json::from_value(value)?;
    let workspace = safe_path(Path::new(&ctx.workspaceRoot))?;
    if ctx.schema != 1
        || (!relative(&ctx.selectedNode) && ctx.selectedNode != ".")
        || safe_path(&workspace.join(&ctx.selectedNode))? != safe_path(root)?
    {
        bail!(
            "authority belongs to another selected node or schema; fix: run `vibe install --path <project>`"
        );
    }
    let mut coordinates = BTreeSet::new();
    for dep in &ctx.dependencies {
        let at = safe_path(Path::new(&dep.root))?;
        if !Path::new(&dep.root).is_absolute()
            || dep.version.is_empty()
            || !coordinates.insert(&dep.coordinate)
        {
            bail!("duplicate or invalid admitted dependency identity");
        }
        for spec in &dep.specRoots {
            if !safe_path(Path::new(spec))?.starts_with(&at) {
                bail!("dependency spec root escaped its admitted owner");
            }
        }
    }
    let dep_roots: BTreeSet<_> = ctx
        .dependencies
        .iter()
        .map(|d| safe_path(Path::new(&d.root)))
        .collect::<Result<_>>()?;
    let mut required = BTreeSet::from([
        (workspace.clone(), "vibe.toml".to_owned()),
        (workspace.clone(), "vibe.lock".to_owned()),
        (safe_path(root)?, "vibe.toml".to_owned()),
        (safe_path(root)?, Config::REL_PATH.to_owned()),
    ]);
    required.extend(dep_roots.iter().flat_map(|r| {
        [
            (r.clone(), "vibe.toml".to_owned()),
            (r.clone(), Config::REL_PATH.to_owned()),
        ]
    }));
    let mut seen = BTreeSet::new();
    for input in &ctx.inputs {
        let at = safe_path(Path::new(&input.root))?;
        if !relative(&input.path)
            || (!at.starts_with(&workspace) && !dep_roots.contains(&at))
            || !seen.insert((at.clone(), input.path.clone()))
        {
            bail!("authority input locator is invalid; regenerate through ordinary install");
        }
        let actual = read(&at.join(&input.path))?.map(|b| format!("{:x}", Sha256::digest(&b)));
        if actual.as_deref() != input.sha256.as_deref().map(|s| s.as_str()) {
            bail!(
                "Vibe specification authority is stale at {}; fix: run `vibe install --path <project>`",
                input.path
            );
        }
    }
    if !required.is_subset(&seen) {
        bail!(
            "authority omits required manifest/lock fingerprints; regenerate through ordinary install"
        );
    }
    Ok(Some(ctx))
}
/// Validate configured roots before any specification or external scan.
pub(crate) fn validate(root: &Path, cfg: &Config) -> Result<()> {
    crate::local_layers::LocalLayers::current(root).validate(root, cfg)?;
    let project = safe_path(root)?;
    for path in cfg.spec_roots.iter().chain(&cfg.root_spec_docs) {
        let target = safe_path(&root.join(path))?;
        if !target.starts_with(&project) {
            bail!(
                "spec://{}: project inventory root is outside its owner; fix: use a declared dependency in external_specs",
                cfg.namespace
            );
        }
    }
    let ctx = context(root)?;
    if ctx
        .as_ref()
        .is_some_and(|ctx| ctx.selfNamespace != cfg.namespace)
    {
        bail!(
            "own namespace differs from admitted specmap configuration; fix: run `vibe install --path <project>`"
        );
    }
    let mut namespaces = BTreeSet::from([cfg.namespace.as_str()]);
    let names: Vec<_> = std::iter::once(cfg.namespace.as_str())
        .chain(cfg.external_specs.iter().map(|e| e.namespace.as_str()))
        .collect();
    for (i, a) in names.iter().enumerate() {
        for b in names.iter().skip(i + 1) {
            if a.strip_prefix(b).is_some_and(|t| t.starts_with('/'))
                || b.strip_prefix(a).is_some_and(|t| t.starts_with('/'))
            {
                bail!(
                    "overlapping authority namespaces `{a}` and `{b}`; fix: choose distinct explicit namespace aliases"
                );
            }
        }
    }

    for ext in &cfg.external_specs {
        if ext.namespace.is_empty() || !namespaces.insert(&ext.namespace) {
            bail!(
                "spec://{}: ambiguous authority namespace; fix: use distinct explicit aliases",
                ext.namespace
            );
        }
        let target = safe_path(&root.join(&ext.root))?;
        if let Some(ctx) = &ctx {
            let admitted = ctx.dependencies.iter().any(|dep| {
                dep.specRoots
                    .iter()
                    .any(|r| safe_path(Path::new(r)).is_ok_and(|r| r == target))
            });
            if !admitted {
                bail!(
                    "spec://{}: undeclared external authority; fix: declare/install the dependency in this selected Vibe node",
                    ext.namespace
                );
            }
        }
    }
    Ok(())
}
/// Every Rust/TS/JTD scope or direct annotation reaches this normalized edge gate.
pub(crate) fn validate_edges(root: &Path, cfg: &Config, edges: &[Edge]) -> Result<()> {
    validate(root, cfg)?;
    let authorities: BTreeMap<_, _> = std::iter::once((cfg.namespace.as_str(), "own"))
        .chain(
            cfg.external_specs
                .iter()
                .map(|r| (r.namespace.as_str(), "declared external")),
        )
        .collect();
    for edge in edges {
        if edge.uri.starts_with("spec://") {
            specmark_grammar::parse_spec_uri(&edge.uri)
                .map_err(|error| anyhow::anyhow!("reference `{}`: {error}", edge.uri))?;
            let uri = edge.uri.strip_prefix("spec://").expect("checked scheme");
            if !authorities.keys().any(|ns| {
                uri.strip_prefix(*ns)
                    .is_some_and(|tail| tail.starts_with('/'))
            }) {
                bail!(
                    "{}:{}: reference `{}` targets an unknown/undeclared namespace or private local layer; fix: cite own specifications or an admitted declared dependency (never personal/override boot)",
                    edge.file,
                    edge.line,
                    edge.uri
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::specmap_authority::{Dependency, Input};
    fn config() -> Config {
        Config {
            namespace: "org.test/project".into(),
            scan_roots: vec![],
            spec_roots: vec![],
            ..Config::default()
        }
    }
    fn context_file(root: &Path, dep: &Path) {
        std::fs::create_dir_all(root.join(".vibe")).unwrap();
        let bytes = std::fs::read(root.join("vibe.toml")).unwrap();
        let input = |at: &Path, path: &str, hash: Option<String>| Input {
            root: at.to_string_lossy().into(),
            path: path.into(),
            sha256: hash.map(Box::new),
        };
        let ctx = SpecmapAuthority {
            schema: 1,
            workspaceRoot: root.to_string_lossy().into(),
            selectedNode: ".".into(),
            selfCoordinate: "org.test/project".into(),
            selfNamespace: "org.test/project".into(),
            nodeRoots: std::collections::HashMap::new(),
            inputs: vec![
                input(
                    root,
                    "vibe.toml",
                    Some(format!("{:x}", Sha256::digest(bytes))),
                ),
                input(root, "vibe.lock", None),
                input(root, Config::REL_PATH, None),
                input(dep, "vibe.toml", None),
                input(dep, Config::REL_PATH, None),
            ],
            dependencies: vec![Dependency {
                coordinate: "org.test/dep".into(),
                version: "1.0.0".into(),
                root: dep.to_string_lossy().into(),
                specRoots: vec![dep.join("specs").to_string_lossy().into()],
            }],
        };
        std::fs::write(root.join(FILE), serde_json::to_vec(&ctx).unwrap()).unwrap();
    }
    #[test]
    fn generic_external_api_remains_explicit_but_namespace_collisions_refuse() {
        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let mut cfg = config();
        cfg.external_specs.push(crate::config::ExternalSpec {
            namespace: "alias".into(),
            root: external.path().to_string_lossy().into(),
        });
        validate(root.path(), &cfg).unwrap();
        cfg.external_specs[0].namespace = cfg.namespace.clone();
        assert!(validate(root.path(), &cfg).is_err());
    }
    #[test]
    fn vibe_requires_context_alias_is_bound_to_admitted_root_and_absence_fingerprint_matters() {
        let root = tempfile::tempdir().unwrap();
        let dep = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("vibe.toml"),
            "[project]\nname='demo'\nversion='1.0.0'\n",
        )
        .unwrap();
        let mut cfg = config();
        assert!(
            validate(root.path(), &cfg)
                .unwrap_err()
                .to_string()
                .contains("authority is missing")
        );
        context_file(root.path(), dep.path());
        cfg.external_specs.push(crate::config::ExternalSpec {
            namespace: "alias-not-coordinate".into(),
            root: dep.path().join("specs").to_string_lossy().into(),
        });
        validate(root.path(), &cfg).unwrap();
        cfg.external_specs[0].root = dep.path().join("other").to_string_lossy().into();
        assert!(validate(root.path(), &cfg).is_err());
        cfg.external_specs.clear();
        std::fs::write(root.path().join("vibe.lock"), "new lock").unwrap();
        assert!(
            validate(root.path(), &cfg)
                .unwrap_err()
                .to_string()
                .contains("stale")
        );
    }
    #[test]
    fn normalized_direct_unknown_reference_is_refused_without_private_scan() {
        let root = tempfile::tempdir().unwrap();
        let cfg = config();
        let edges:Vec<Edge>=serde_json::from_value(serde_json::json!([{"file":"src/lib.rs","line":1,"from_symbol":"demo","provenance":"authored","verb":"implements","uri":"spec://org.private/memory/common/PREF#x"}])).unwrap();
        let error = validate_edges(root.path(), &cfg, &edges)
            .unwrap_err()
            .to_string();
        assert!(error.contains("org.private/memory"));
        assert!(error.contains("private local layer"));
    }
}
