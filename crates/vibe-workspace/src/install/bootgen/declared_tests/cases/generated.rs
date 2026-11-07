//! Actual substituted generated XML and freshness refusal cases.
#[cfg(test)]
mod tests {
    use super::super::*;
    #[test]
    fn generated_normal_xml_preserves_prior_embed_data_without_reminting_attribution() {
        #[derive(Default)]
        struct Observer(std::sync::Mutex<Vec<vibe_spec::EmissionEvent>>);
        impl vibe_spec::CompileObserver for Observer {
            fn stage_delta(&self, _: &vibe_spec::StageDeltaEvent) {}
            fn emission(&self, event: &vibe_spec::EmissionEvent) {
                self.0.lock().unwrap().push(event.clone());
            }
        }
        let f = generated_fixture();
        let parent = &f.resolution[0];
        let slot = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        );
        let unit =
            fs::read_to_string(slot.join(vibe_core::layout::current_boot_static_xml())).unwrap();
        let unit_projection = vibe_spec::project_emitted_xml_to_markdown(&unit).unwrap();
        assert!(
            unit_projection.contains("<!-- vibe:c1 embed:"),
            "actual unit did not carry embed framing: {unit_projection}"
        );
        assert_eq!(unit_projection.matches("embedded_once_marker").count(), 1);
        crate::install::tests_analyze_parity::publish_resolution_lock(
            &f.workspace.root,
            &f.resolution,
        );
        let observer = std::sync::Arc::new(Observer::default());
        let analyzed = install::analyze_node_lane_with_source_format(
            &f.workspace,
            ".",
            Some(observer.clone()),
            SpecFormat::Xml,
        )
        .unwrap()
        .unwrap();
        assert!(
            analyzed
                .identities
                .iter()
                .any(|(_, path)| path.ends_with("boot/STATIC.xml")),
            "fixture did not traverse substituted generated Normal input: {:?}",
            analyzed.identities
        );
        let text = std::str::from_utf8(analyzed.artifact.bytes()).unwrap();
        assert_eq!(text.matches("embedded_once_marker").count(), 1, "{text}");
        assert_eq!(text.matches("fragment_once_marker").count(), 1, "{text}");
        assert_eq!(text.matches("current_parent_marker").count(), 1, "{text}");
        assert_eq!(
            text.matches("forged").count(),
            1,
            "literal prose was lost or promoted: {text}"
        );
        let events = observer.0.lock().unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.total_bytes(), analyzed.artifact.bytes().len());
        assert_eq!(
            event.frame_bytes()
                + event
                    .contributions()
                    .iter()
                    .map(|row| row.bytes())
                    .sum::<usize>(),
            event.total_bytes()
        );
        let current = event
            .contributions()
            .iter()
            .filter(|row| row.kind() == vibe_spec::EmissionKind::Normal)
            .collect::<Vec<_>>();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].origin(), "org.vibevm/parent");
        assert!(current[0].path().ends_with("boot/STATIC.xml"));
        // Attribution derives from this compile's witness, never from prior
        // c1 strings or escaped prose in its generated initial input.
        assert_eq!(current[0].occurrences(), 1);
        assert_eq!(
            analyzed.artifact.bytes(),
            fs::read(
                f.workspace
                    .root
                    .join(vibe_core::layout::current_boot_static_xml())
            )
            .unwrap()
        );
    }
    #[test]
    fn generated_analysis_refuses_missing_invalid_and_stale_freshness_evidence() {
        let f = generated_fixture();
        crate::install::tests_analyze_parity::publish_resolution_lock(
            &f.workspace.root,
            &f.resolution,
        );
        let parent = &f.resolution[0];
        let slot = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        );
        let index = slot.join(vibe_core::layout::current_boot_index());
        let original = fs::read(&index).unwrap();
        let analyze = || {
            install::analyze_node_lane_with_source_format(&f.workspace, ".", None, SpecFormat::Xml)
        };
        assert!(analyze().unwrap().is_some());
        fs::remove_file(&index).unwrap();
        assert!(
            analyze()
                .unwrap_err()
                .to_string()
                .contains("generated artifact is absent")
        );
        fs::write(&index, b"unproved index").unwrap();
        assert!(
            analyze()
                .unwrap_err()
                .to_string()
                .contains("freshness evidence")
        );
        let text = std::str::from_utf8(&original).unwrap();
        let fingerprint = boot_artifacts::publication::read_unit_index_freshness(text)
            .unwrap()
            .fingerprint;
        fs::write(
            &index,
            text.replace(&fingerprint, &"0".repeat(fingerprint.len())),
        )
        .unwrap();
        assert!(
            analyze()
                .unwrap_err()
                .to_string()
                .contains("freshness evidence")
        );
        fs::write(&index, original).unwrap();
        assert!(analyze().unwrap().is_some());
        let static_path = slot.join(vibe_core::layout::current_boot_static_xml());
        let static_bytes = fs::read(&static_path).unwrap();
        fs::remove_file(&static_path).unwrap();
        assert!(
            analyze()
                .unwrap_err()
                .to_string()
                .contains("generated artifact is absent")
        );
        fs::write(&static_path, static_bytes).unwrap();
        let stale = slot.join(vibe_core::layout::current_boot_static_md());
        fs::write(&stale, b"obsolete representation").unwrap();
        assert!(
            analyze()
                .unwrap_err()
                .to_string()
                .contains("stale representation sibling")
        );
        fs::remove_file(&stale).unwrap();
        assert!(analyze().unwrap().is_some());
    }
}
