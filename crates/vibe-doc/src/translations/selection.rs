//! Resolve a translation's declared source, preserving absence versus refusal.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOC-MIRROR");
use super::*;
use crate::citations::sources::Instance;

/// What a package adapts, and which instance of it answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapted {
    /// The `<group>/<name>` coordinate of the documentation being
    /// mirrored, exactly as `[translates].package` spells it.
    pub coordinate: String,
    /// The instance the four sources offered for it, nearest first.
    pub instance: Instance,
}

/// The source documentation `package_dir` adapts, located.
///
/// `Ok(None)` means the package declares no `[translates]` — it is a
/// source documentation, and there is nothing to look for.
///
/// One lookup, two callers: the mirror check ([`check`]) reads the source
/// to compare structure, and [`borrowed`] reads it for the example bodies
/// a translation's pages borrow. A second spelling of «which source does
/// this package adapt» would be the day one of them looked in a different
/// tree than the other reported on.
///
/// Both refusals are errors rather than a silent `None`: a coordinate that
/// is not `<group>/<name>` is a manifest defect, and a source no source
/// holds leaves a mirror check with no verdict to give — green would be
/// the worst of the three possible answers there.
///
/// ```
/// use vibe_doc::citations::SpecSources;
/// use vibe_doc::translations::adapted;
///
/// let dir = tempfile::tempdir().unwrap();
/// std::fs::write(
///     dir.path().join("vibe.toml"),
///     "[package]\nname = \"pair\"\ngroup = \"com.example.docs\"\n",
/// )
/// .unwrap();
///
/// // A source documentation adapts nothing, so there is nothing to find
/// // — and that is not a failure.
/// assert_eq!(adapted(dir.path(), &SpecSources::new())?, None);
/// # Ok::<(), vibe_doc::DocError>(())
/// ```
pub fn adapted(package_dir: &Path, sources: &SpecSources) -> Result<Option<Adapted>> {
    adapted_selection(package_dir, sources, false)
}

pub(super) fn adapted_selection(
    package_dir: &Path,
    sources: &SpecSources,
    missing_is_empty: bool,
) -> Result<Option<Adapted>> {
    let relations = match manifest::relations(package_dir, TRANSLATES) {
        Ok(relations) => relations,
        Err(DocError::Io { source, .. })
            if missing_is_empty && source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let Some((coordinate, _constraint)) = relations.into_iter().next() else {
        return Ok(None);
    };
    let (group, name) = coordinate
        .split_once('/')
        .ok_or_else(|| DocError::Translation {
            adapts: coordinate.clone(),
            message: "`[translates].package` is not a `<group>/<name>` coordinate".to_owned(),
        })?;
    let selected =
        sources
            .selected_instance(group, name)
            .map_err(|message| DocError::Translation {
                adapts: coordinate.clone(),
                message,
            })?;
    if selected.is_none() && missing_is_empty {
        return Ok(None);
    }
    let instance = selected.ok_or_else(|| DocError::Translation {
        adapts: coordinate.clone(),
        message: format!(
            "no source holds it, so there is nothing to mirror against; warm it with \
                 `vibe cache add {coordinate}`, or point the check at a tree that carries it"
        ),
    })?;
    Ok(Some(Adapted {
        coordinate,
        instance,
    }))
}
