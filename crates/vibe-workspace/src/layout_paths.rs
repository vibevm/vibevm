//! Forward-slashed projections of the live project layout.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-052#ONE-LAYOUT-MODULE");

use std::path::{Path, PathBuf};

use vibe_core::layout;

fn with_tail(root: PathBuf, tail: impl AsRef<Path>) -> PathBuf {
    let tail = tail.as_ref();
    if tail.as_os_str().is_empty() {
        root
    } else {
        root.join(tail)
    }
}

fn slash(path: &Path) -> String {
    crate::path_to_slash(path)
}

pub(crate) fn specs_path(tail: impl AsRef<Path>) -> PathBuf {
    with_tail(layout::current_specs_root(), tail)
}

pub(crate) fn specs(tail: impl AsRef<Path>) -> String {
    slash(&specs_path(tail))
}

#[cfg(test)]
pub(crate) fn packages_path(tail: impl AsRef<Path>) -> PathBuf {
    with_tail(layout::current_packages_root(), tail)
}

#[cfg(test)]
pub(crate) fn packages(tail: impl AsRef<Path>) -> String {
    slash(&packages_path(tail))
}

pub(crate) fn vibedeps_path(tail: impl AsRef<Path>) -> PathBuf {
    with_tail(layout::current_vibedeps_root(), tail)
}

pub(crate) fn vibedeps(tail: impl AsRef<Path>) -> String {
    slash(&vibedeps_path(tail))
}

#[cfg(test)]
pub(crate) fn vibefacts_path(tail: impl AsRef<Path>) -> PathBuf {
    with_tail(layout::current_vibefacts_root(), tail)
}

pub(crate) fn boot_path(tail: impl AsRef<Path>) -> PathBuf {
    with_tail(layout::current_boot_dir(), tail)
}

pub(crate) fn boot(tail: impl AsRef<Path>) -> String {
    slash(&boot_path(tail))
}

pub(crate) fn slot_specs_path(slot: impl AsRef<Path>, tail: impl AsRef<Path>) -> PathBuf {
    with_tail(slot.as_ref().join(layout::current_specs_root()), tail)
}

pub(crate) fn slot_specs(slot: impl AsRef<Path>, tail: impl AsRef<Path>) -> String {
    slash(&slot_specs_path(slot, tail))
}
