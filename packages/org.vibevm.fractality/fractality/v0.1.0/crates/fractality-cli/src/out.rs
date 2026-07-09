//! Plain-text rendering (D17): stable columns, grep/awk-able, title last
//! so every other column splits on whitespace.

use camino::Utf8Path;
use fractality_core::run::RunRecord;
use fractality_core::time::{format_duration_ms, now_ms};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The result pointer from the run dir's `usage.json` (Phase 3
/// collection). This is a persistence-plane read, deliberately: the pod
/// computed the pointer at collection time, the bus record stays lean
/// (D10), and run dirs are the API of last resort (D17). `None` when
/// the run has no collected result (or predates collection).
fn result_line(run_dir: &Utf8Path) -> Option<String> {
    let raw = std::fs::read_to_string(run_dir.join("usage.json").as_std_path()).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let source = v.get("result_source")?.as_str()?;
    if source == "none" {
        return None;
    }
    let path = v.get("result_path")?.as_str()?;
    Some(format!("{path} ({source})"))
}

/// Usage + cost lines shared by the summary and the detail views.
fn print_usage_lines(r: &RunRecord) {
    println!(
        "usage:      in={} out={} cache_w={} cache_r={} events={}",
        r.usage.input_tokens,
        r.usage.output_tokens,
        r.usage.cache_creation_input_tokens,
        r.usage.cache_read_input_tokens,
        r.usage.events,
    );
    if r.usage.total_cost_usd > 0.0 {
        println!("cost_usd:   {:.6}", r.usage.total_cost_usd);
    }
    if let Some(result) = result_line(&r.run_dir) {
        println!("result:     {result}");
    }
}

pub fn print_runs(runs: &[RunRecord], quiet: bool) {
    if quiet {
        for r in runs {
            println!("{}", r.run_id);
        }
        return;
    }
    println!(
        "{:<26} {:<15} {:<10} {:<7} {:<7} TITLE",
        "RUN_ID", "STATE", "PROFILE", "MODEL", "AGE"
    );
    let now = now_ms();
    for r in runs {
        println!(
            "{:<26} {:<15} {:<10} {:<7} {:<7} {}",
            r.run_id,
            r.state,
            r.profile,
            r.model,
            format_duration_ms(now.saturating_sub(r.created_ts_ms)),
            r.title,
        );
    }
}

/// The one-screen summary `fractality run` prints at the end (D13).
pub fn print_run_summary(r: &RunRecord, waited: std::time::Duration) {
    println!("state:      {}", r.state);
    println!(
        "exit_code:  {}",
        r.exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".into())
    );
    if let Some(f) = &r.failure {
        println!("failure:    {f}");
    }
    if let Some(k) = r.kill_reason {
        println!("killed:     {k}");
    }
    println!(
        "waited:     {}",
        format_duration_ms(waited.as_millis() as u64)
    );
    println!("run_dir:    {}", r.run_dir);
    println!("transcript: {}", r.run_dir.join("worker-stdout.jsonl"));
    if r.usage.events > 0 {
        print_usage_lines(r);
    }
}

pub fn print_run_detail(r: &RunRecord) {
    let now = now_ms();
    println!("run_id:     {}", r.run_id);
    println!("title:      {}", r.title);
    println!("state:      {}", r.state);
    println!("profile:    {} (model slot: {})", r.profile, r.model);
    println!("workspace:  {}", r.workspace_mode.as_str());
    println!(
        "parent:     {}",
        r.parent
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".into())
    );
    println!("node:       {}", r.node_id);
    println!("run_dir:    {}", r.run_dir);
    println!(
        "age:        {}",
        format_duration_ms(now.saturating_sub(r.created_ts_ms))
    );
    match r.pod {
        Some(pod) => println!("pod:        {} (pid {})", pod.pod_id, pod.pod_pid),
        None => println!("pod:        -"),
    }
    println!(
        "worker_pid: {}",
        r.worker_pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".into())
    );
    println!(
        "exit_code:  {}",
        r.exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".into())
    );
    if let Some(f) = &r.failure {
        println!("failure:    {f}");
    }
    if let Some(k) = r.kill_reason {
        println!("killed:     {k}");
    }
    print_usage_lines(r);
}
