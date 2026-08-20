//! The discipline half of the tool set: twelve tools, each a thin
//! adapter over the SAME lib fn the umbrella CLI calls — parity by
//! construction, pinned by the parity map in this package's brief. The
//! adapter wraps every runner in `mcp_core::capture`, so the report an
//! agent reads is the run's WHOLE story: the runner's own stderr and
//! every child process (go, gofmt, staticcheck, exhaustive) it
//! spawned.

specmark::scope!("spec://go-ai-native-mcp/tools/discipline-mcp-go#discipline-tools");

use std::path::{Path, PathBuf};

use mcp_core::{Tool, ToolDescriptor, ToolOutput};
use serde_json::{Value, json};

/// The shared language guard (PROP-026 grammar continuity): every tool
/// accepts an optional `language` and refuses a mismatch with a recipe
/// naming the right server, never another language's fix surface.
pub(crate) fn language_mismatch(args: &Value) -> Option<ToolOutput> {
    let asked = args.get("language").and_then(Value::as_str)?;
    if asked == "go" {
        return None;
    }
    Some(ToolOutput::failed(format!(
        "this server serves language `go`; asked for `{asked}` — mount that \
         language's own discipline server (mcp:org.vibevm.ai-native/{asked}-ai-native-mcp) \
         and call it there (PROP-027; PROP-026 §2)"
    )))
}

/// One string arg, defaulted.
fn str_arg<'v>(args: &'v Value, key: &str, default: &'v str) -> &'v str {
    args.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn bool_arg(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// A discipline tool: name, schema, and the runner it delegates to.
/// `run_fn` returns the CLI lib fn's own `Result`; the captured stderr
/// IS the report, and a runner error appends its Class-F chain.
struct DisciplineTool {
    name: &'static str,
    description: &'static str,
    properties: Value,
    root: PathBuf,
    run_fn: fn(&Path, &Value) -> anyhow::Result<()>,
}

impl Tool for DisciplineTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: self.name.to_string(),
            description: self.description.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": self.properties,
                "additionalProperties": false,
            }),
        }
    }

    fn run(&mut self, args: &Value) -> ToolOutput {
        if let Some(refusal) = language_mismatch(args) {
            return refusal;
        }
        let root = self.root.clone();
        let run_fn = self.run_fn;
        match mcp_core::capture(|| run_fn(&root, args)) {
            Ok((Ok(()), said)) => ToolOutput::ok(said),
            Ok((Err(e), said)) => ToolOutput::failed(format!("{said}{e:#}")),
            Err(e) => ToolOutput::failed(e.to_string()),
        }
    }
}

/// The common `language` property every tool's schema carries.
fn language_property() -> (&'static str, Value) {
    (
        "language",
        json!({
            "type": "string",
            "description": "must be `go` when given — this server serves one language",
        }),
    )
}

fn props(mut extra: serde_json::Map<String, Value>) -> Value {
    let (k, v) = language_property();
    extra.insert(k.to_string(), v);
    Value::Object(extra)
}

macro_rules! properties {
    ($($key:literal : $schema:tt),* $(,)?) => {{
        // `mut` is unused for the zero-property expansions.
        #[allow(unused_mut)]
        let mut m = serde_json::Map::new();
        $( m.insert($key.to_string(), json!($schema)); )*
        props(m)
    }};
}

/// Build the twelve discipline tools for `root`. Every entry names the
/// CLI invocation it is parity-locked to in its description — the
/// brief's parity map is generated from the same table of truth.
pub fn discipline_tools(root: &Path) -> Vec<Box<dyn Tool>> {
    let t = |name, description, properties, run_fn| -> Box<dyn Tool> {
        Box::new(DisciplineTool {
            name,
            description,
            properties,
            root: root.to_path_buf(),
            run_fn,
        })
    };
    vec![
        t(
            "init",
            "Bootstrap the discipline surface (conform.toml with the [go] table, \
             specmap.toml, registries, external specs). = `go-ai-native init`. Never \
             overwrites without `force`.",
            properties! {
                "namespace": {"type": "string", "description": "spec:// namespace; default: the root dir's name"},
                "force": {"type": "boolean", "description": "overwrite init-owned files"},
            },
            |root, args| {
                go_ai_native_cli::run_init(
                    root,
                    &go_ai_native_cli::InitOptions {
                        namespace: args
                            .get("namespace")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        force: bool_arg(args, "force"),
                    },
                )
            },
        ),
        t(
            "floor",
            "The seven-step verification floor: gofmt → vet → tests → staticcheck+\
             exhaustive → conform → specmap → test-gate, one report, one verdict. \
             = `go-ai-native floor`. Runs the module's whole toolchain — expect minutes.",
            properties! {
                "keep_going": {"type": "boolean", "description": "run every step even after a failure"},
            },
            |root, args| {
                go_ai_native_cli::run_floor(
                    root,
                    &go_ai_native_cli::FloorOptions {
                        keep_going: bool_arg(args, "keep_going"),
                        quiet: false,
                    },
                )
            },
        ),
        t(
            "conform_check",
            "The conform gate: extract facts through go-extract, run the Go rules, \
             fail on any new finding past the frozen ratchet. = `go-ai-native-conform \
             check` / `go-ai-native conform check`.",
            properties! {
                "scope": {"type": "string", "description": "limit to one root-relative path prefix"},
                "baseline": {"type": "string", "description": "ratchet file, root-relative (default go-ai-native-conform-baseline.json)"},
            },
            |root, args| {
                go_ai_native_conform::run_check(
                    root,
                    str_arg(args, "baseline", go_ai_native_conform::DEFAULT_GO_BASELINE),
                    args.get("scope").and_then(Value::as_str),
                )
            },
        ),
        t(
            "conform_freeze",
            "Rewrite the conform ratchet to the current finding set (a NEW rule \
             landing, or a re-freeze after the set shrank). = `go-ai-native-conform \
             freeze`.",
            properties! {
                "baseline": {"type": "string", "description": "ratchet file, root-relative"},
            },
            |root, args| {
                go_ai_native_conform::run_freeze(
                    root,
                    str_arg(args, "baseline", go_ai_native_conform::DEFAULT_GO_BASELINE),
                )
            },
        ),
        t(
            "specmap_check",
            "Rebuild the traceability index in memory and byte-compare against the \
             committed specmap.json; the orphan ratchet blocks. = `go-ai-native-specmap \
             --check`.",
            properties! {},
            |root, _| go_ai_native_specmap::run_specmap_go(root, true),
        ),
        t(
            "specmap_write",
            "Regenerate and write specmap.json (drift report to the log); the orphan \
             ratchet reports non-blocking. = `go-ai-native-specmap`.",
            properties! {},
            |root, _| go_ai_native_specmap::run_specmap_go(root, false),
        ),
        t(
            "trace_explain",
            "Explain one declaration or spec unit through the index: what implements, \
             verifies, documents it. = `go-ai-native trace <target>`.",
            properties! {
                "target": {"type": "string", "description": "a `<file>::<decl>` symbol or spec:// URI (required)"},
                "json": {"type": "boolean"},
                "prose": {"type": "boolean"},
            },
            |root, args| {
                let Some(target) = args.get("target").and_then(Value::as_str) else {
                    anyhow::bail!("`trace_explain` needs `target`");
                };
                go_ai_native_cli::run_trace_explain(
                    root,
                    target,
                    bool_arg(args, "json"),
                    bool_arg(args, "prose"),
                )
            },
        ),
        t(
            "test_gate",
            "The xfail-strict test gate over `go test -json` results vs the tests \
             baseline. = `go-ai-native test-gate`. Runs the whole test suite — expect \
             minutes.",
            properties! {
                "baseline": {"type": "string", "description": "root-relative baseline (default discipline/registry/tests-baseline.json)"},
            },
            |root, args| {
                go_ai_native_cli::run_test_gate(
                    root,
                    str_arg(args, "baseline", go_ai_native_cli::DEFAULT_TESTS_BASELINE),
                )
            },
        ),
        t(
            "tripwire",
            "Debt tripwires: which debt-registry entries' watched files moved. \
             = `go-ai-native tripwire`.",
            properties! {
                "base": {"type": "string", "description": "git base rev to diff against"},
                "debt": {"type": "string", "description": "root-relative debt registry"},
            },
            |root, args| {
                go_ai_native_cli::run_tripwire(
                    root,
                    args.get("base").and_then(Value::as_str),
                    str_arg(args, "debt", go_ai_native_cli::DEFAULT_DEBT_REGISTRY),
                )
            },
        ),
        t(
            "health",
            "The sweep's fact collector: one JSON snapshot of budgets, the ban census, \
             Example coverage, the orphan backlog. = `go-ai-native health`.",
            properties! {
                "out": {"type": "string", "description": "root-relative output (default discipline/health/latest-go.json)"},
            },
            |root, args| {
                go_ai_native_cli::run_health(
                    root,
                    str_arg(args, "out", go_ai_native_cli::DEFAULT_HEALTH_OUT),
                )
            },
        ),
        t(
            "fast_loop",
            "Per-cell first-signal budget: run each cell's tests in isolation. \
             = `go-ai-native fast-loop`.",
            properties! {
                "cell": {"type": "string", "description": "one cell (package dir) by name"},
                "budget_secs": {"type": "integer", "description": "per-cell budget (default 60)"},
                "enforce_budget": {"type": "boolean"},
            },
            |root, args| {
                go_ai_native_cli::run_fast_loop(
                    root,
                    args.get("cell").and_then(Value::as_str),
                    args.get("budget_secs")
                        .and_then(Value::as_u64)
                        .unwrap_or(60),
                    bool_arg(args, "enforce_budget"),
                )
            },
        ),
        t(
            "codemod_add_cell",
            "Scaffold one cell package (doc.go with //spec:scope + New + a smoke test \
             with an executed Example) with rollback on failure. = `go-ai-native \
             codemod add-cell`.",
            properties! {
                "cell": {"type": "string", "description": "cell package name, lowercase letters/digits (required)"},
                "spec_uri": {"type": "string", "description": "the spec:// unit the cell implements (required)"},
            },
            |root, args| {
                let need = |k: &str| {
                    args.get(k)
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow::anyhow!("`codemod_add_cell` needs `{k}`"))
                };
                go_ai_native_cli::run_codemod_add_cell(root, need("cell")?, need("spec_uri")?)
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_tools_with_object_schemas_and_the_language_guard() {
        let tools = discipline_tools(Path::new("."));
        assert_eq!(tools.len(), 12);
        for t in &tools {
            let d = t.descriptor();
            let schema = serde_json::to_value(&d).unwrap();
            assert_eq!(schema["inputSchema"]["type"], "object", "{}", d.name);
            assert!(
                schema["inputSchema"]["properties"]["language"].is_object(),
                "{} carries the language property",
                d.name
            );
        }
    }

    #[test]
    fn language_mismatch_refuses_with_the_recipe() {
        let out = language_mismatch(&json!({"language": "rust"})).expect("mismatch refuses");
        assert!(out.is_error);
        assert!(out.report.contains("rust-ai-native-mcp"), "{}", out.report);
        assert!(language_mismatch(&json!({"language": "go"})).is_none());
        assert!(language_mismatch(&json!({})).is_none());
    }
}
