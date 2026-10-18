use super::*;
use std::{cell::RefCell, rc::Rc};

struct Fixture {
    calls: Rc<RefCell<Vec<q::Command>>>,
    view: r::BodyInspect,
}
impl Service for Fixture {
    fn query(&self, command: q::Command) -> Result<r::Result, String> {
        self.calls.borrow_mut().push(command.clone());
        let body = match command {
            q::Command::Capture(_) => r::Body::Capture(Box::new(r::BodyCapture {
                capture_id: "frozen".into(),
                resources: vec![],
            })),
            q::Command::Inspect(c) if c.snapshot_id == "frozen" => {
                r::Body::Inspect(Box::new(self.view.clone()))
            }
            _ => return Err("host authority refused observation".into()),
        };
        Ok(r::Result {
            body,
            context_id: "host".into(),
            result_id: "result".into(),
            schema: 1,
            snapshot_id: Some("frozen".into()),
        })
    }
}
fn fixture() -> Fixture {
    Fixture {
        calls: Rc::new(RefCell::new(vec![])),
        view: r::BodyInspect {
            capture_id: "frozen".into(),
            compiler_carrier_format_id: None,
            compiler_carrier_json: None,
            gaps: vec!["source and parent mappings unavailable".into()],
            resources: vec![],
            sources: vec![],
            nodes: (0..1100)
                .map(|i| r::Node {
                    artifact_lines: None,
                    contribution: 0,
                    occurrence: i,
                    origin: "same label".into(),
                    parent: if i > 0 { Some(i - 1) } else { None },
                    path: "same path".into(),
                    requested: None,
                    root: i == 0,
                    source: 99,
                })
                .collect(),
        },
    }
}

#[test]
fn m_06_c_2_cases_viewport_navigation_preserves_typed_occurrences_and_gaps() {
    let service = fixture();
    let original = service.view.clone();
    let mut viewer = Viewer::new(service).unwrap();
    let mut collected = vec![];
    for i in (0..1100).step_by(20) {
        viewer.apply(Action::Select(i)).unwrap();
        collected.extend(viewer.frame(None, None).nodes);
    }
    assert_eq!(collected, original.nodes);
    viewer.apply(Action::Select(1099)).unwrap();
    viewer.apply(Action::Parent).unwrap();
    assert_eq!(viewer.frame(None, None).selected, Some(1098));
    viewer.apply(Action::Child).unwrap();
    assert_eq!(viewer.frame(None, None).selected, Some(1099));
    assert!(viewer.apply(Action::Next).is_err());
    assert_eq!(viewer.frame(None, None).gaps, original.gaps);
    assert!(viewer.apply(Action::Select(1100)).is_err());
    assert_eq!(viewer.frame(None, None).selected, Some(1099));
}

#[test]
fn m_06_c_2_cases_missing_source_and_refused_domain_queries_never_mint_authority() {
    let service = fixture();
    let calls = service.calls.clone();
    let mut viewer = Viewer::new(service).unwrap();
    assert!(
        viewer
            .apply(Action::Source(0))
            .unwrap_err()
            .contains("mapping unavailable")
    );
    assert!(viewer.apply(Action::Snapshot(99)).is_err());
    assert!(viewer.apply(Action::Compare(0, 99)).is_err());
    assert_eq!(calls.borrow().len(), 2);
    viewer.apply(Action::Select(21)).unwrap();
    assert!(
        viewer
            .apply(Action::Detail)
            .unwrap_err()
            .contains("authority refused")
    );
    let calls = calls.borrow();
    let q::Command::Explain(c) = calls.last().unwrap() else {
        panic!()
    };
    assert_eq!((c.contribution, c.occurrence), (0, 21));
    assert_eq!(c.snapshot_id, "frozen");
    assert_eq!(viewer.frame(None, None).snapshot_id, "frozen");
}
