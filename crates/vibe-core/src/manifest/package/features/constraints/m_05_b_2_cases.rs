use super::super::super::FeaturesTable;
use super::*;

fn f(s: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(s).unwrap()
}
fn definition() -> (Constraints, FeatureProgram) {
    let program = FeatureProgram::lower_table(&FeaturesTable {
        features: [
            ("a".into(), vec!["b".into()]),
            ("b".into(), vec!["a".into()]),
        ]
        .into(),
        exclusive: BTreeMap::new(),
    })
    .unwrap();
    let constraints = Constraints {
        groups: [(
            f("choice"),
            ConstrainedGroup {
                cardinality: Cardinality::ExactlyOne,
                members: [f("a"), f("b")].into(),
                guards: vec![],
                defaults: Some([f("a")].into()),
            },
        )]
        .into(),
        member_guards: BTreeMap::new(),
    };
    (constraints, program)
}
#[test]
fn four_cardinalities() {
    for (cardinality, accepted) in [
        (Cardinality::ZeroOrMore, [true, true, true]),
        (Cardinality::OneOrMore, [false, true, true]),
        (Cardinality::ZeroOrOne, [true, true, false]),
        (Cardinality::ExactlyOne, [false, true, false]),
    ] {
        for (count, expected) in accepted.into_iter().enumerate() {
            assert_eq!(cardinality.accepts(count), expected);
        }
    }
}
#[test]
fn guards_refuse_foreign_expressions_unknown_and_cycles() {
    for (kind, name) in [
        ("member", "other/a"),
        ("member", "a && b"),
        ("not", "a"),
        ("expression", "a"),
    ] {
        assert!(Guard::parse(kind, name).is_err());
    }
    let (mut c, p) = definition();
    c.groups
        .get_mut(&f("choice"))
        .unwrap()
        .guards
        .push(Guard::Member(f("missing")));
    assert!(matches!(
        c.order(&p, &FeatureDeclarations::default()),
        Err(ConstraintError::Invalid(_))
    ));
    c.groups.get_mut(&f("choice")).unwrap().guards = vec![Guard::Member(f("a"))];
    assert!(matches!(
        c.order(&p, &FeatureDeclarations::default()),
        Err(ConstraintError::GuardCycle(_))
    ));
}
#[test]
fn positive_guard_order_preserves_feature_cycles() {
    let (mut c, p) = definition();
    assert_eq!(
        c.order(&p, &FeatureDeclarations::default()).unwrap().len(),
        3
    );
    c.member_guards.insert(f("b"), vec![Guard::Member(f("a"))]);
    let order = c.order(&p, &FeatureDeclarations::default()).unwrap();
    assert!(
        order
            .iter()
            .position(|n| *n == Guard::Member(f("a")))
            .unwrap()
            < order
                .iter()
                .position(|n| *n == Guard::Member(f("b")))
                .unwrap()
    );
    c.member_guards.insert(f("a"), vec![Guard::Member(f("b"))]);
    assert!(matches!(
        c.order(&p, &FeatureDeclarations::default()),
        Err(ConstraintError::GuardCycle(_))
    ));
}
#[test]
fn definitions_and_explicit_empty_are_distinct() {
    assert_ne!(Selection::Omitted, Selection::Explicit(BTreeSet::new()));
    let (mut c, p) = definition();
    c.groups.get_mut(&f("choice")).unwrap().defaults = Some(BTreeSet::new());
    assert!(c.order(&p, &FeatureDeclarations::default()).is_err());
    c.groups.get_mut(&f("choice")).unwrap().defaults = None;
    c.member_guards.insert(f("ghost"), vec![]);
    assert!(c.order(&p, &FeatureDeclarations::default()).is_err());
}
