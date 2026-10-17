use super::*;
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/valid/representative.toml"
));
const ROLE: &str = "\n[[documents]]\nrole='release-impact'\npath='release-impact.json'\nnormative=false\ninput=true\n";
fn context() -> ChangeContext {
    ChangeContext::for_node(
        &Manifest::parse_str("[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n")
            .unwrap(),
        ".",
        Some(Path::new("custom/specs")),
    )
    .unwrap()
}
#[test]
fn retained_immutable_id_and_evidence_only_registration() {
    let c = context();
    for id in ["preview-accounting-foundation", "legacy_ID-2", ID] {
        let id = ChangeId::parse(id).unwrap();
        let text = format!(
            "schema=1\norigins=[]\n[change]\nid='{}'\nselected_node='.'\nslug='display'\ntitle='Display'\n{ROLE}",
            id.as_str()
        );
        let m = ChangeManifest::parse_str(&text, &c, &c.directory(&id)).unwrap();
        assert!(m.is_evidence_only());
        assert_eq!(
            m.intent_identity(&[]).unwrap_err().kind,
            ChangeErrorKind::MissingRole
        );
        assert_eq!(m.id(), &id);
        let renamed = text.replace("display", "renamed");
        assert_eq!(
            ChangeManifest::parse_str(&renamed, &c, &c.directory(&id))
                .unwrap()
                .id(),
            &id
        );
    }
    assert!(ChangeId::parse("chg-invalid").is_err());
}
#[test]
fn normative_self_closing_escaping_and_unknown_epoch_refuse() {
    let c = context();
    let dir = c.directory(&ChangeId::parse(ID).unwrap());
    for role in [
        ROLE.replace("normative=false", "normative=true"),
        ROLE.replace("input=true", "input=false"),
        ROLE.replace("release-impact.json", "../release-impact.json"),
        ROLE.replace("role='release-impact'", "role='closure'"),
    ] {
        assert!(ChangeManifest::parse_str(&format!("{TEXT}{role}"), &c, &dir).is_err());
    }
    assert!(
        ChangeManifest::parse_str(
            &format!("{TEXT}{ROLE}").replacen("schema = 1", "schema = 9", 1),
            &c,
            &dir
        )
        .is_err()
    );
    assert!(
        ChangeManifest::parse_str(
            &format!("{TEXT}{ROLE}"),
            &c,
            &dir.with_file_name("replacement-id")
        )
        .is_err()
    );
}
#[test]
fn evidence_addition_excluded_from_independent_subjects() {
    let c = context();
    let dir = c.directory(&ChangeId::parse(ID).unwrap());
    let before = ChangeManifest::parse_str(TEXT, &c, &dir).unwrap();
    let after = ChangeManifest::parse_str(&format!("{TEXT}{ROLE}"), &c, &dir).unwrap();
    let inputs = [
        NormativeInput {
            path: "intent.md",
            bytes: b"intent",
        },
        NormativeInput {
            path: "acceptance.md",
            bytes: b"acceptance",
        },
    ];
    assert_eq!(
        before.intent_identity(&inputs).unwrap(),
        after.intent_identity(&inputs).unwrap()
    );
    let graph = WorkGraph::parse_documents(&[
        "schema=1\n[[items]]\nid='work-a'\npredecessors=[]\nneeds=[]\ntouches=[]\n",
    ])
    .unwrap();
    assert_eq!(
        graph.identity(&before).unwrap(),
        graph.identity(&after).unwrap()
    );
    assert_eq!(
        graph.contribution_identity(&before, &["work-a"]).unwrap(),
        graph.contribution_identity(&after, &["work-a"]).unwrap()
    );
    let mut self_subject = inputs.to_vec();
    self_subject.push(NormativeInput {
        path: "release-impact.json",
        bytes: b"evidence",
    });
    assert!(after.intent_identity(&self_subject).is_err());
}

#[test]
fn evidence_only_public_work_subjects_refuse_empty_and_selected_graphs() {
    let c = context();
    let id = ChangeId::parse("preview-accounting-foundation").unwrap();
    let text = format!(
        "schema=1\norigins=[]\n[change]\nid='{}'\nselected_node='.'\nslug='display'\ntitle='Display'\n{ROLE}",
        id.as_str()
    );
    let manifest = ChangeManifest::parse_str(&text, &c, &c.directory(&id)).unwrap();
    let empty = WorkGraph::parse_documents(&[]).unwrap();
    let selected = WorkGraph::parse_documents(&["schema=1\n[[items]]\nid='work-a'\n"]).unwrap();
    for (graph, roots) in [(&empty, vec![]), (&selected, vec!["work-a"])] {
        for error in [
            graph.identity(&manifest).unwrap_err(),
            graph.contribution_identity(&manifest, &roots).unwrap_err(),
        ] {
            assert_eq!(error.kind, ChangeErrorKind::MissingRole);
            assert_eq!(error.field, "work");
        }
    }
}
