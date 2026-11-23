//! `vibe prompt init`: opt a node in to the system prompt by writing its
//! authored template, and nothing else.
//!
//! The opt-in is the presence of the authored `SYSTEM-TEMPLATE.md`
//! (`#SYS-OPT-IN`), so the command that opts a project in is the command
//! that writes that file. It writes a minimal frame naming the project and
//! builds nothing: the prefix, the lane and the subagent types are the next
//! build's, which the hint names. An existing template is the user's own
//! text and is never overwritten, so the command refuses rather than
//! offering a `--force`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPT-IN");

use std::io::Write;

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::manifest::Manifest;
use vibe_wire::generated::prompt_init as wire;

use crate::output;
use crate::output::language::{Block, Glyph, Meta, hint};

use super::Selected;

/// The protocol tag `vibe-prompt-init/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-init/1";

/// Write the selected node's authored template and report it.
///
/// The file is opened create-new, so a template that appears between the
/// existence check and the write is refused rather than overwritten.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPT-IN")]
pub(crate) fn run(ctx: &output::Context, selected: &Selected) -> Result<()> {
    let template = selected
        .node_dir
        .join(vibe_core::layout::current_boot_system_template());
    let shown = workspace_relative(selected);
    if template.exists() {
        bail!(exists(&shown));
    }
    let body = frame(&selected.manifest).ok_or_else(|| anyhow::anyhow!(uncoordinated(selected)))?;
    let boot = template
        .parent()
        .context("the template path has no parent directory")?;
    std::fs::create_dir_all(boot).with_context(|| format!("creating `{}`", boot.display()))?;
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&template)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => bail!(exists(&shown)),
        Err(error) => {
            return Err(error).with_context(|| format!("writing `{}`", template.display()));
        }
    };
    // A partial template would opt the project in with a broken frame and
    // make every rerun refuse, so a failed write takes its file with it.
    if let Err(error) = file
        .write_all(body.as_bytes())
        .and_then(|()| file.sync_all())
    {
        drop(file);
        let _ = std::fs::remove_file(&template);
        return Err(error).with_context(|| format!("writing `{}`", template.display()));
    }
    if ctx.is_json() {
        ctx.emit_json(&wire::PromptInitReport {
            protocol: PROTOCOL.to_owned(),
            node: selected.node_rel.clone(),
            template: shown,
            created: true,
        })?;
    } else {
        let block = Block::new(Glyph::Done, "opted in to the system prompt")
            .with_meta(vec![Meta::note(shown)])
            .render();
        ctx.block(&format!(
            "{block}\n  {}",
            hint("edit the frame, then run vibe prompt build (vibe install and vibe agent build it too)")
        ));
    }
    Ok(())
}

/// The minimal frame: one paragraph naming the project, wrapped as the
/// authored text it becomes, ending with one newline. `None` when the
/// manifest declares neither `[project]` nor `[package]`, so there is no
/// name to put in it.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPT-IN")]
pub(crate) fn frame(manifest: &Manifest) -> Option<String> {
    Some(format!(
        "You are working inside {}, a vibevm-managed project. The\n\
         project's own instructions and its compiled boot lane follow this frame;\n\
         treat them as the project's standing rules.\n",
        coordinate(manifest)?
    ))
}

/// `group/name` from whichever section the node's manifest declares; a
/// project without a group is named by its name alone.
fn coordinate(manifest: &Manifest) -> Option<String> {
    if let Some(package) = &manifest.package {
        return Some(format!("{}/{}", package.group, package.name));
    }
    let project = manifest.project.as_ref()?;
    Some(match &project.group {
        Some(group) => format!("{group}/{}", project.name),
        None => project.name.clone(),
    })
}

/// The template's path as the workspace spells it, with forward slashes.
fn workspace_relative(selected: &Selected) -> String {
    let path = vibe_core::layout::current_boot_system_template();
    let path = if selected.node_rel == "." {
        path
    } else {
        std::path::Path::new(&selected.node_rel).join(path)
    };
    vibe_workspace::path_to_slash(&path)
}

/// The refusal for a template that is already there.
fn exists(shown: &str) -> String {
    format!(
        "template_exists: `{shown}` already exists; this project is already opted in \
         (violates spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPT-IN; fix: edit the \
         file, or run vibe prompt build)"
    )
}

/// The refusal for a node whose manifest names nothing the frame could name.
fn uncoordinated(selected: &Selected) -> String {
    format!(
        "no_coordinate: the `vibe.toml` of node `{}` declares neither [project] nor [package], \
         so there is no project to name in the frame (violates \
         spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPT-IN; fix: declare a [project] \
         with a group and a name, then run vibe prompt init again)",
        selected.node_rel
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_names_the_project_and_ends_with_one_newline() {
        let mut manifest = Manifest::new_project("demo", "0.1.0");
        if let Some(project) = manifest.project.as_mut() {
            project.group = Some("org.example".parse().unwrap());
        }
        assert_eq!(
            frame(&manifest).unwrap(),
            "You are working inside org.example/demo, a vibevm-managed project. The\n\
             project's own instructions and its compiled boot lane follow this frame;\n\
             treat them as the project's standing rules.\n"
        );
    }

    #[test]
    fn a_package_node_is_named_by_its_package_coordinate() {
        let manifest = Manifest::parse_str(
            "[package]\ngroup = \"org.demo\"\nname = \"member\"\nkind = \"flow\"\n\
             version = \"0.1.0\"\n",
        )
        .unwrap();
        assert!(
            frame(&manifest)
                .unwrap()
                .starts_with("You are working inside org.demo/member, a vibevm-managed project.")
        );
    }

    #[test]
    fn a_project_without_a_group_is_named_by_its_name_alone() {
        let manifest = Manifest::new_project("demo", "0.1.0");
        assert!(
            frame(&manifest)
                .unwrap()
                .starts_with("You are working inside demo, a vibevm-managed project.")
        );
    }

    #[test]
    fn a_manifest_with_neither_section_has_no_frame() {
        let mut manifest = Manifest::new_project("demo", "0.1.0");
        manifest.project = None;
        assert_eq!(frame(&manifest), None);
    }
}
