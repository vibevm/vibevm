//! Plain-text rendering (D17): stable columns, grep/awk-able, title last
//! so every other column splits on whitespace.

use camino::Utf8Path;
use fractality_core::api::TreeNode;
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
    if r.usage.web_tool_calls > 0 {
        println!("web_tools:  {} (D12 quota counter)", r.usage.web_tool_calls);
    }
    if r.usage.total_cost_usd > 0.0 {
        println!("cost_usd:   {:.6}", r.usage.total_cost_usd);
    }
    // Phase 4: collection rides the bus and folds into the record — the
    // record is authoritative; the plane files remain the fallback for
    // runs that predate the Collected event.
    if let Some(c) = &r.collected {
        if c.result_source != "none" {
            let path = c
                .result_path
                .as_ref()
                .map(|p| p.as_str())
                .unwrap_or("<collected>");
            println!("result:     {path} ({})", c.result_source);
        }
        if let Some(fref) = &c.result {
            println!(
                "ref:        {}:{}{}",
                fref.fs,
                fref.path,
                fref.etag
                    .as_deref()
                    .map(|e| format!(" etag={e}"))
                    .unwrap_or_default()
            );
        }
        if let Some(reason) = &c.acceptance_skipped {
            println!("acceptance: skipped ({reason})");
        } else if c.acceptance_total > 0 {
            println!(
                "acceptance: {}/{} ok",
                c.acceptance_passed, c.acceptance_total
            );
        }
        return;
    }
    if let Some(result) = result_line(&r.run_dir) {
        println!("result:     {result}");
    }
    print_acceptance(&r.run_dir);
}

/// The scoreboard (D16): one totals block, then per-profile and
/// per-model tables — stable columns, greppable, cost explicit.
pub fn print_metrics(m: &fractality_core::api::MetricsResponse) {
    println!(
        "runs:       {} total = {} completed + {} failed + {} killed + {} open",
        m.totals.runs, m.totals.completed, m.totals.failed, m.totals.killed, m.totals.open
    );
    println!(
        "tokens:     in={} out={} cache_w={} cache_r={}",
        m.totals.input_tokens,
        m.totals.output_tokens,
        m.totals.cache_creation_input_tokens,
        m.totals.cache_read_input_tokens
    );
    println!("cost_usd:   {:.6}", m.totals.total_cost_usd);
    println!(
        "wall:       {} across terminal runs",
        format_duration_ms(m.totals.wall_ms)
    );
    if m.totals.web_tool_calls > 0 {
        println!(
            "web_tools:  {} (D12 quota counter; per-day split below)",
            m.totals.web_tool_calls
        );
    }
    for (title, map) in [("profile", &m.by_profile), ("model", &m.by_model)] {
        if map.is_empty() {
            continue;
        }
        println!(
            "{:<10} {:>5} {:>5} {:>5} {:>5} {:>5} {:>12} {:>10}  # by {title}",
            "NAME", "RUNS", "OK", "FAIL", "KILL", "OPEN", "OUT_TOKENS", "COST_USD"
        );
        for (name, b) in map {
            println!(
                "{:<10} {:>5} {:>5} {:>5} {:>5} {:>5} {:>12} {:>10.4}",
                name,
                b.runs,
                b.completed,
                b.failed,
                b.killed,
                b.open,
                b.output_tokens,
                b.total_cost_usd
            );
        }
    }
    if !m.by_day.is_empty() {
        println!(
            "{:<12} {:>5} {:>12} {:>10} {:>10}  # by day (UTC)",
            "DAY", "RUNS", "OUT_TOKENS", "COST_USD", "WEB_TOOLS"
        );
        for (day, b) in &m.by_day {
            println!(
                "{:<12} {:>5} {:>12} {:>10.4} {:>10}",
                day, b.runs, b.output_tokens, b.total_cost_usd, b.web_tool_calls
            );
        }
    }
}

/// One call tree, ASCII-rendered: two spaces per depth, id first so the
/// output stays xargs-able even indented (D17).
pub fn print_tree(node: &TreeNode, indent: usize) {
    let r = &node.run;
    println!(
        "{:indent$}{} {} {} {}",
        "",
        r.run_id,
        r.state,
        r.profile,
        r.title,
        indent = indent * 2
    );
    for child in &node.children {
        print_tree(child, indent + 1);
    }
}

/// Acceptance verdicts from the run dir's `status.json` — the same
/// deliberate persistence-plane read as [`result_line`]; the bus
/// promotion is Phase 4 work.
fn print_acceptance(run_dir: &Utf8Path) {
    let Ok(raw) = std::fs::read_to_string(run_dir.join("status.json").as_std_path()) else {
        return;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    if let Some(reason) = v.get("acceptance_skipped").and_then(|s| s.as_str()) {
        println!("acceptance: skipped ({reason})");
        return;
    }
    let Some(items) = v.get("acceptance").and_then(|a| a.as_array()) else {
        return;
    };
    if items.is_empty() {
        return;
    }
    let ok = items
        .iter()
        .filter(|i| i.get("ok").and_then(|b| b.as_bool()).unwrap_or(false))
        .count();
    println!("acceptance: {ok}/{} ok", items.len());
    for i in items {
        let command = i.get("command").and_then(|c| c.as_str()).unwrap_or("?");
        let passed = i.get("ok").and_then(|b| b.as_bool()).unwrap_or(false);
        let code = i
            .get("exit_code")
            .and_then(|c| c.as_i64())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".into());
        println!(
            "  {} exit={code} {command}",
            if passed { "ok  " } else { "FAIL" }
        );
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
    if let Some(q) = &r.question {
        println!("question:   {q}");
    }
    print_usage_lines(r);
}
