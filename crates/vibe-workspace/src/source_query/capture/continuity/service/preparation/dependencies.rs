//! Lock-ordered captured closure metadata, then visible payload capture.
use super::*;
use crate::extension_world::ExtensionWorldEpoch;
use crate::install::ResolvedDep;
use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, LockedPackage, Lockfile, Materialization};

pub(super) struct Dependencies {
    pub(super) epoch: ExtensionWorldEpoch,
    pub(super) captures: Vec<PreparedOwnerCapture>,
    pub(super) records: BTreeMap<(String, String), StableFileSnapshot>,
    pub(super) lock: Option<StableFileSnapshot>,
}

pub(super) fn prepare(
    root: &Path,
    node_root: &Path,
    node: &Manifest,
) -> Result<Dependencies, QueryError> {
    let project = Project::open(root).map_err(|_| unavailable("held workspace root"))?;
    let observed = project
        .read_file_snapshot_bounded("vibe.lock", 8 * 1024 * 1024)
        .map_err(|_| unavailable("held selected lock"))?;
    let lock = match &observed {
        Some(snapshot) => {
            let text =
                std::str::from_utf8(&snapshot.bytes).map_err(|_| unavailable("lock UTF-8"))?;
            let lock: Lockfile = toml::from_str(text).map_err(|_| unavailable("lock grammar"))?;
            if lock.meta.schema_version != CURRENT_SCHEMA_VERSION {
                return Err(unavailable("lock epoch"));
            }
            lock
        }
        None => Lockfile::empty("source query", "unavailable"),
    };
    // The lock supplies exact selected rows; it cannot override the effective
    // node's authored registry admission constraint. Workspace version vars
    // have already been resolved by the incumbent finalizer.
    for requirement in &node.requires.packages {
        let group = requirement
            .group
            .as_ref()
            .ok_or_else(|| unavailable("qualified selected requirement"))?;
        let row = locked(&lock, &(group.to_string(), requirement.name.to_string()))?;
        if requirement.kind.is_some_and(|kind| kind != row.kind) {
            return Err(unavailable("selected requirement kind"));
        }
        if !requirement.version.matches(&row.version) {
            return Err(unavailable("selected requirement version"));
        }
    }
    let mut queue = node
        .requires
        .iter_pkgrefs()
        .map(|(group, name)| {
            group
                .map(|group| (group.to_string(), name.to_owned()))
                .ok_or_else(|| unavailable("qualified selected requirement"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut selected = BTreeSet::new();
    while let Some(key) = queue.pop() {
        if !selected.insert(key.clone()) {
            continue;
        }
        if selected.len() > 256 {
            return Err(unavailable("selected closure bound"));
        }
        let row = locked(&lock, &key)?;
        for edge in &row.dependencies {
            let group = edge
                .group
                .as_ref()
                .ok_or_else(|| unavailable("qualified locked edge"))?;
            queue.push((group.to_string(), edge.name.to_string()));
        }
    }
    // Only closure manifests enter this first phase. A private package's
    // payload cannot poison the point or become an unauthorized read.
    let mut resolved = Vec::new();
    let mut metadata = Vec::new();
    let mut retained = Vec::new();
    for row in &lock.packages {
        let key = (row.group.to_string(), row.name.to_string());
        if !selected.contains(&key) {
            continue;
        }
        let slot = match row.materialization {
            Materialization::InPlace => {
                crate::vibedeps::in_place_slot_abs_path(root, &row.group, row.name.as_str())
            }
            _ => crate::vibedeps::slot_abs_path(root, &row.group, row.name.as_str(), &row.version),
        };
        let project = Project::open(&slot).map_err(|_| unavailable("selected slot root"))?;
        let manifest_snapshot = snapshot(&project, "vibe.toml", MANIFEST_CAP)?;
        let manifest = membership::parse_manifest(&manifest_snapshot)?;
        let package = manifest
            .package
            .as_ref()
            .ok_or_else(|| unavailable("selected package identity"))?;
        if package.group != row.group
            || package.name != row.name.as_str()
            || package.version != row.version
            || package.kind != row.kind
            || package.materialization != row.materialization
        {
            return Err(unavailable("selected slot/lock identity"));
        }
        let requires = row
            .dependencies
            .iter()
            .map(|edge| {
                edge.group
                    .as_ref()
                    .map(|group| (group.clone(), edge.name.to_string()))
                    .ok_or_else(|| unavailable("qualified locked edge"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        resolved.push(ResolvedDep {
            kind: row.kind,
            group: row.group.clone(),
            name: row.name.to_string(),
            version: row.version.clone(),
            content_dir: slot.clone(),
            source_hash: Some(row.content_hash.clone()),
            embedded_sources: row.embedded_sources.clone(),
            manifest,
            requires,
            admitted_by: row.admitted_by.clone(),
            via_override: row.via_override.clone(),
            source_mutable: row.materialization.is_in_place(),
            in_place_changed: None,
        });
        metadata.push(PreparedOwnerCapture::dependency(
            &key.0,
            &key.1,
            slot.clone(),
            manifest_snapshot.clone(),
            BTreeMap::new(),
            None,
        ));
        retained.push((row.clone(), slot, manifest_snapshot));
    }
    let epoch = ExtensionWorldEpoch::from_resolution(root, &resolved)
        .map_err(|_| unavailable("selected ordered resolution"))?;
    let visibility =
        crate::source_query::visibility::owner_visibility(&epoch, node_root, node, &metadata)
            .map_err(world_error)?;
    let mut captures = Vec::new();
    let mut records = BTreeMap::new();
    let mut remaining = TOTAL_CAP;
    for (row, slot, manifest_snapshot) in retained {
        let key = (row.group.to_string(), row.name.to_string());
        if !visibility
            .effective
            .contains_key(&format!("{}/{}", key.0, key.1))
        {
            captures.push(PreparedOwnerCapture::dependency(
                &key.0,
                &key.1,
                slot,
                manifest_snapshot,
                BTreeMap::new(),
                None,
            ));
            continue;
        }
        let project = Project::open(&slot).map_err(|_| unavailable("visible slot root"))?;
        let map = crate::vibedeps::read_validated_descriptor_map_for_citation(
            &slot,
            &row.group,
            row.name.as_str(),
            &row.version,
            &row.content_hash,
            !row.materialization.is_in_place(),
        )
        .map_err(|_| unavailable("selected descriptor/output map"))?;
        let mut outputs = BTreeMap::new();
        if let Some(map) = &map {
            let retained_record = snapshot(
                &project,
                crate::vibedeps::SLOT_RECORD_FILENAME,
                8 * 1024 * 1024,
            )?;
            let record = crate::vibedeps::read_slot_record_bytes(
                &slot.join(crate::vibedeps::SLOT_RECORD_FILENAME),
                &retained_record.bytes,
            )
            .map_err(|_| unavailable("selected record grammar"))?;
            for output in record
                .files
                .iter()
                .filter(|file| map.output_row(&file.path).is_some())
            {
                let held = snapshot(&project, &output.path, FILE_CAP.min(remaining))?;
                remaining = remaining
                    .checked_sub(held.bytes.len())
                    .ok_or_else(|| unavailable("capture byte bound"))?;
                outputs.insert(output.path.clone(), held);
            }
            if outputs.is_empty() {
                return Err(unavailable("selected output capture"));
            }
            records.insert(key.clone(), retained_record);
        } else {
            outputs.insert("vibe.toml".into(), manifest_snapshot.clone());
        }
        captures.push(PreparedOwnerCapture::dependency(
            &key.0,
            &key.1,
            slot,
            manifest_snapshot,
            outputs,
            map,
        ));
    }
    Ok(Dependencies {
        epoch,
        captures,
        records,
        lock: observed,
    })
}

fn locked<'a>(lock: &'a Lockfile, key: &(String, String)) -> Result<&'a LockedPackage, QueryError> {
    let matches = lock
        .packages
        .iter()
        .filter(|row| row.group.as_str() == key.0 && row.name.as_str() == key.1)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [row] => Ok(*row),
        [] => Err(unavailable("missing selected lock row")),
        _ => Err(unavailable("duplicate selected lock row")),
    }
}
