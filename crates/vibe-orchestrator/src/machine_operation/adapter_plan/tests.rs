use super::*;
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("vibe.toml"),
        r#"[package]
group = "org.example"
name = "fixture"
kind = "tool"
version = "1.0.0"
authors = ["Fixture"]
license = "MIT"
description = "Fixture"
keywords = ["fixture"]
[[skill]]
name = "demo"
path = "skills/demo"
agents = ["claude"]
"#,
    )
    .unwrap();
    std::fs::create_dir_all(root.path().join("skills/demo/references")).unwrap();
    std::fs::write(root.path().join("skills/demo/SKILL.md"), "body").unwrap();
    std::fs::write(root.path().join("skills/demo/references/a.txt"), "resource").unwrap();
    root
}
fn host(root: &Path) -> AdapterPlanHost {
    AdapterPlanHost::new(
        root,
        AdapterTarget::Skills {
            scope: AdapterScope::Project,
            user_home: None,
        },
    )
    .unwrap()
}
fn plan(route: &AdapterPlanRoute) -> q::Request {
    q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        target_id: route.target_id().into(),
        command: q::Command::Plan(Box::new(q::CommandPlan {
            capture_id: route.capture_id().map(str::to_owned),
        })),
    }
}
#[test]
fn prepared_plan_is_nonempty_deterministic_and_source_drift_refuses() {
    let root = fixture();
    let host = host(root.path());
    let prepared = host.prepare(true).unwrap();
    let route = AdapterPlanRoute::new(host, Some(prepared)).unwrap();
    let first = route.request(plan(&route)).unwrap();
    let second = route.request(plan(&route)).unwrap();
    assert_eq!(first, second);
    let r::Body::Plan(body) = first.body else {
        panic!()
    };
    assert_eq!(body.rows.len(), 1);
    let r::Row::Skill(skill) = &body.rows[0] else {
        panic!()
    };
    assert_eq!(skill.files.len(), 2);
    assert!(!root.path().join(".vibe").exists());
    std::fs::write(root.path().join("skills/demo/references/a.txt"), "drift").unwrap();
    let error = route.request(plan(&route)).unwrap_err();
    assert_eq!(error.reason, Some(e::ErrorReason::SourceCaptureStale));
}
#[test]
fn unprepared_and_foreign_bindings_have_closed_refusals() {
    let root = fixture();
    let route = AdapterPlanRoute::new(host(root.path()), None).unwrap();
    assert_eq!(
        route.request(plan(&route)).unwrap_err().reason,
        Some(e::ErrorReason::SourceCaptureMissing)
    );
    let mut request = plan(&route);
    request.context_id = "foreign".into();
    assert_eq!(
        route.request(request).unwrap_err().code,
        e::ErrorCode::WrongContext
    );
    let mut request = plan(&route);
    request.target_id = "foreign".into();
    assert_eq!(
        route.request(request).unwrap_err().code,
        e::ErrorCode::WrongTarget
    );
    assert!(!root.path().join(".vibe").exists());
}
#[test]
fn unknown_capture_and_unknown_fields_refuse_without_refresh() {
    let root = fixture();
    let host = host(root.path());
    let prepared = host.prepare(true).unwrap();
    let route = AdapterPlanRoute::new(host, Some(prepared)).unwrap();
    let mut request = plan(&route);
    request.command = q::Command::Plan(Box::new(q::CommandPlan {
        capture_id: Some("foreign".into()),
    }));
    assert_eq!(
        route.request(request).unwrap_err().code,
        e::ErrorCode::UnknownCapture
    );
    let reply = route.exchange(br#"{"unknown":true}"#);
    assert!(matches!(reply.body, env::Body::Error(_)));
}

#[cfg(unix)]
#[test]
fn prepared_deploy_binds_client_probes_and_refuses_executable_drift() {
    let (_root, owned, route, trace) = shared_fixture::deploy_route(true);
    let settings = owned.path().join("settings");
    let executable = owned.path().join("client");
    let before = std::fs::read_to_string(&trace).unwrap();
    assert_eq!(before.lines().count(), 2);
    let first = route.request(plan(&route)).unwrap();
    assert_eq!(route.request(plan(&route)).unwrap(), first);
    let r::Body::Plan(body) = first.body else {
        panic!()
    };
    assert_eq!(body.rows.len(), 1);
    assert!(matches!(&body.rows[0], r::Row::Deploy(_)));
    assert_eq!(std::fs::read_to_string(&trace).unwrap(), before);
    assert!(!settings.exists());
    std::fs::write(&executable, "changed").unwrap();
    assert_eq!(
        route.request(plan(&route)).unwrap_err().reason,
        Some(e::ErrorReason::ClientCaptureStale)
    );
    assert_eq!(std::fs::read_to_string(&trace).unwrap(), before);
}

#[test]
fn both_scopes_use_host_injected_home_without_creating_destinations() {
    let root = fixture();
    let owned = tempfile::tempdir().unwrap();
    let home = owned.path().join("absent-home");
    let host = AdapterPlanHost::new(
        root.path(),
        AdapterTarget::Skills {
            scope: AdapterScope::Both,
            user_home: Some(home.clone()),
        },
    )
    .unwrap();
    let capture = host.prepare(true).unwrap();
    let route = AdapterPlanRoute::new(host, Some(capture)).unwrap();
    let answer = route.request(plan(&route)).unwrap();
    let r::Body::Plan(body) = answer.body else {
        panic!()
    };
    assert_eq!(body.rows.len(), 2);
    let scopes: Vec<_> = body
        .rows
        .iter()
        .map(|row| {
            let r::Row::Skill(skill) = row else { panic!() };
            skill.scope.clone()
        })
        .collect();
    assert_eq!(scopes, [r::RowSkillScope::Project, r::RowSkillScope::User]);
    assert!(!home.exists());
    assert!(!root.path().join(".claude").exists());
}

use super as adapter_owner;
#[cfg(unix)]
#[path = "test_fixture.rs"]
mod shared_fixture;
#[cfg(unix)]
#[test]
fn unqualified_deploy_retains_history_and_returns_typed_unavailable() {
    let (_root, _owned, route, trace) = shared_fixture::deploy_route(false);
    let before = std::fs::read(&trace).unwrap();
    assert_eq!(
        before
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .count(),
        2
    );
    for _ in 0..2 {
        let error = route.request(plan(&route)).unwrap_err();
        assert_eq!(error.reason, Some(e::ErrorReason::ClientStateUnavailable));
    }
    assert!(route.capture_id().is_some());
    assert_eq!(std::fs::read(&trace).unwrap(), before);
}

#[path = "m_08_b_2_fixture.rs"]
mod m_08_b_2_fixture;

#[test]
fn m_08_b_2_cases_selected_self_and_dependency_bridge_roots() {
    m_08_b_2_fixture::isolated(
        "m_08_b_2_cases_selected_self_and_dependency_bridge_roots",
        |owned| {
            let fixture = m_08_b_2_fixture::Fixture::new(owned);
            fixture.positive();
        },
    );
}
#[test]
fn m_08_b_2_cases_missing_changed_and_membership_drift_refuse_without_refresh() {
    m_08_b_2_fixture::isolated(
        "m_08_b_2_cases_missing_changed_and_membership_drift_refuse_without_refresh",
        |owned| {
            for kind in [
                "missing",
                "changed",
                "member",
                "composed",
                "lock",
                "manifest",
                "absent-cache",
            ] {
                let fixture = m_08_b_2_fixture::Fixture::new(owned);
                fixture.positive();
                fixture.tamper(kind);
                for _ in 0..2 {
                    let error = fixture.query(kind).unwrap_err();
                    assert_eq!(error.code, e::ErrorCode::NeedsPreparation);
                    assert_eq!(error.reason, Some(e::ErrorReason::SourceCaptureStale));
                }
            }
        },
    );
}
#[test]
fn m_08_b_2_cases_decoy_roots_cannot_override_selected_context() {
    m_08_b_2_fixture::isolated(
        "m_08_b_2_cases_decoy_roots_cannot_override_selected_context",
        |owned| {
            let fixture = m_08_b_2_fixture::Fixture::new(owned);
            let before = fixture.positive();
            fixture.decoys();
            assert_eq!(fixture.positive(), before);
            assert!(
                fixture
                    .route
                    .compare_context_root(&std::env::current_dir().unwrap())
                    .is_err()
            );
        },
    );
}
