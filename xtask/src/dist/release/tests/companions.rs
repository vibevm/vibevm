//! `dist finalize` with companion px assets (PROP-019 `#companion-tools`).

use super::*;

/// px as a companion asset (PROP-019 `#companion-tools`): each uploaded
/// pair is verified and listed in `TOOLS.json`; a listing whose archive
/// differs refuses the whole finalize; `DISTRIBUTIONS.json` is unchanged.
#[test]
fn finalize_lists_verified_companion_px_and_refuses_a_mismatched_one() {
    use vibe_publish::release_manifest::{CompanionAsset, CompanionTool};
    let seeded = |px_target: &str, archive: &[u8], listed: &[u8]| {
        let host = MockHost::draft();
        for target in SUPPORTED_DISTRIBUTION_TARGETS {
            let (fragment, bundle, bootstrap, fragment_bytes) = platform(target);
            host.seed(fragment.asset.name.clone(), bundle);
            host.seed(fragment.bootstrap.name.clone(), bootstrap);
            host.seed(
                fragment_asset_name(&Version::parse("1.0.0").unwrap(), target),
                fragment_bytes,
            );
        }
        let name = format!("px-0.11.0-{px_target}.tar.gz");
        let tool = CompanionTool {
            name: "px".into(),
            version: "0.11.0".into(),
            target: px_target.into(),
            executable: "px".into(),
            asset: CompanionAsset {
                name: name.clone(),
                size: listed.len() as u64,
                digest: sha256_digest(listed),
            },
        };
        host.seed(name, archive.to_vec());
        host.seed(
            crate::dist::px_tool::fragment_name("0.11.0", px_target),
            serde_json::to_vec(&tool).unwrap(),
        );
        host
    };
    let identity = GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    };
    let version = Version::parse("1.0.0").unwrap();
    let host = seeded("x86_64-unknown-linux-gnu", b"px archive", b"px archive");
    let verified = finalize_with(&host, &version, &identity).unwrap();
    let tools = verified.tools.expect("a companion listing");
    assert_eq!(tools.tag, verified.aggregate.tag);
    assert_eq!(tools.source_commit, verified.aggregate.source_commit);
    assert_eq!(tools.tools.len(), 1);
    assert_eq!(tools.tools[0].target, "x86_64-unknown-linux-gnu");
    // The aggregate is the closed shape older vibe releases read.
    let aggregate = String::from_utf8(verified.aggregate.to_json_bytes().unwrap()).unwrap();
    assert!(!aggregate.contains("px-0.11.0"));

    let host = seeded("x86_64-unknown-linux-gnu", b"px archive", b"another px");
    let error = finalize_with(&host, &version, &identity).err().unwrap();
    assert!(format!("{error:#}").contains("px-0.11.0"), "{error:#}");
}
