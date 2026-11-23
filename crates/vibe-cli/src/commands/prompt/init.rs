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
    file.write_all(frame(&selected.manifest).as_bytes())
        .and_then(|()| file.sync_all())
        .with_context(|| format!("writing `{}`", template.display()))?;
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
/// authored text it becomes, ending with one newline.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPT-IN")]
pub(crate) fn frame(manifest: &Manifest) -> String {
    format!(
        "You are working inside {}, a vibevm-managed project. The\n\
         project's own instructions and its compiled boot lane follow this frame;\n\
         treat them as the project's standing rules.\n",
        coordinate(manifest)
    )
}

/// `group/name` from whichever section the node's manifest declares; a
/// project without a group is named by its name alone.
fn coordinate(manifest: &Manifest) -> String {
    if let Some(package) = &manifest.package {
        return format!("{}/{}", package.group, package.name);
    }
    match &manifest.project {
        Some(project) => match &project.group {
            Some(group) => format!("{group}/{}", project.name),
            None => project.name.clone(),
        },
        None => "this project".to_owned(),
    }
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
         file, or run `vibe prompt build`)"
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
            frame(&manifest),
            "You are working inside org.example/demo, a vibevm-managed project. The\n\
             project's own instructions and its compiled boot lane follow this frame;\n\
             treat them as the project's standing rules.\n"
        );
    }
}
