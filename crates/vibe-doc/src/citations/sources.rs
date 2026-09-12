//! Where a cited specification is found (PROP-057 `##LOCAL-WARMUP`).
//!
//! Documentation is read in four situations and the address grammar is
//! the same in all of them, so the resolver is one and the SOURCES are
//! four, tried in a fixed order:
//!
//! 1. **the checkout** — the tree the reader is standing in. Its own
//!    coordinate resolves here, which is what makes `vibe doc` work on a
//!    developer's machine before anything is published.
//! 2. **in-tree packages** — the project-local registry this repository
//!    authors (`LocalRegistry`). A manual documenting a package that
//!    lives beside it must reach the version in the tree, not a stale
//!    published one.
//! 3. **the lock file** — the exact instances this project selected,
//!    read from `vibe.lock` and taken from their materialisation slots.
//! 4. **the machine store** — warmed by `vibe cache add`. This is how the
//!    local reader answers an address that belongs to no project at all.
//!
//! The order is the answer to «which instance did I get»: nearest first.
//! A source the caller did not configure is simply absent from the chain,
//! so a library call names its world instead of discovering one — this
//! crate reads no environment (the ambient-environment law of the
//! new-crate checklist).
//!
//! Not one of the four layouts is spelled here. The in-tree registry and
//! the store share one shape and [`vibe_registry`] owns it; the slot
//! shape is [`vibe_workspace::vibedeps::slot_abs_path`]; the `spec/` root
//! inside a package and the file a doc-path names are
//! [`vibe_spec::FileResolver`]. A second spelling of any of them would be
//! a second law, and the day one moved only one of the two would follow.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-WARMUP");

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use vibe_core::Group;
use vibe_core::manifest::Lockfile;
use vibe_registry::{LocalRegistry, store};
use vibe_spec::{Authority, FileResolver, SelectedPackage, SelfCoordinate, SpecAddress};

/// Which of the four sources answered an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// The checkout the reader is standing in — the documenting project's
    /// own coordinate.
    Checkout,
    /// The project-local registry of packages this repository authors.
    InTree,
    /// A materialisation slot the lock file selected.
    Lock,
    /// The machine store.
    Store,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Checkout => "checkout",
            Source::InTree => "in-tree",
            Source::Lock => "lock",
            Source::Store => "store",
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One package instance a source offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub source: Source,
    /// The version the source holds, or empty for the checkout: a
    /// project's own coordinate is unversioned by law (B-031), because a
    /// checkout is whatever is on disk right now.
    pub version: String,
    pub root: PathBuf,
}

/// A located document: the file, which source held it, and the language
/// the package it belongs to is written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub path: PathBuf,
    pub source: Source,
    pub lang: String,
}

impl Located {
    /// The path as a report prints it — forward-slashed, so a refusal
    /// reads the same on every platform.
    pub fn display_path(&self) -> String {
        fwd(&self.path)
    }
}

/// The world a citation resolves against.
///
/// Every field is supplied by the caller: the composition root knows
/// where the checkout, the workspace and the store are, and handing them
/// down is what keeps a documentation build reproducible by inspection.
#[derive(Debug, Clone, Default)]
pub struct SpecSources {
    /// The checkout root, with the coordinate it answers to.
    checkout: Option<(SelfCoordinate, PathBuf)>,
    /// The in-tree package registry root.
    in_tree: Option<PathBuf>,
    /// The workspace root whose `vibe.lock` and slots are read.
    lock: Option<PathBuf>,
    /// The machine store root.
    store: Option<PathBuf>,
}

impl SpecSources {
    /// An empty world. Every address refuses until a source is added —
    /// the safe default for a caller that has not said where it stands.
    pub fn new() -> SpecSources {
        SpecSources::default()
    }

    /// The ordinary developer's world: one repository checkout, whose own
    /// coordinate is `<group>/<name>`. Adds the checkout, its in-tree
    /// package registry and its lock slots in one call.
    ///
    /// ```
    /// use vibe_doc::citations::SpecSources;
    /// let world = SpecSources::for_checkout("/repo", Some("org.vibevm.core"), "vibevm");
    /// assert!(world.has_checkout());
    /// ```
    pub fn for_checkout(
        repo_root: impl Into<PathBuf>,
        group: Option<&str>,
        name: &str,
    ) -> SpecSources {
        let root: PathBuf = repo_root.into();
        SpecSources {
            checkout: Some((
                SelfCoordinate::new(group.map(str::to_owned), name.to_owned()),
                root.clone(),
            )),
            in_tree: Some(root.join(vibe_core::layout::current_packages_root())),
            lock: Some(root),
            store: None,
        }
    }

    /// Add the machine store.
    #[must_use]
    pub fn with_store(mut self, store_root: impl Into<PathBuf>) -> SpecSources {
        self.store = Some(store_root.into());
        self
    }

    /// Add (or replace) the in-tree package registry root.
    #[must_use]
    pub fn with_in_tree(mut self, packages_root: impl Into<PathBuf>) -> SpecSources {
        self.in_tree = Some(packages_root.into());
        self
    }

    /// Add (or replace) the workspace whose lock file is read.
    #[must_use]
    pub fn with_lock(mut self, workspace_root: impl Into<PathBuf>) -> SpecSources {
        self.lock = Some(workspace_root.into());
        self
    }

    /// Whether a checkout is configured — the only source that answers an
    /// address to the documenting project itself.
    pub fn has_checkout(&self) -> bool {
        self.checkout.is_some()
    }

    /// Find the instance of `<group>/<name>` the chain offers, nearest
    /// source first.
    pub fn instance_of(&self, group: &str, name: &str) -> Option<Instance> {
        if let Some((coord, root)) = &self.checkout
            && coord.group.as_deref() == Some(group)
            && coord.name == name
        {
            return Some(Instance {
                source: Source::Checkout,
                version: String::new(),
                root: root.clone(),
            });
        }
        let parsed = Group::parse(group).ok()?;
        if let Some(packages) = &self.in_tree
            && let Some(instance) = registry_instance(packages, Source::InTree, &parsed, name)
        {
            return Some(instance);
        }
        if let Some(workspace) = &self.lock
            && let Some(instance) = locked_instance(workspace, &parsed, name)
        {
            return Some(instance);
        }
        if let Some(root) = &self.store
            && let Some(instance) = registry_instance(root, Source::Store, &parsed, name)
        {
            return Some(instance);
        }
        None
    }

    /// Resolve one parsed address to the file that holds its document.
    ///
    /// The «which root» half is this chain; the «which file inside a
    /// root» half — including the inversion of the lossy `PROP-042`
    /// truncation back to `PROP-042-example-thing.xml` — is
    /// [`vibe_spec::FileResolver`], which owns that law.
    pub fn locate(&self, address: &SpecAddress) -> Result<Located, String> {
        let (group, name) = match &address.authority {
            Authority::Package { group, name, .. } => (group.clone(), name.clone()),
            Authority::Host(h) => {
                return Err(format!(
                    "`spec://{h}/…` names no package — an address is \
                     `spec://<group>/<name>/<doc-path>#<anchor>`"
                ));
            }
        };
        let instance = self.instance_of(&group, &name).ok_or_else(|| {
            format!(
                "no source holds `{group}/{name}` — looked in {}",
                self.describe()
            )
        })?;
        let resolver = self.resolver_for(&group, &name, &instance);
        let path = resolver
            .resolve_file(address)
            .map_err(|e| format!("{e} (source: {}, {})", instance.source, fwd(&instance.root)))?;
        Ok(Located {
            path,
            source: instance.source,
            lang: canonical_language(&instance.root),
        })
    }

    /// A resolver pinned to exactly the instance the chain chose, so no
    /// «freshest installed» search inside `vibe-spec` can substitute an
    /// instance this chain did not pick.
    fn resolver_for(&self, group: &str, name: &str, instance: &Instance) -> FileResolver {
        let coord = SelfCoordinate::new(Some(group.to_owned()), name.to_owned());
        if instance.source == Source::Checkout {
            // The self coordinate is unversioned by law, so it resolves
            // through the self arm rather than the selected world.
            return FileResolver::new(instance.root.clone(), coord);
        }
        let mut selected: BTreeMap<(String, String), SelectedPackage> = BTreeMap::new();
        selected.insert(
            (group.to_owned(), name.to_owned()),
            SelectedPackage::new(instance.version.clone(), instance.root.clone()),
        );
        FileResolver::with_selected_world(
            instance.root.clone(),
            instance.root.clone(),
            selected,
            // A coordinate that is deliberately NOT the address's own, so
            // the address takes the selected arm and its `@version`, when
            // it carries one, is CHECKED against what the chain found
            // rather than silently dropped.
            SelfCoordinate::new(None, String::new()),
        )
    }

    /// The chain, named — so a refusal says where the resolver looked
    /// instead of leaving the author to guess.
    fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some((coord, root)) = &self.checkout {
            let group = coord.group.as_deref().unwrap_or("<none>");
            parts.push(format!(
                "checkout `{group}/{}` at `{}`",
                coord.name,
                fwd(root)
            ));
        }
        if let Some(p) = &self.in_tree {
            parts.push(format!("in-tree `{}`", fwd(p)));
        }
        if let Some(p) = &self.lock {
            parts.push(format!("lock `{}`", fwd(&p.join(Lockfile::FILENAME))));
        }
        if let Some(p) = &self.store {
            parts.push(format!("store `{}`", fwd(p)));
        }
        if parts.is_empty() {
            return "no source at all — the caller configured none".to_owned();
        }
        parts.join(", ")
    }
}

/// The freshest instance a directory registry holds. The project-local
/// registry and the machine store share one layout — `LocalRegistry` is
/// its reader and `store::entry_dir` its one spelling — so one function
/// serves both sources.
fn registry_instance(root: &Path, source: Source, group: &Group, name: &str) -> Option<Instance> {
    let registry = LocalRegistry::new(root).ok()?;
    let newest = registry.list_versions(group, name).ok()?.pop()?;
    let dir = store::entry_dir(root, group, name, &newest);
    dir.is_dir().then(|| Instance {
        source,
        version: newest.to_string(),
        root: dir,
    })
}

/// The instance the lock file selected, from its materialisation slot.
/// The lock is read rather than the slot directory walked: the question
/// this source answers is «which one did this project choose», and only
/// the lock knows.
fn locked_instance(workspace_root: &Path, group: &Group, name: &str) -> Option<Instance> {
    let lock = Lockfile::read(workspace_root.join(Lockfile::FILENAME)).ok()?;
    let locked = lock
        .packages
        .iter()
        .find(|p| p.group == *group && p.name.as_str() == name)?;
    let slot =
        vibe_workspace::vibedeps::slot_abs_path(workspace_root, group, name, &locked.version);
    slot.is_dir().then(|| Instance {
        source: Source::Lock,
        version: locked.version.to_string(),
        root: slot,
    })
}

/// The language a package's own text is written in — `[i18n].canonical`
/// of its manifest, or the registry default when it declares none.
///
/// The manifest is read as TOML DATA rather than through the typed model:
/// this crate wants one optional string out of a document whose strict
/// shape belongs to whoever owns the manifest, and a strict parse would
/// make a language lookup fail over a field it never reads.
fn canonical_language(root: &Path) -> String {
    let default = vibe_core::manifest::i18n::DEFAULT_CANONICAL_LANGUAGE.to_owned();
    let Ok(text) = std::fs::read_to_string(root.join("vibe.toml")) else {
        return default;
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return default;
    };
    value
        .get("i18n")
        .and_then(|i| i.get("canonical"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .unwrap_or(default)
}

/// A path as a message prints it.
fn fwd(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests;
