//! T4 carriage tests (`R4-TRANSFORM-PLAN-ABI-v0.1.md` §7.1): every legacy
//! constructor pins the empty transform plan, and whole-value attachment
//! preserves the semantic members without exposing any entry/order surface.

use specmark::verifies;

use super::*;
use crate::SpecAddress;
use crate::compiler::transform::carriage::one_document_transform;
use crate::compiler::transform::plan::TransformPlan;

fn seed() -> SpecAddress {
    SpecAddress::parse("spec://org.demo/pkg/boot/entry#root").unwrap()
}

fn lane_plan() -> ArtifactPlan {
    ArtifactPlan::static_lane(
        ArtifactTarget::StaticMarkdown,
        "vibevm/vibespecs/boot/STATIC.md",
        "vibevm/vibespecs",
        vec![ArtifactInput::normal("org.demo/pkg", "boot/entry.md", seed()).unwrap()],
    )
    .unwrap()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn every_legacy_constructor_pins_the_empty_transform_plan() {
    let empty = TransformPlan::empty();

    let lane = lane_plan();
    assert_eq!(lane.transforms(), &empty);
    assert!(lane.transforms().is_empty());
    assert_eq!(lane.transforms().digest(), None);
    assert!(lane.transforms().entries().is_empty());

    // The raw validated constructor takes the same pin, not only its wrappers.
    let raw = ArtifactPlan::new(lane.context().clone(), Vec::new()).unwrap();
    assert_eq!(raw.transforms(), &empty);

    let compatibility = ArtifactPlan::compatibility(seed(), StaticCompileMode::Plain);
    assert_eq!(compatibility.transforms(), &empty);

    let custom = ArtifactPlan::custom_for_test("opaque-test", Vec::new()).unwrap();
    assert_eq!(custom.transforms(), &empty);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn whole_value_attachment_preserves_members_and_compares_by_carriage() {
    let base = lane_plan();
    let transforms = one_document_transform();
    assert!(!transforms.is_empty());

    let carried = base.clone().with_transforms(transforms.clone());
    assert_eq!(carried.context(), base.context());
    assert_eq!(carried.contributions(), base.contributions());
    assert_eq!(carried.transforms(), &transforms);
    assert_ne!(carried, base);

    // Equality is semantic: two independently built lanes carrying the same
    // plan agree, and clone preserves the carriage.
    let again = lane_plan().with_transforms(transforms.clone());
    assert_eq!(carried, again);
    assert_eq!(carried.clone(), carried);

    // Re-attaching the empty plan restores the constructed identity exactly.
    assert_eq!(carried.with_transforms(TransformPlan::empty()), base);
}
