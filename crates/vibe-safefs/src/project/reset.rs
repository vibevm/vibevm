//! Capability-relative empty/recreate of one engine-owned directory.

use std::ffi::OsString;

use anyhow::{Context, Result};
use cap_fs_ext::DirExt;

use super::{Pinned, Project};
use crate::component::split_relative;
use crate::file::{cap_options, verify_regular_single_link};

impl Project {
    /// Empty one project-relative directory and return its retained
    /// capability, creating it when absent. Every existing ancestor and entry
    /// is opened no-follow; links, reparse points and hard-linked files refuse.
    pub fn reset_dir(&self, relative: &str) -> Result<Pinned> {
        let (parents, name) = split_relative(relative)?;
        let root = self.root_dir()?;
        let mut created = Vec::new();
        let parent = if parents.is_empty() {
            root
        } else {
            let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
            self.dir_at_recording(&root, &chain, &mut created)?
        };
        match parent.open_child_checked(&name) {
            Ok(Some(directory)) => {
                clear(&directory)?;
                Ok(directory)
            }
            Ok(None) => parent.ensure_child(&name),
            Err(directory_error) => match self.remove_file_in(&parent, &name) {
                Ok(true) => parent.ensure_child(&name),
                Ok(false) => Err(directory_error),
                Err(error) => Err(error.context(format!(
                    "resetting `{}` after it refused as a directory: {directory_error:#}",
                    parent.join(&name).display()
                ))),
            },
        }
    }
}

fn clear(directory: &Pinned) -> Result<()> {
    let names: Vec<OsString> = directory
        .dir
        .entries()
        .with_context(|| format!("listing `{}`", directory.path.display()))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<std::io::Result<_>>()?;
    for name in names {
        let display = directory.path.join(&name);
        match directory.dir.open_dir_nofollow(&name) {
            Ok(child) => {
                let child = Pinned {
                    dir: child,
                    path: display.clone(),
                };
                clear(&child)?;
                drop(child);
                directory
                    .dir
                    .remove_dir(&name)
                    .with_context(|| format!("removing directory `{}`", display.display()))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                let mut options = cap_options();
                let file = match directory.dir.open_with(&name, options.read(true)) {
                    Ok(file) => file.into_std(),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => {
                        return Err(anyhow::Error::new(error).context(format!(
                            "opening `{}` without following links while resetting its parent",
                            display.display()
                        )));
                    }
                };
                verify_regular_single_link(&file, &display)?;
                drop(file);
                directory
                    .dir
                    .remove_file(&name)
                    .with_context(|| format!("removing file `{}`", display.display()))?;
            }
        }
    }
    Ok(())
}
