//! Selected package members are source inputs, not extra in-place node outputs.

use vibe_core::manifest::Manifest;

use super::super::ResolvedDep;

/// Root and ordinary member entrypoints remain published. A non-root authored
/// package member with the selected typed package identity is protected even
/// when its authored version has drifted from the durable selected slot.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#ARTIFACTS-PAIR"
)]
pub(super) fn publish_node_boot(
    rel: &str,
    manifest: &Manifest,
    resolution: &[ResolvedDep],
) -> bool {
    if rel == "." {
        return true;
    }
    let Some(package) = manifest.package.as_ref() else {
        return true;
    };
    !resolution.iter().any(|selected| {
        selected.kind == package.kind
            && selected.group == package.group
            && selected.name == package.name.as_str()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::test_helpers::dep_with_boot;

    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#ARTIFACTS-PAIR"
    )]
    fn protection_matches_kind_group_name_but_not_version_or_source_path() {
        let (selected, _source) = dep_with_boot("member", "1.0.0", "", "boot/main.md", "# Main\n");
        let mut member = selected.manifest.clone();
        member.package.as_mut().unwrap().version = "1.0.1".parse().unwrap();
        let resolution = [selected];
        assert!(!publish_node_boot("members/source", &member, &resolution));
        assert!(publish_node_boot(".", &member, &resolution));
        for mismatch in ["kind", "group", "name"] {
            let mut other = member.clone();
            let package = other.package.as_mut().unwrap();
            match mismatch {
                "kind" => package.kind = vibe_core::PackageKind::Tool,
                "group" => package.group = vibe_core::Group::parse("org.other").unwrap(),
                "name" => package.name = "other".to_owned(),
                _ => unreachable!(),
            }
            assert!(
                publish_node_boot("members/source", &other, &resolution),
                "{mismatch}"
            );
        }
    }
}
