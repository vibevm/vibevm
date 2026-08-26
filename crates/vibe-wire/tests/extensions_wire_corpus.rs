use vibe_wire::generated::extensions_report::{ExtensionsReport, Handler, State};

fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../formats/corpora/extensions/e1/report.json"
    ))
    .unwrap()
}

#[test]
fn exhaustive_report_corpus_round_trips_every_handler_and_state_shape() {
    let value = corpus();
    let report: ExtensionsReport = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(report.count, 5);
    assert_eq!(report.declarations.len(), 5);
    assert_eq!(report.effective_count, 2);
    assert!(report.declarations[4].authored_config.is_none());
    assert!(
        report.declarations[4]
            .effective_config
            .as_ref()
            .is_some_and(|config| config.is_empty())
    );
    assert!(matches!(
        report.declarations[0].handler,
        Handler::Builtin(_)
    ));
    assert!(matches!(report.declarations[1].handler, Handler::Native(_)));
    assert!(matches!(report.declarations[2].handler, Handler::Agent(_)));
    assert!(matches!(report.declarations[3].handler, Handler::Binary(_)));
    assert!(matches!(report.declarations[4].handler, Handler::Script(_)));
    assert_eq!(report.declarations[0].state, State::Disabled);
    assert_eq!(report.declarations[1].state, State::Inactive);
    assert_eq!(report.declarations[2].state, State::SelectorMismatch);
    assert_eq!(report.declarations[3].state, State::Effective);
    let native = report.declarations[1].native.as_ref().unwrap();
    assert_eq!(native.build_state, "unavailable");
    assert!(native.artifact_path.is_none());
    assert!(native.content_hash.is_none());

    assert_eq!(serde_json::to_value(report).unwrap(), value);
}

#[test]
fn required_nullable_reader_refuses_a_missing_null_capable_field() {
    let mut missing = corpus();
    missing["declarations"][0]
        .as_object_mut()
        .unwrap()
        .remove("authored_config");
    assert!(serde_json::from_value::<ExtensionsReport>(missing).is_err());
}

#[test]
fn closed_domain_vocabularies_reject_unknown_foreign_values() {
    for (pointer, impossible) in [
        ("/declarations/0/provider/kind", "widget"),
        ("/declarations/1/pass/kind", "sideways"),
        ("/declarations/1/pass/level", "bytecode"),
    ] {
        let mut invalid = corpus();
        *invalid
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing corpus pointer {pointer}")) =
            serde_json::Value::String(impossible.into());
        assert!(
            serde_json::from_value::<ExtensionsReport>(invalid).is_err(),
            "foreign parser accepted `{impossible}` at `{pointer}`"
        );
    }
}
