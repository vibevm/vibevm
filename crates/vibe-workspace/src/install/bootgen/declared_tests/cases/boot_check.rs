//! Bound verification uses selected records while retaining legacy refusal.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-BOOT-CHECK",
    r = 1
);
use super::*;

#[test]
fn bound_check_uses_selected_format_and_retains_unbound_refusal() {
    for format in [SpecFormat::Mixed, SpecFormat::Xml] {
        let f = fixture(true);
        apply(&f, format);
        crate::install::tests_analyze_parity::publish_resolution_lock(
            &f.workspace.root,
            &f.resolution,
        );
        assert!(install::verify_boot_graph(&f.workspace).is_err());
        assert!(
            install::verify_boot_graph_with_spec_format(&f.workspace, format)
                .unwrap()
                .is_empty()
        );
        let wrong = if format == SpecFormat::Mixed {
            SpecFormat::Xml
        } else {
            SpecFormat::Mixed
        };
        assert!(install::verify_boot_graph_with_spec_format(&f.workspace, wrong).is_err());
        let parent = &f.resolution[0];
        let index = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        )
        .join(vibe_core::layout::current_boot_index());
        fs::remove_file(index).unwrap();
        assert_eq!(
            install::verify_boot_graph_with_spec_format(&f.workspace, format).unwrap(),
            vec![(parent.group.clone(), parent.name.clone())]
        );
    }
}

#[test]
fn bound_check_preserves_selected_source_hash_consistency() {
    let mut f = fixture(true);
    apply(&f, SpecFormat::Mixed);
    f.resolution[0].source_hash = Some(
        vibe_core::ContentHash::parse(
            "sha256:2222222222222222222222222222222222222222222222222222222222222222",
        )
        .unwrap(),
    );
    crate::install::tests_analyze_parity::publish_resolution_lock(&f.workspace.root, &f.resolution);
    let error =
        install::verify_boot_graph_with_spec_format(&f.workspace, SpecFormat::Mixed).unwrap_err();
    assert!(error.to_string().contains("hash"), "{error}");
}
