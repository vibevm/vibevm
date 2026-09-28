//! Draft assembly only: generated readers own the wire; review owns acceptance.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#TRANSITION-INDEX");

mod draft;
mod output;
#[cfg(test)]
mod tests;

#[cfg(test)]
use std::fs;
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};
use vibe_safefs::Project;
use vibe_wire::generated::preview::e1::{release_impact::ReleaseImpact, transition::Transition};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn bytes(project: &Project, relative: &Path) -> Result<Vec<u8>> {
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str().ok_or("input component is not UTF-8"),
            _ => Err("input has an unsafe component"),
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let (name, parents) = components.split_last().ok_or("input has no filename")?;
    let directory = project.dir(parents, false)?;
    let (_, length) = project
        .inspect_file_in(&directory, name)?
        .ok_or("input is absent")?;
    let cap = usize::try_from(length).map_err(|_| "input cannot fit the platform address space")?;
    let snapshot = project
        .read_file_snapshot_bounded_in(&directory, name, cap)?
        .ok_or("input disappeared")?;
    if snapshot.size != length {
        return Err("input size changed during observation".into());
    }
    Ok(snapshot.bytes)
}

fn records(project: &Project, transition: &Transition, specs: &Path) -> Result<Vec<ReleaseImpact>> {
    transition.validate_preview()?;
    let references = transition.capture_references()?;
    let captures: BTreeSet<_> = references
        .iter()
        .map(|reference| reference.capture_id.as_str())
        .collect();
    let mut records = Vec::new();
    for id in &transition.change_records {
        if id.is_empty()
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("invalid immutable change ID".into());
        }
        let wire = bytes(
            project,
            &specs.join("changes").join(id).join("release-impact.json"),
        )
        .map_err(|_| format!("cannot read safe release-impact record {id}"))?;
        let record: ReleaseImpact = serde_json::from_slice(&wire)
            .map_err(|_| format!("invalid generated release-impact shape for {id}"))?;
        record.validate_release_impact()?;
        if record.change_id != *id {
            return Err(format!("record identity mismatch for {id}").into());
        }
        if transition.dp2.capture_id.is_some() {
            for capture in [&record.before_capture_id, &record.after_capture_id] {
                if !captures.contains(capture.as_str()) {
                    return Err(format!("unresolved record capture for {id}").into());
                }
            }
        }
        records.push(record);
    }
    let identities: BTreeSet<_> = records.iter().map(|r| r.change_id.as_str()).collect();
    for record in &records {
        for relation in &record.relations {
            if !identities.contains(relation.change_id.as_str()) {
                return Err(format!("dangling relation in {}", record.change_id).into());
            }
        }
    }
    Ok(records)
}

fn assemble(transition_path: &Path, output_dir: &Path) -> Result<()> {
    let path = std::path::absolute(transition_path)?;
    let root = path
        .ancestors()
        .skip(1)
        .find(|dir| dir.join("vibe.toml").is_file())
        .ok_or("transition has no enclosing vibe.toml project")?;
    let project = Project::open(root)?;
    // Manifest remains configuration, never a second release-wire parser.
    let manifest =
        bytes(&project, Path::new("vibe.toml")).map_err(|_| "cannot read safe project manifest")?;
    let text = std::str::from_utf8(&manifest).map_err(|_| "project manifest is not UTF-8")?;
    let _: vibe_core::manifest::Manifest =
        toml::from_str(text).map_err(|_| "invalid project manifest shape")?;
    let wire = bytes(&project, path.strip_prefix(root)?)
        .map_err(|_| "cannot read safe transition input")?;
    let transition: Transition =
        serde_json::from_slice(&wire).map_err(|_| "invalid generated transition shape")?;
    let records = records(
        &project,
        &transition,
        &vibe_core::layout::current_specs_root(),
    )?;
    let drafts = draft::render(&transition, &records)?;
    output::write(output_dir, &drafts)?;
    Ok(())
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = if args.len() == 2 {
        assemble(Path::new(&args[0]), Path::new(&args[1]))
    } else {
        Err("usage: preview-transition-assemble <transition> <output-dir>".into())
    };
    if let Err(error) = result {
        eprintln!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-081#TRANSITION-INDEX: preview draft assembly failed: {error}; fix surface: transition, selected release-impact records, or owned output directory"
        );
        std::process::exit(1);
    }
}
