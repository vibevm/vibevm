//! The one comparison a run makes before it renders anything.

use chrono::TimeZone;
use vibe_wire::generated::doc_site_state::RenderedVersion;

use super::*;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 12, 12, 0, 0).unwrap()
}

fn pair(name: &str, version: &str, hash: &str) -> Pair {
    Pair {
        source: "vibespecs".into(),
        group: "org.example".into(),
        name: name.into(),
        version: version.into(),
        content_hash: hash.into(),
        origin: Origin::Registry,
        entry: None,
    }
}

fn host_pair(name: &str, hash: &str) -> Pair {
    Pair {
        source: "https://example.invalid/host".into(),
        origin: Origin::HostPackage {
            dir: std::path::PathBuf::from("/checkout"),
        },
        ..pair(name, "1.0.0", hash)
    }
}

fn row(name: &str, version: &str, hash: &str, failed: bool) -> RenderedVersion {
    RenderedVersion {
        source: "vibespecs".into(),
        group: vibe_core::Group::parse("org.example").expect("a group"),
        name: name.into(),
        version: version.parse().expect("a version"),
        content_hash: hash.into(),
        rendered_at: Utc.with_ymd_and_hms(2026, 9, 12, 8, 0, 0).unwrap(),
        files: 12,
        failed,
    }
}

#[test]
fn a_pair_nothing_has_rendered_is_new() {
    let queue = plan(
        &[pair("a", "1.0.0", "sha256:aa")],
        &state::empty(),
        now(),
        60,
    );
    assert_eq!(queue.rebuild.len(), 1);
    assert_eq!(queue.rebuild[0].verdict, Verdict::New);
    assert_eq!(queue.unchanged, 0);
}

#[test]
fn a_pair_rendered_from_these_very_bytes_is_not_rebuilt() {
    let mut state = state::empty();
    state.rendered.push(row("a", "1.0.0", "sha256:aa", false));
    let queue = plan(&[pair("a", "1.0.0", "sha256:aa")], &state, now(), 60);
    assert!(queue.is_empty());
    assert_eq!(queue.unchanged, 1);
}

/// The republication case, and the whole reason the key is a hash and not
/// a version: one number, new bytes, one address to rebuild.
#[test]
fn a_version_republished_with_other_bytes_has_moved() {
    let mut state = state::empty();
    state.rendered.push(row("a", "1.0.0", "sha256:aa", false));
    let queue = plan(&[pair("a", "1.0.0", "sha256:bb")], &state, now(), 60);
    assert_eq!(queue.rebuild[0].verdict, Verdict::Moved);
}

/// A package whose source never moves again would never be retried if a
/// failure were recorded as an absence.
#[test]
fn a_render_that_failed_is_retried_although_nothing_moved() {
    let mut state = state::empty();
    state.rendered.push(row("a", "1.0.0", "sha256:aa", true));
    let queue = plan(&[pair("a", "1.0.0", "sha256:aa")], &state, now(), 60);
    assert_eq!(queue.rebuild[0].verdict, Verdict::Failed);
    assert_eq!(queue.unchanged, 0);
}

/// The registry keeps no history, so a version that left the catalog is
/// only still visible in the render — and the run has to say so.
#[test]
fn an_address_no_source_publishes_any_more_is_reported_as_gone() {
    let mut state = state::empty();
    state.rendered.push(row("a", "1.0.0", "sha256:aa", false));
    let queue = plan(&[], &state, now(), 60);
    assert_eq!(queue.gone, vec!["org.example/a@1.0.0".to_string()]);
    assert!(!queue.is_empty());
}

/// A burst of commits must cost one render, not one per commit — which is
/// why the debounce is measured on the source and not on the pair.
#[test]
fn the_host_is_held_as_one_source_while_the_debounce_runs() {
    let mut state = state::empty();
    state.host_rendered_at = Some(Utc.with_ymd_and_hms(2026, 9, 12, 11, 30, 0).unwrap());
    let queue = plan(
        &[
            host_pair("a", "sha256:aa"),
            host_pair("b", "sha256:bb"),
            pair("published", "1.0.0", "sha256:cc"),
        ],
        &state,
        now(),
        60,
    );
    let held = queue.held.expect("the host held back");
    assert_eq!(held.pairs, 2);
    assert_eq!(held.minutes_left, 31);
    // The registry is never held: its feed moves only when somebody
    // publishes, and a publication is the event a site exists to show.
    assert_eq!(queue.rebuild.len(), 1);
    assert_eq!(queue.rebuild[0].pair.name, "published");
}

#[test]
fn the_host_goes_through_once_the_debounce_has_run_out() {
    let mut state = state::empty();
    state.host_rendered_at = Some(Utc.with_ymd_and_hms(2026, 9, 12, 10, 59, 0).unwrap());
    let queue = plan(&[host_pair("a", "sha256:aa")], &state, now(), 60);
    assert!(queue.held.is_none());
    assert_eq!(queue.rebuild.len(), 1);
}

/// Holding pairs back is not the same as having none, and a report that
/// said «nothing to do» while four renders waited would look broken.
#[test]
fn a_hold_with_nothing_behind_it_is_not_reported() {
    let mut state = state::empty();
    state.host_rendered_at = Some(Utc.with_ymd_and_hms(2026, 9, 12, 11, 30, 0).unwrap());
    let queue = plan(&[pair("a", "1.0.0", "sha256:aa")], &state, now(), 60);
    assert!(queue.held.is_none());
}

/// Two sources under one coordinate is reported and never resolved by
/// picking: a site that chose silently would serve one registry's package
/// under another's name.
#[test]
fn one_coordinate_from_two_sources_with_different_bytes_is_a_collision() {
    let other = Pair {
        source: "mirror".into(),
        content_hash: "sha256:bb".into(),
        ..pair("a", "1.0.0", "sha256:aa")
    };
    let queue = plan(
        &[pair("a", "1.0.0", "sha256:aa"), other],
        &state::empty(),
        now(),
        60,
    );
    assert_eq!(queue.collisions.len(), 1);
    assert!(queue.collisions[0].contains("org.example/a@1.0.0"));
    // One address is rendered once, from the source that answered first.
    assert_eq!(queue.rebuild.len(), 1);
}

/// The same bytes under one coordinate from two mirrors is not a
/// collision — it is the same package, twice.
#[test]
fn one_coordinate_from_two_sources_with_the_same_bytes_is_not_a_collision() {
    let mirror = Pair {
        source: "mirror".into(),
        ..pair("a", "1.0.0", "sha256:aa")
    };
    let queue = plan(
        &[pair("a", "1.0.0", "sha256:aa"), mirror],
        &state::empty(),
        now(),
        60,
    );
    assert!(queue.collisions.is_empty());
    assert_eq!(queue.rebuild.len(), 1);
}

#[test]
fn the_report_names_the_verdict_the_source_and_the_totals() {
    let mut state = state::empty();
    state
        .rendered
        .push(row("kept", "1.0.0", "sha256:kk", false));
    let report = plan(
        &[
            pair("kept", "1.0.0", "sha256:kk"),
            pair("fresh", "2.0.0", "sha256:ff"),
        ],
        &state,
        now(),
        60,
    )
    .render();
    assert!(
        report.contains("new    org.example/fresh@2.0.0"),
        "{report}"
    );
    assert!(report.contains("vibespecs"), "{report}");
    assert!(
        report.contains("queue: 1 to render, 1 unchanged, 0 gone"),
        "{report}"
    );
}
