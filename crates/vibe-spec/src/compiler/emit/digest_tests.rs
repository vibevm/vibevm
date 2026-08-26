use super::digest::lane_digest;
use super::tests::{Source, full_lane};
use crate::SpecAddress;
use crate::compiler::builtin::compile_artifact_lane;
use crate::compiler::ir::{
    ArtifactPlan, LaneChunk, LaneContribution, LaneIr, LaneNode, LinkFenceSnapshot, LinkMarkerKey,
    StaticCompileMode,
};

fn differs(base: &LaneIr, changed: &LaneIr, field: &str) {
    assert_ne!(
        lane_digest(base),
        lane_digest(changed),
        "digest omitted {field}"
    );
}

#[test]
fn every_contribution_and_chunk_field_is_load_bearing_in_the_lane_digest() {
    let base = full_lane();

    let mut simple_meta_path = base.clone();
    let LaneContribution::Simple { meta, .. } = &mut simple_meta_path.contributions[0] else {
        unreachable!()
    };
    meta.path.push('x');
    differs(&base, &simple_meta_path, "simple meta path");

    for field in 0..7 {
        let mut changed = base.clone();
        let LaneContribution::Simple {
            address, chunks, ..
        } = &mut changed.contributions[0]
        else {
            unreachable!()
        };
        let LaneChunk::Node(node) = &mut chunks[0] else {
            unreachable!()
        };
        let LaneNode::Simple {
            contribution,
            occurrence,
            address: node_address,
            origin,
            fence_before,
            fence_after,
            ..
        } = node.as_mut()
        else {
            unreachable!()
        };
        match field {
            0 => *contribution += 1,
            1 => *occurrence += 1,
            2 => {
                *address = crate::compiler::ir::DocumentAddress::StaticEntry {
                    origin: "changed".to_string(),
                    path: "boot/a.md".to_string(),
                }
            }
            3 => {
                *node_address = crate::compiler::ir::DocumentAddress::StaticEntry {
                    origin: "changed".to_string(),
                    path: "boot/a.md".to_string(),
                }
            }
            4 => origin.push('x'),
            5 => {
                *fence_before = LinkFenceSnapshot::Open {
                    delimiter: '`',
                    run: 3,
                }
            }
            6 => {
                *fence_after = LinkFenceSnapshot::Open {
                    delimiter: '~',
                    run: 4,
                }
            }
            _ => unreachable!(),
        }
        differs(&base, &changed, "simple identity/fence field");
    }

    let mut elided_origin = base.clone();
    let LaneContribution::Elided { meta } = &mut elided_origin.contributions[2] else {
        unreachable!()
    };
    meta.origin.push('x');
    differs(&base, &elided_origin, "elided meta origin");
    let mut hoisted_meta = base.clone();
    let LaneContribution::Hoisted { meta, .. } = &mut hoisted_meta.contributions[3] else {
        unreachable!()
    };
    meta.origin.push('x');
    differs(&base, &hoisted_meta, "hoisted meta");

    let normal = compile_artifact_lane(
        ArtifactPlan::compatibility(
            SpecAddress::parse("spec://org.demo/pkg/boot/entry#root~r7").unwrap(),
            StaticCompileMode::Plain,
        ),
        &Source("# Entry {#root}\n"),
    )
    .unwrap();
    for field in 0..3 {
        let mut changed = normal.clone();
        let LaneContribution::Normal {
            meta,
            seed,
            seed_address,
            ..
        } = &mut changed.contributions[0]
        else {
            unreachable!()
        };
        match field {
            0 => meta.origin.push('x'),
            1 => seed.0 += 1,
            2 => {
                *seed_address =
                    SpecAddress::parse("spec://org.demo/pkg/boot/entry#root~r8").unwrap()
            }
            _ => unreachable!(),
        }
        differs(&normal, &changed, "normal contribution field");
    }

    for field in 0..3 {
        let mut changed = normal.clone();
        let LaneContribution::Normal { chunks, .. } = &mut changed.contributions[0] else {
            unreachable!()
        };
        let LaneChunk::NormalOpen {
            contribution,
            occurrence,
            marker,
        } = &mut chunks[0]
        else {
            unreachable!()
        };
        match field {
            0 => *contribution += 1,
            1 => *occurrence += 1,
            2 => {
                *marker = LinkMarkerKey::from_address(
                    &SpecAddress::parse("spec://org.demo/pkg/boot/open#root").unwrap(),
                )
            }
            _ => unreachable!(),
        }
        differs(&normal, &changed, "normal open field");
    }

    for field in 0..9 {
        let mut changed = normal.clone();
        let LaneContribution::Normal { chunks, .. } = &mut changed.contributions[0] else {
            unreachable!()
        };
        let LaneChunk::Node(node) = &mut chunks[1] else {
            unreachable!()
        };
        let LaneNode::Normal {
            contribution,
            occurrence,
            node,
            requested_address,
            origin,
            marker,
            fence_before,
            fence_after,
            body,
        } = node.as_mut()
        else {
            unreachable!()
        };
        match field {
            0 => *contribution += 1,
            1 => *occurrence += 1,
            2 => node.0 += 1,
            3 => {
                *requested_address =
                    SpecAddress::parse("spec://org.demo/pkg/boot/entry#root~r8").unwrap()
            }
            4 => origin.push('x'),
            5 => {
                *marker = LinkMarkerKey::from_address(
                    &SpecAddress::parse("spec://org.demo/pkg/boot/node#root").unwrap(),
                )
            }
            6 => {
                *fence_before = LinkFenceSnapshot::Open {
                    delimiter: '`',
                    run: 3,
                }
            }
            7 => {
                *fence_after = LinkFenceSnapshot::Open {
                    delimiter: '~',
                    run: 4,
                }
            }
            8 => body.push('x'),
            _ => unreachable!(),
        }
        differs(&normal, &changed, "normal node field");
    }

    let newline_index = normal.contributions[0]
        .chunks()
        .iter()
        .position(|chunk| matches!(chunk, LaneChunk::ForcedNewline { .. }))
        .unwrap();
    for field in 0..2 {
        let mut changed = normal.clone();
        let LaneContribution::Normal { chunks, .. } = &mut changed.contributions[0] else {
            unreachable!()
        };
        let LaneChunk::ForcedNewline {
            contribution,
            occurrence,
        } = &mut chunks[newline_index]
        else {
            unreachable!()
        };
        if field == 0 {
            *contribution += 1;
        } else {
            *occurrence += 1;
        }
        differs(&normal, &changed, "forced newline field");
    }

    for field in 0..3 {
        let mut changed = normal.clone();
        let LaneContribution::Normal { chunks, .. } = &mut changed.contributions[0] else {
            unreachable!()
        };
        let LaneChunk::NormalClose {
            contribution,
            occurrence,
            marker,
        } = chunks.last_mut().unwrap()
        else {
            unreachable!()
        };
        match field {
            0 => *contribution += 1,
            1 => *occurrence += 1,
            2 => {
                *marker = LinkMarkerKey::from_address(
                    &SpecAddress::parse("spec://org.demo/pkg/boot/close#root").unwrap(),
                )
            }
            _ => unreachable!(),
        }
        differs(&normal, &changed, "normal close field");
    }
}

trait ContributionChunks {
    fn chunks(&self) -> &[LaneChunk];
}

impl ContributionChunks for LaneContribution {
    fn chunks(&self) -> &[LaneChunk] {
        match self {
            LaneContribution::Normal { chunks, .. } | LaneContribution::Simple { chunks, .. } => {
                chunks
            }
            LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => &[],
        }
    }
}
