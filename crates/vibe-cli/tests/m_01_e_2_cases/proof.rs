//! Mandatory existing source replay/derivation route, not surface grade fiction.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-QUERY-PURITY-EVIDENCE");
use super::fixture::*;
use vibe_core::manifest::SpecFormat;
use vibe_install::{PreparedPackageOverlay, source_strength, verify_original_source_capture};
use vibe_workspace::source_query::{SourceQueryUnavailable, SourceStrength};
#[test]
fn verified_original_and_derivation_refuse_missing_and_coordinated_poison() {
    for format in [SpecFormat::Mixed, SpecFormat::Xml] {
        let f = Fixture::new(format);
        let world = f.world(format);
        let captured = f.capture();
        let before = state(f.root.path());
        eprintln!("E2_QUERY_BEGIN mandatory-replay");
        let proof =
            verify_original_source_capture(&f.original, f.pin.clone(), f.hash.clone(), captured)
                .unwrap();
        eprintln!("E2_QUERY_END mandatory-replay");
        assert!(proof.contains(SOURCE));
        assert_eq!(proof.file_count(), 2);
        eprintln!("E2_QUERY_BEGIN missing-strict-proof");
        let missing = source_strength(&world, Some(("org.demo", "dep")), &f.output, None, true);
        eprintln!("E2_QUERY_END missing-strict-proof");
        assert!(matches!(
            missing,
            Err(SourceQueryUnavailable::SourceProofUnavailable(
                "original source capture"
            ))
        ));
        if format == SpecFormat::Mixed {
            eprintln!("E2_QUERY_BEGIN original-strict-positive");
            assert_eq!(
                source_strength(
                    &world,
                    Some(("org.demo", "dep")),
                    &f.output,
                    Some(&proof),
                    true
                )
                .unwrap(),
                SourceStrength::SourceProven
            );
            eprintln!("E2_QUERY_END original-strict-positive");
        } else {
            let registry = vibe_facts::Registry::load_with_witnesses(&f.host).unwrap();
            let overlay =
                PreparedPackageOverlay::from_registry_snapshot(&registry, "org.demo", "dep")
                    .unwrap();
            eprintln!("E2_QUERY_BEGIN builtin-derivation");
            let derived = proof
                .derive_builtin_output(&world, "org.demo", "dep", &f.output, &overlay)
                .unwrap();
            eprintln!("E2_QUERY_END builtin-derivation");
            assert_eq!(derived.output_path(), f.output);
            assert_ne!(
                derived.original_raw_sha256(),
                derived.representation_sha256()
            );
        }
        let after = state(f.root.path());
        assert_eq!(after, before);
        println!(
            "E2_PROOF_SNAPSHOT {:?} {}",
            format,
            serde_json::json!({"phase":"before","before":before,"after":after})
        );
        let poisoned_output = b"COORDINATED_POISON";
        std::fs::write(f.slot.join(&f.output), poisoned_output).unwrap();
        let mut record = vibe_workspace::vibedeps::read_slot_record(&f.slot).unwrap();
        use sha2::{Digest, Sha256};
        record
            .files
            .iter_mut()
            .find(|row| row.path == f.output)
            .unwrap()
            .sha256 = format!("{:x}", Sha256::digest(poisoned_output));
        vibe_workspace::vibedeps::write_slot_record(&f.slot, &record).unwrap();
        let poisoned_world = f.world(format);
        let coordinated_state = state(f.root.path());
        eprintln!("E2_QUERY_BEGIN coordinated-proof-refusal");
        if format == SpecFormat::Mixed {
            assert!(matches!(
                source_strength(
                    &poisoned_world,
                    Some(("org.demo", "dep")),
                    &f.output,
                    Some(&proof),
                    true
                ),
                Err(SourceQueryUnavailable::SourceProofUnavailable(
                    "original source membership or derivation"
                ))
            ));
        } else {
            let registry = vibe_facts::Registry::load_with_witnesses(&f.host).unwrap();
            let overlay =
                PreparedPackageOverlay::from_registry_snapshot(&registry, "org.demo", "dep")
                    .unwrap();
            assert!(matches!(
                proof.derive_builtin_output(
                    &poisoned_world,
                    "org.demo",
                    "dep",
                    &f.output,
                    &overlay
                ),
                Err(SourceQueryUnavailable::DerivationUnavailable(
                    "materialized output differs from builtin replay"
                ))
            ));
        }
        eprintln!("E2_QUERY_END coordinated-proof-refusal");
        let after = state(f.root.path());
        assert_eq!(after, coordinated_state);
        println!(
            "E2_PROOF_SNAPSHOT {:?} {}",
            format,
            serde_json::json!({"phase":"coordinated_state","before":coordinated_state,"after":after})
        );
        let pristine_hash = f.hash.clone();
        std::fs::write(
            f.original.join(SOURCE),
            ORIGINAL.replace("Original.", "POISONED."),
        )
        .unwrap();
        let captured = f.capture();
        let poisoned_state = state(f.root.path());
        eprintln!("E2_QUERY_BEGIN poisoned-original-replay");
        let rejected =
            verify_original_source_capture(&f.original, f.pin.clone(), pristine_hash, captured);
        eprintln!("E2_QUERY_END poisoned-original-replay");
        assert!(matches!(
            rejected,
            Err(SourceQueryUnavailable::SourceHashMismatch)
        ));
        let after = state(f.root.path());
        assert_eq!(after, poisoned_state);
        println!(
            "E2_PROOF_SNAPSHOT {:?} {}",
            format,
            serde_json::json!({"phase":"poisoned_state","before":poisoned_state,"after":after})
        );
    }
}
