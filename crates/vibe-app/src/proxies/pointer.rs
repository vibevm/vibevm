//! `<store>/vibe-app-home.toml`: the home a proxy in `<store>/bin` belongs
//! to, when the applications follow an install root other than the
//! settings root (`PROP-019#install-root`, `PROP-059#proxy`). A proxy finds
//! its home by walking up to `applications/index.json`; from the install
//! root's `bin` that walk reaches no index, so the store names the home.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#proxy");

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, StateError};
use crate::settings::{SettingsRoot, io, same_place, write_atomic};

const POINTER_SCHEMA: u32 = 1;
/// A schema line and one path: far below this.
const POINTER_MAX_BYTES: u64 = 64 * 1024;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Pointer {
    schema: u32,
    home: PathBuf,
}

/// Name this home in the store before a proxy is written to `<store>/bin`.
/// A pointer to another home that still has an index is refused: one
/// install root store serves one home. A pointer to a home that is gone is
/// replaced; anything that is not a pointer is refused and left alone.
pub(crate) fn ensure(settings: &SettingsRoot) -> Result<(), AppError> {
    let Some(path) = settings.home_pointer_path() else {
        return Ok(());
    };
    let home = fs::canonicalize(settings.path())
        .map(crate::resolve::strip_verbatim)
        .map_err(|source| io("resolving", settings.path(), source))?;
    match read(&path)? {
        Some(Ok(pointer)) if same_place(&pointer.home, &home) => return Ok(()),
        Some(Ok(pointer)) if has_index(&pointer.home) => {
            return Err(shared(&path, pointer.home.display().to_string()));
        }
        Some(Ok(_)) | None => {}
        Some(Err(())) => {
            return Err(shared(
                &path,
                "unknown: the file is not a home pointer vibe wrote".into(),
            ));
        }
    }
    let text = toml::to_string(&Pointer {
        schema: POINTER_SCHEMA,
        home,
    })
    .map_err(|error| {
        io(
            "serialising the home pointer",
            &path,
            std::io::Error::other(error),
        )
    })?;
    write_atomic(&path, text.as_bytes())?;
    Ok(())
}

fn has_index(home: &Path) -> bool {
    home.join("applications").join("index.json").is_file()
}

fn shared(path: &Path, other: String) -> AppError {
    StateError::InstallRootShared {
        store: path
            .parent()
            .map(|store| store.display().to_string())
            .unwrap_or_default(),
        other,
        pointer: path.display().to_string(),
    }
    .into()
}

/// The pointer at `path`: absent, a pointer, or a file that is not one.
fn read(path: &Path) -> Result<Option<Result<Pointer, ()>>, AppError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(io("reading the home pointer", path, source).into()),
    };
    if !metadata.is_file() || metadata.len() > POINTER_MAX_BYTES {
        return Ok(Some(Err(())));
    }
    let text =
        fs::read_to_string(path).map_err(|source| io("reading the home pointer", path, source))?;
    Ok(Some(
        toml::from_str::<Pointer>(&text)
            .ok()
            .filter(|pointer| pointer.schema == POINTER_SCHEMA && pointer.home.is_absolute())
            .ok_or(()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_store_names_one_home_and_refuses_a_second_live_one() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("Work/opt");
        let first = SettingsRoot::new(&dir.path().join("first")).with_install_root(&store);
        fs::create_dir_all(first.applications_dir()).unwrap();
        ensure(&first).unwrap();
        ensure(&first).unwrap();
        let text = fs::read_to_string(store.join(crate::settings::HOME_POINTER)).unwrap();
        assert!(text.contains("first"), "{text}");

        let second = SettingsRoot::new(&dir.path().join("second")).with_install_root(&store);
        fs::create_dir_all(second.applications_dir()).unwrap();
        // The first home has no index yet: its pointer is stale and replaced.
        ensure(&second).unwrap();
        fs::write(second.index_path(), b"{}").unwrap();
        let error = ensure(&first).unwrap_err();
        assert!(
            matches!(error, AppError::State(StateError::InstallRootShared { .. })),
            "{error}"
        );
        assert!(error.to_string().contains("PROP-059#proxy"), "{error}");

        // A file that is not a pointer is never overwritten.
        fs::write(store.join(crate::settings::HOME_POINTER), b"operator notes").unwrap();
        assert!(ensure(&second).is_err());
        assert_eq!(
            fs::read(store.join(crate::settings::HOME_POINTER)).unwrap(),
            b"operator notes"
        );
    }
}
