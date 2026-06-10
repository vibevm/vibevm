//! Plain-text Prometheus metrics serialiser. We do not pull a
//! prometheus crate for slice 5; the surface is small enough to roll
//! by hand and keep the dep tree minimal.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::server::state::AppState;

pub fn render(state: &AppState, package_count: u64, version_count: u64) -> String {
    let uptime = state.started_at.elapsed();
    let requests = state.stats.requests_total.load(Ordering::Relaxed);
    let mutations = state.stats.mutations_total.load(Ordering::Relaxed);
    let read_only = state.read_only as u64;

    let mut s = String::with_capacity(512);
    metric(
        &mut s,
        "vibe_index_uptime_seconds",
        "Server uptime in seconds.",
        "gauge",
        uptime_seconds(uptime),
    );
    metric(
        &mut s,
        "vibe_index_packages_total",
        "Number of distinct packages held in the index.",
        "gauge",
        package_count,
    );
    metric(
        &mut s,
        "vibe_index_versions_total",
        "Number of (package, version) entries held in the index.",
        "gauge",
        version_count,
    );
    metric(
        &mut s,
        "vibe_index_requests_total",
        "Total HTTP requests served since process start.",
        "counter",
        requests,
    );
    metric(
        &mut s,
        "vibe_index_mutations_total",
        "Total mutating HTTP requests served since process start.",
        "counter",
        mutations,
    );
    metric(
        &mut s,
        "vibe_index_read_only",
        "1 if the server was started in read-only mode, else 0.",
        "gauge",
        read_only,
    );
    s
}

fn metric(out: &mut String, name: &str, help: &str, ty: &str, value: u64) {
    use std::fmt::Write;
    writeln!(out, "# HELP {name} {help}").unwrap();
    writeln!(out, "# TYPE {name} {ty}").unwrap();
    writeln!(out, "{name} {value}").unwrap();
}

fn uptime_seconds(d: Duration) -> u64 {
    d.as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_canonical_lines() {
        let state = AppState::new(
            std::path::PathBuf::from("."),
            true,
            crate::index::Index::new(
                "vibespecs",
                "https://example.invalid",
                crate::types::NamingConvention::KindName,
            ),
        );
        let s = render(&state, 0, 0);
        assert!(s.contains("# TYPE vibe_index_uptime_seconds gauge"));
        assert!(s.contains("vibe_index_packages_total 0"));
        assert!(s.contains("vibe_index_read_only 1"));
    }
}
