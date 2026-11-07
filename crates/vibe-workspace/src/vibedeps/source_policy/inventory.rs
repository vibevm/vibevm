//! Finite published-payload inventory, not an original-tree/world census.
use super::*;
use std::fs;
use vibe_core::manifest::PackageMeta;

pub(super) fn capture(policy: &mut SourcePolicy) -> Result<(), WorkspaceError> {
    let package = policy
        .observed
        .package()
        .ok_or_else(|| refused(&policy.root, "declared package context missing"))?
        .clone();
    let mut paths = Vec::new();
    let physical = policy
        .root
        .canonicalize()
        .map_err(|error| refused(&policy.root, error))?;
    let project =
        vibe_safefs::Project::open(&physical).map_err(|error| refused(&policy.root, error))?;
    let root = project
        .root_dir()
        .map_err(|error| refused(&policy.root, error))?;
    policy
        .observed
        .recheck_in(&project, &root)
        .map_err(|error| refused(&policy.root, error))?;
    walk(&project, &root, "", &package, &mut paths)?;
    policy
        .observed
        .recheck_in(&project, &root)
        .map_err(|error| refused(&policy.root, error))?;
    paths.sort();
    for descriptor in package.sources.as_deref().unwrap_or(&[]) {
        if let SourceSelector::File(path) = descriptor.selector()
            && !paths.iter().any(|found| found == path)
        {
            return Err(refused(
                &policy.root.join(path),
                "declared exact input is absent from the published regular-file payload (missing, excluded or not regular)",
            ));
        }
    }
    for path in paths {
        match package.select_source(&path).map_err(|error| {
            refused(
                &policy.root.join(&path),
                format!("source selection failed: {error:?}"),
            )
        })? {
            SelectedSource::NeedsInput {
                declaration_indices,
                ..
            } => {
                return Err(refused(
                    &policy.root.join(&path),
                    format!("source declaration conflict at rows {declaration_indices:?}"),
                ));
            }
            SelectedSource::Descriptor { .. } | SelectedSource::Unclassified { .. } => {}
        }
        if !policy.captured.contains_key(&path) {
            policy.capture(&path)?;
        }
    }
    Ok(())
}

fn walk(
    project: &vibe_safefs::Project,
    directory: &vibe_safefs::Pinned,
    prefix: &str,
    package: &PackageMeta,
    paths: &mut Vec<String>,
) -> Result<(), WorkspaceError> {
    recheck_directory(project, directory, prefix)?;
    for name in project
        .child_names(directory)
        .map_err(|error| refused(directory.path(), error))?
    {
        // Child names are not paths: never reinterpret a Unix literal backslash.
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let path = directory.join(&name);
        if name == ".git" || vibe_core::layout::is_generated_boot_output(Path::new(&relative)) {
            continue;
        }
        package
            .select_source(&relative)
            .map_err(|error| refused(&path, format!("source path selection refused: {error:?}")))?;
        recheck_directory(project, directory, prefix)?;
        let kind = fs::symlink_metadata(&path)
            .map_err(|error| refused(&path, error))?
            .file_type();
        recheck_directory(project, directory, prefix)?;
        if kind.is_dir() {
            #[cfg(test)]
            inject::run(&path);
            let child = directory
                .open_child(&name)
                .map_err(|error| refused(&path, error))?;
            walk(project, &child, &relative, package, paths)?;
        } else if kind.is_file() {
            paths.push(relative);
        } else if selected_nonregular(package, &relative)? {
            return Err(refused(
                &path,
                "declared payload input is a symlink or special entry; no-follow publication refuses it",
            ));
        }
    }
    recheck_directory(project, directory, prefix)
}

fn recheck_directory(
    project: &vibe_safefs::Project,
    directory: &vibe_safefs::Pinned,
    relative: &str,
) -> Result<(), WorkspaceError> {
    let components = if relative.is_empty() {
        Vec::new()
    } else {
        relative.split('/').collect()
    };
    let named = project
        .dir(&components, false)
        .map_err(|error| refused(directory.path(), error))?;
    if named
        .identity()
        .map_err(|error| refused(directory.path(), error))?
        != directory
            .identity()
            .map_err(|error| refused(directory.path(), error))?
    {
        return Err(refused(
            directory.path(),
            "enumerated directory changed identity",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod inject {
    use std::{cell::RefCell, path::Path};
    type Hook = Box<dyn FnOnce(&Path)>;
    thread_local! { static BEFORE: RefCell<Option<Hook>> = const { RefCell::new(None) }; }
    pub(in crate::vibedeps) fn arm(hook: Hook) {
        BEFORE.with(|slot| *slot.borrow_mut() = Some(hook));
    }
    pub(super) fn run(path: &Path) {
        let hook = BEFORE.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(path);
        }
    }
}

fn selected_nonregular(package: &PackageMeta, path: &str) -> Result<bool, WorkspaceError> {
    if !matches!(
        package.select_source(path).map_err(|error| refused(
            Path::new(path),
            format!("source selection failed: {error:?}")
        ))?,
        SelectedSource::Unclassified { .. }
    ) {
        return Ok(true);
    }
    Ok(package
        .sources
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .any(|descriptor| {
            let target = descriptor.selector().path();
            target == path
                || target
                    .strip_prefix(path)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        }))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("vibe.toml"), "[package]\ngroup='org.inventory'\nname='sample'\nversion='1.0.0'\nkind='flow'\n[[package.sources]]\ndirectory='docs'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='scan'\n").unwrap();
        root
    }
    fn materialise(source: &Path, root: &Path) -> Result<Vec<PathBuf>, WorkspaceError> {
        crate::vibedeps::materialise(
            root,
            &vibe_core::Group::parse("org.inventory").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
            source,
            &vibe_core::ContentHash::parse(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
        )
    }
    #[test]
    fn m_06_a_3_unix_literal_backslash_never_aliases_nested_payload() {
        let source = fixture();
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(source.path().join("a")).unwrap();
        fs::write(source.path().join("a/b.md"), b"nested").unwrap();
        fs::write(source.path().join("a\\b.md"), b"literal backslash").unwrap();
        fs::write(root.path().join("sentinel"), b"unchanged").unwrap();
        let error = materialise(source.path(), root.path()).unwrap_err();
        assert!(error.to_string().contains("backslash"), "{error}");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        assert_eq!(fs::read(source.path().join("a/b.md")).unwrap(), b"nested");
        assert_eq!(
            fs::read(source.path().join("a\\b.md")).unwrap(),
            b"literal backslash"
        );
    }
    #[test]
    fn m_06_a_3_enumerated_directory_swap_cannot_supply_outside_inventory() {
        let source = fixture();
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(source.path().join("docs")).unwrap();
        fs::write(source.path().join("docs/inside"), b"inside").unwrap();
        fs::write(outside.path().join("outside"), b"outside").unwrap();
        let outside_path = outside.path().to_owned();
        inject::arm(Box::new(move |path| {
            fs::rename(path, path.with_file_name("saved-docs")).unwrap();
            std::os::unix::fs::symlink(&outside_path, path).unwrap();
        }));
        assert!(materialise(source.path(), root.path()).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        assert_eq!(
            fs::read(outside.path().join("outside")).unwrap(),
            b"outside"
        );
        assert_eq!(
            fs::read(source.path().join("saved-docs/inside")).unwrap(),
            b"inside"
        );
        assert!(
            fs::symlink_metadata(source.path().join("docs"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}
