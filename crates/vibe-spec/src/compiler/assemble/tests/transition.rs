use super::*;

#[test]
fn transition_rejects_context_provenance_rename_request_and_simple_origin_mutations() {
    let (closure, lane) = lane_fixture();
    let mut provenance = lane.clone();
    let LaneContribution::Elided { meta } = &mut provenance.contributions[2] else {
        unreachable!()
    };
    meta.origin = "changed/elided".to_string();
    assert!(validate_assembled_transition(&closure, &provenance).is_err());

    let mut renames = lane.clone();
    renames.frame.renames.clear();
    assert!(validate_assembled_transition(&closure, &renames).is_err());

    let mut request = lane.clone();
    let LaneContribution::Normal { chunks, .. } = &mut request.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[1] else {
        unreachable!()
    };
    let LaneNode::Normal {
        requested_address, ..
    } = node.as_mut()
    else {
        unreachable!()
    };
    *requested_address = spec("spec://org.demo/a/boot/a#root~r8");
    assert!(validate_lane(&request).is_ok());
    assert!(validate_assembled_transition(&closure, &request).is_err());

    let mut simple_origin = lane.clone();
    let LaneContribution::Simple { chunks, .. } = &mut simple_origin.contributions[1] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[0] else {
        unreachable!()
    };
    let LaneNode::Simple {
        origin: node_origin,
        ..
    } = node.as_mut()
    else {
        unreachable!()
    };
    *node_origin = "meta/simple".to_string();
    assert!(validate_lane(&simple_origin).is_ok());
    assert!(validate_assembled_transition(&closure, &simple_origin).is_err());

    let mut body = lane.clone();
    let LaneContribution::Simple { chunks, .. } = &mut body.contributions[1] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[0] else {
        unreachable!()
    };
    let LaneNode::Simple {
        body: linked_body,
        embeds,
        ..
    } = node.as_mut()
    else {
        unreachable!()
    };
    let rewritten = format!("{linked_body}X");
    *embeds = embeds.rewrite_lines(linked_body, &rewritten).unwrap();
    *linked_body = rewritten;
    assert!(validate_lane(&body).is_ok());
    assert!(validate_assembled_transition(&closure, &body).is_err());

    // A context change the lane itself cannot see: the target stays structured
    // and the frame still matches, so `validate_lane` is blind and only the
    // transition catches the drift.
    let changed_context = ArtifactContext::new(
        ArtifactId::new("static-xml").unwrap(),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/OTHER.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();
    let mut changed_frame = lane.frame.clone();
    changed_frame.generated_path = Some("vibevm/vibespecs/boot/OTHER.xml".to_string());
    let context = LaneIr::assembled(
        changed_context,
        lane.source_node_count,
        lane.source_link_digest.clone(),
        changed_frame,
        lane.contributions.clone(),
    );
    assert!(validate_lane(&context).is_ok());
    assert!(validate_assembled_transition(&closure, &context).is_err());
}
