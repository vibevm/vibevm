use super::*;
use crate::EmbedError;
use crate::compiler::builtin::ArtifactCompileError;
use crate::compiler::ir::ClosureEdgeKind;

fn static_context(target: ArtifactTarget) -> ArtifactContext {
    let (id, path) = if target == ArtifactTarget::StaticMarkdown {
        ("static-md", "vibevm/vibespecs/boot/STATIC.md")
    } else {
        ("static-xml", "vibevm/vibespecs/boot/STATIC.xml")
    };
    ArtifactContext::new(
        ArtifactId::new(id).unwrap(),
        target,
        ArtifactFrame::StaticLane {
            generated_path: path.to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap()
}

fn normal_input(origin: &str, path: &str, address: &str) -> ArtifactInput {
    ArtifactInput::normal(origin, path, spec(address)).unwrap()
}

fn linked(closure: &super::super::ir::ClosureIr) -> &super::super::ir::LinkResult {
    let LinkState::Linked(link) = &closure.link else {
        unreachable!()
    };
    link
}

fn point_source_closure(second_pin: u32) -> (super::super::ir::ClosureIr, CountingSource) {
    let alpha = "spec://org.demo/alpha/boot/root#root";
    let omega = "spec://org.demo/omega/boot/root#root";
    let shared = "spec://org.demo/shared/source/impl#root";
    let source = CountingSource::with(&[
        (
            alpha,
            &format!("# Alpha {{#root}}\n#source {shared}~r7\nALPHA\n"),
        ),
        (shared, "# Shared {#root}\nSHARED\n"),
        (
            omega,
            &format!("# Omega {{#root}}\n#source {shared}~r{second_pin}\nOMEGA\n"),
        ),
    ]);
    let plan = ArtifactPlan::new(
        static_context(ArtifactTarget::StaticMarkdown),
        vec![
            normal_input("org.demo/alpha", "boot/alpha", alpha),
            normal_input("org.demo/omega", "boot/omega", omega),
        ],
    )
    .unwrap();
    let closure = compile_artifact_prefix(plan, &source).unwrap();
    (closure, source)
}

#[test]
fn pin_distinct_point_sources_share_intrinsic_document_work_but_keep_exact_edges() {
    reset_parse_invocations();
    let (closure, source) = point_source_closure(9);
    let shared = "spec://org.demo/shared/source/impl#root";

    assert_eq!(source.load_count(shared), 1);
    assert_eq!(parse_invocations(), 3);
    assert_eq!(closure.nodes.len(), 3);
    assert_eq!(
        closure
            .edges
            .iter()
            .filter(|edge| edge.kind == ClosureEdgeKind::Source)
            .map(|edge| edge.requested_target.pinned_r)
            .collect::<Vec<_>>(),
        [Some(7), Some(9)]
    );
}

#[test]
fn point_source_pin_changes_link_digest_and_edge_replay() {
    let (closure, _) = point_source_closure(9);
    let original_digest = linked(&closure).input_digest.clone();
    let (changed, _) = point_source_closure(10);
    assert_ne!(original_digest, linked(&changed).input_digest);

    let mut replay_drift = closure.clone();
    let edge = replay_drift
        .edges
        .iter_mut()
        .find(|edge| {
            edge.kind == ClosureEdgeKind::Source && edge.requested_target.pinned_r == Some(9)
        })
        .unwrap();
    edge.requested_target = spec("spec://org.demo/shared/source/impl#root~r10");
    assert!(validate_linked(&replay_drift).is_err());
}

#[test]
fn pin_distinct_seeds_and_dependencies_share_nodes_but_keep_exact_requests() {
    let seed = "spec://org.demo/shared/boot/root#root";
    let source = CountingSource::with(&[(seed, "# Root {#root}\nROOT\n")]);
    let plan = ArtifactPlan::new(
        static_context(ArtifactTarget::StaticMarkdown),
        vec![
            normal_input("org.demo/shared", "boot/r7", &format!("{seed}~r7")),
            normal_input("org.demo/shared", "boot/r9", &format!("{seed}~r9")),
        ],
    )
    .unwrap();
    let closure = compile_artifact_prefix(plan, &source).unwrap();
    assert_eq!(source.load_count(seed), 1);
    assert_eq!(closure.nodes.len(), 1);
    let seeds = closure
        .contributions
        .iter()
        .map(|contribution| match contribution {
            ClosureContribution::Normal { seed_address, .. } => seed_address.pinned_r,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(seeds, [Some(7), Some(9)]);

    let alpha = "spec://org.demo/alpha/boot/root#root";
    let omega = "spec://org.demo/omega/boot/root#root";
    let dep = "spec://org.demo/dep/boot/shared#root";
    let source = CountingSource::with(&[
        (alpha, &format!("# Alpha {{#root}}\n#use {dep}~r7\nALPHA\n")),
        (dep, "# Dep {#root}\nDEP\n"),
        (omega, &format!("# Omega {{#root}}\n#use {dep}~r9\nOMEGA\n")),
    ]);
    let plan = ArtifactPlan::new(
        static_context(ArtifactTarget::StaticMarkdown),
        vec![
            normal_input("org.demo/alpha", "boot/alpha", alpha),
            normal_input("org.demo/omega", "boot/omega", omega),
        ],
    )
    .unwrap();
    let closure = compile_artifact_prefix(plan, &source).unwrap();
    assert_eq!(source.load_count(dep), 1);
    assert_eq!(
        closure
            .edges
            .iter()
            .filter(|edge| edge.requested_target.without_pin() == dep)
            .map(|edge| edge.requested_target.pinned_r)
            .collect::<Vec<_>>(),
        [Some(7), Some(9)]
    );
    let dep_occurrences = linked(&closure)
        .occurrences
        .iter()
        .filter_map(|occurrence| match occurrence {
            LinkOccurrence::Normal {
                address, marker, ..
            } if address.without_pin() == dep => Some((address.pinned_r, marker.as_str())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        dep_occurrences,
        [(Some(7), dep), (Some(9), dep)],
        "markers stay pinless while exact requests retain pins"
    );

    let original_digest = linked(&closure).input_digest.clone();
    let mut pin_drift = closure.clone();
    let ClosureContribution::Normal { emission_order, .. } = &mut pin_drift.contributions[1] else {
        unreachable!()
    };
    emission_order[0].requested_address = spec(&format!("{dep}~r10"));
    assert!(validate_linked(&pin_drift).is_err());

    let mut edge_drift = closure.clone();
    edge_drift.edges[1].requested_target = spec(&format!("{dep}~r10"));
    assert!(validate_linked(&edge_drift).is_err());

    let source = CountingSource::with(&[
        (alpha, &format!("# Alpha {{#root}}\n#use {dep}~r7\nALPHA\n")),
        (dep, "# Dep {#root}\nDEP\n"),
        (
            omega,
            &format!("# Omega {{#root}}\n#use {dep}~r10\nOMEGA\n"),
        ),
    ]);
    let changed = compile_artifact_prefix(
        ArtifactPlan::new(
            static_context(ArtifactTarget::StaticMarkdown),
            vec![
                normal_input("org.demo/alpha", "boot/alpha", alpha),
                normal_input("org.demo/omega", "boot/omega", omega),
            ],
        )
        .unwrap(),
        &source,
    )
    .unwrap();
    assert_ne!(original_digest, linked(&changed).input_digest);
}

#[test]
fn simple_embed_discovery_uses_plan_order_not_lexical_identity_order() {
    let z_target = "spec://org.demo/z/boot/piece#root";
    let a_target = "spec://org.demo/a/boot/piece#root";
    let source = CountingSource::with(&[(z_target, "Z"), (a_target, "A")]);
    let plan = ArtifactPlan::new(
        static_context(ArtifactTarget::StaticMarkdown),
        vec![
            simple(
                "z-host",
                "boot/z.md",
                &format!("# Z {{#root}}\n#embed {z_target}\n"),
            ),
            simple(
                "a-host",
                "boot/a.md",
                &format!("# A {{#root}}\n#embed {a_target}\n"),
            ),
        ],
    )
    .unwrap();
    let closure = compile_artifact_prefix(plan, &source).unwrap();
    assert_eq!(
        source.load_order(),
        [z_target.to_string(), a_target.to_string()]
    );
    assert!(matches!(
        closure.contributions.as_slice(),
        [ClosureContribution::Simple { meta: first, .. }, ClosureContribution::Simple { meta: second, .. }]
            if first.origin == "z-host" && second.origin == "a-host"
    ));
}

#[test]
fn embed_failure_precedence_follows_interleaved_plan_roots() {
    let first = "spec://org.demo/first/boot/root#root";
    let last = "spec://org.demo/last/boot/root#root";
    let simple_missing = "spec://org.demo/z/boot/missing#root";
    let later_missing = "spec://org.demo/a/boot/missing#root";
    let source = CountingSource::with(&[
        (first, "# First {#root}\nFIRST\n"),
        (last, &format!("# Last {{#root}}\n#embed {later_missing}\n")),
    ]);
    let plan = ArtifactPlan::new(
        static_context(ArtifactTarget::StaticMarkdown),
        vec![
            normal_input("org.demo/first", "boot/first", first),
            simple(
                "z-host",
                "boot/z.md",
                &format!("# Z {{#root}}\n#embed {simple_missing}\n"),
            ),
            normal_input("org.demo/last", "boot/last", last),
        ],
    )
    .unwrap();
    assert!(matches!(
        compile_artifact_prefix(plan, &source),
        Err(ArtifactCompileError::Compile(
            CompileError::Embed(EmbedError::Unresolved { addr, .. })
        )) if addr == simple_missing
    ));
}

#[test]
fn simple_aliases_fail_at_plan_validation_without_resolver_or_gather() {
    for authored in [
        "#use spec://org.demo/dep/boot/entry#root as X\n",
        "undeclared @!X\n",
    ] {
        reset_counters();
        let source = CountingSource::default();
        let error = ArtifactPlan::new(
            static_context(ArtifactTarget::StaticMarkdown),
            vec![simple("host", "boot/local.md", authored)],
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "the simple package `host` carries alias machinery (`#use … as` / `@!`) that is `normal`-format only (PROP-035 §7.2); convert the package to `format = \"normal\"` or drop the alias"
        );
        assert_eq!(parse_invocations(), 0);
        assert_eq!(gather_invocations(), 0);
        assert!(source.load_order().is_empty());
    }
}

#[test]
fn duplicate_simple_identity_requires_equal_source_and_keeps_two_occurrences() {
    let first = simple("host", "boot/local.md", "# Same {#root}\nSAME\n");
    let second = first.clone();
    reset_counters();
    let closure = compile_artifact_prefix(
        ArtifactPlan::new(
            static_context(ArtifactTarget::StaticMarkdown),
            vec![
                first,
                ArtifactInput::elided("separator", "boot/sep").unwrap(),
                second,
            ],
        )
        .unwrap(),
        &CountingSource::default(),
    )
    .unwrap();
    assert_eq!(parse_invocations(), 1);
    assert!(matches!(
        closure.contributions.as_slice(),
        [
            ClosureContribution::Simple { .. },
            ClosureContribution::Elided { .. },
            ClosureContribution::Simple { .. }
        ]
    ));
    assert_eq!(
        linked(&closure)
            .occurrences
            .iter()
            .filter(|occurrence| matches!(occurrence, LinkOccurrence::Simple { .. }))
            .count(),
        2
    );

    let error = ArtifactPlan::new(
        static_context(ArtifactTarget::StaticMarkdown),
        vec![
            simple("host", "boot/local.md", "FIRST"),
            simple("host", "boot/local.md", "SECOND"),
        ],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        super::super::ir::ArtifactPlanError::ConflictingSimpleIdentity {
            first: 0,
            second: 1,
            ..
        }
    ));
}

#[test]
fn artifact_context_accepts_only_engine_tuples_and_survives_to_link() {
    assert!(ArtifactId::new("static-md\nforged").is_err());
    assert!(ArtifactId::new("static-md\0forged").is_err());

    let frames = [
        ArtifactFrame::CompatibilityFragment,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.md".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
    ];
    for id in ["static-fragment", "static-md", "static-xml", "other"] {
        for target in [&ArtifactTarget::StaticMarkdown, &ArtifactTarget::StaticXml] {
            for frame in &frames {
                for mode in [StaticCompileMode::Plain, StaticCompileMode::QualifyPerNode] {
                    let valid = match (id, target, frame, mode) {
                        ("static-fragment", target, ArtifactFrame::CompatibilityFragment, _)
                            if *target == ArtifactTarget::StaticMarkdown =>
                        {
                            true
                        }
                        (
                            "static-md",
                            target,
                            ArtifactFrame::StaticLane { generated_path, .. },
                            StaticCompileMode::QualifyPerNode,
                        ) if *target == ArtifactTarget::StaticMarkdown => {
                            generated_path.ends_with(".md")
                        }
                        (
                            "static-xml",
                            target,
                            ArtifactFrame::StaticLane { generated_path, .. },
                            StaticCompileMode::QualifyPerNode,
                        ) if *target == ArtifactTarget::StaticXml => {
                            generated_path.ends_with(".xml")
                        }
                        _ => false,
                    };
                    let actual = ArtifactContext::new(
                        ArtifactId::new(id).unwrap(),
                        target.clone(),
                        frame.clone(),
                        mode,
                    );
                    assert_eq!(actual.is_ok(), valid, "{id} {target:?} {frame:?} {mode:?}");
                }
            }
        }
    }

    let root = "spec://org.demo/root/boot/entry#root";
    let source = CountingSource::with(&[(root, "# Root {#root}\n")]);
    let plan = ArtifactPlan::new(
        static_context(ArtifactTarget::StaticXml),
        vec![normal_input("org.demo/root", "boot/root", root)],
    )
    .unwrap();
    let expected = plan.context().clone();
    let closure = compile_artifact_prefix(plan, &source).unwrap();
    assert_eq!(closure.context(), &expected);
    assert_eq!(linked(&closure).mode, expected.mode());
}

#[test]
fn normal_root_definitions_and_fences_do_not_leak_to_the_next_root() {
    let a = "spec://org.demo/a/boot/root#root";
    let b = "spec://org.demo/b/boot/root#root";
    let dep = "spec://org.demo/dep/boot/rules#root";
    let source = CountingSource::with(&[
        (a, "# A {#root}\nSee (#ONLY-B).\n```\n"),
        (b, &format!("# B {{#root}}\nSee (#X).\n#use {dep}\n")),
        (dep, "# Rules {#root}\n##X x\n##ONLY-B b\n"),
    ]);
    let closure = compile_artifact_prefix(
        ArtifactPlan::new(
            static_context(ArtifactTarget::StaticMarkdown),
            vec![
                normal_input("org.demo/a", "boot/a", a),
                normal_input("org.demo/b", "boot/b", b),
            ],
        )
        .unwrap(),
        &source,
    )
    .unwrap();
    let link = linked(&closure);
    let a_body = link
        .occurrences
        .iter()
        .find_map(|occurrence| match occurrence {
            LinkOccurrence::Normal { address, body, .. } if address.without_pin() == a => {
                Some(body)
            }
            _ => None,
        })
        .unwrap();
    assert!(a_body.contains("(#ONLY-B)"), "{a_body}");
    let b_occurrence = link
        .occurrences
        .iter()
        .find(|occurrence| {
            matches!(occurrence, LinkOccurrence::Normal { address, .. } if address.without_pin() == b)
        })
        .unwrap();
    let LinkOccurrence::Normal {
        body, fence_before, ..
    } = b_occurrence
    else {
        unreachable!()
    };
    assert!(matches!(
        fence_before,
        super::super::ir::LinkFenceSnapshot::Closed
    ));
    assert!(body.contains("(#org-demo--dep--X)"), "{body}");
}
