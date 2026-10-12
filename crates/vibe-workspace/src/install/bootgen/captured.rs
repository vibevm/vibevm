//! Incumbent boot rules over a supplied inventory; no discovery or publication.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#effective-boot");
use crate::{WorkspaceError, boot::AuthoredBoot, boot_artifacts, path_to_slash};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use vibe_core::{
    Group, layout,
    manifest::{BootCategory, SpecFormat, WhenCondition},
};
pub fn authored_boot_from_inventory(
    node_dir: &Path,
    node_rel: &str,
    files: &[PathBuf],
) -> Result<Vec<AuthoredBoot>, WorkspaceError> {
    let mut result = vec![];
    let mut spec_paths = vec![];
    for file in files {
        if file.parent() != Some(layout::current_boot_dir().as_path())
            || !vibe_specdoc::is_spec_source(file)
        {
            continue;
        }
        let Some(name) = file.file_name().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        if name == boot_artifacts::static_file(SpecFormat::Mixed)
            || name == boot_artifacts::static_file(SpecFormat::Xml)
            || name == boot_artifacts::INDEX_FILE
        {
            continue;
        }
        let category = match name.as_str() {
            "00-core.md" => Some(BootCategory::Foundation),
            "90-user.md" => Some(BootCategory::UserOverride),
            _ => None,
        };
        let path = if node_rel == "." {
            path_to_slash(file)
        } else {
            path_to_slash(&Path::new(node_rel).join(file))
        };
        spec_paths.push(node_dir.join(file));
        result.push(AuthoredBoot {
            path,
            category,
            origin: node_rel.into(),
        });
    }
    if let Some(collision) = vibe_specdoc::pair_collisions_in(&spec_paths).first() {
        return Err(WorkspaceError::Io {
            path: collision
                .markdown
                .strip_prefix(node_dir)
                .unwrap_or(&collision.markdown)
                .to_path_buf(),
            reason: collision.message(),
        });
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}
/// Literal first, then paired representation, then unchanged missing literal.
pub fn resolve_snippet_source_with(
    slot: &str,
    source: &Path,
    exists: impl Fn(&str) -> bool,
) -> String {
    let source_rel = source.to_string_lossy().replace('\\', "/");
    let literal = format!("{slot}/{source_rel}");
    if exists(&literal) {
        return literal;
    }
    if let Some(swapped) = super::snippet_source::swap_spec_extension(Path::new(&source_rel)) {
        let alt = format!("{slot}/{}", swapped.to_string_lossy().replace('\\', "/"));
        if exists(&alt) {
            return alt;
        }
    }
    literal
}
pub fn resolve_snippet_condition(
    when: Option<&WhenCondition>,
    installed: &HashSet<(Group, String)>,
) -> Option<Option<WhenCondition>> {
    match when {
        None => Some(None),
        Some(WhenCondition::Os(os)) => Some(Some(WhenCondition::Os(*os))),
        Some(WhenCondition::Installed { group, name }) => installed
            .contains(&(group.clone(), name.to_string()))
            .then_some(None),
    }
}
#[cfg(test)]
mod m_16_d_1_cases {
    use super::*;
    #[test]
    fn captured_inventory_preserves_categories_pairs_and_snippet_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let boot = layout::current_boot_dir();
        let files: Vec<_> = [
            "00-core.md",
            "10-rule.xml",
            "90-user.md",
            "STATIC.md",
            "STATIC.xml",
        ]
        .iter()
        .map(|name| boot.join(name))
        .collect();
        let own = authored_boot_from_inventory(dir.path(), "member", &files).unwrap();
        assert_eq!(own.len(), 3);
        assert_eq!(own[0].category, Some(BootCategory::Foundation));
        assert_eq!(own[2].category, Some(BootCategory::UserOverride));
        let mut paired = files.clone();
        paired.push(boot.join("10-rule.md"));
        assert!(matches!(
            authored_boot_from_inventory(dir.path(), ".", &paired),
            Err(WorkspaceError::Io { .. })
        ));
        let source = Path::new("spec/X.md");
        assert_eq!(
            resolve_snippet_source_with("slot", source, |_| false),
            "slot/spec/X.md"
        );
        assert_eq!(
            resolve_snippet_source_with("slot", source, |p| p.ends_with(".xml")),
            "slot/spec/X.xml"
        );
        assert_eq!(
            resolve_snippet_source_with("slot", source, |_| true),
            "slot/spec/X.md"
        );
    }
    #[test]
    fn captured_condition_keeps_os_and_filters_installed() {
        let when = WhenCondition::Installed {
            group: Group::parse("org.demo").unwrap(),
            name: vibe_core::PackageName::parse("dep").unwrap(),
        };
        assert!(resolve_snippet_condition(Some(&when), &HashSet::new()).is_none());
        assert_eq!(
            resolve_snippet_condition(
                Some(&when),
                &HashSet::from([(Group::parse("org.demo").unwrap(), "dep".into())])
            ),
            Some(None)
        );
        let os = WhenCondition::Os(vibe_core::manifest::TargetOs::Linux);
        assert_eq!(
            resolve_snippet_condition(Some(&os), &HashSet::new()),
            Some(Some(os.clone()))
        );
    }
}
