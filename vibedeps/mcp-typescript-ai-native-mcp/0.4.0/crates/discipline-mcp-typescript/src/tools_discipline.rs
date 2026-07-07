//! The discipline half of the tool set: twelve tools (the TS umbrella
//! has no ledger), each a thin adapter over the SAME lib fn the
//! `discipline-typescript` / `conform-typescript` /
//! `specmap-typescript` CLIs call — parity by construction, pinned by
//! the parity map in this package's brief. The adapter wraps every
//! runner in `mcp_core::capture`, so the report an agent reads is the
//! run's WHOLE story: the runner's own stderr and every child process
//! (node, tsc, prettier, eslint) it spawned. Absent-toolchain refusals
//! (the hard-fail-with-recipe posture) arrive as `isError` results
//! carrying the recipe.

specmark::scope!(
    "spec://typescript-ai-native-mcp/tools/discipline-mcp-typescript#discipline-tools"
);

use std::path::{Path, PathBuf};

use mcp_core::{Tool, ToolDescriptor, ToolOutput};
use serde_json::{Value, json};

/// The shared language guard (PROP-026 grammar continuity): every tool
/// accepts an optional `language` and refuses a mismatch with a recipe
/// naming the right server, never another language's fix surface.
pub(crate) fn language_mismatch(args: &Value) -> Option<ToolOutput> {
    let asked = args.get("language").and_then(Value::as_str)?;
    if asked == "typescript" {
        return None;
    }
    Some(ToolOutput::failed(format!(
        "this server serves language `typescript`; asked for `{asked}` — mount \
         that language's own discipline server (mcp:org.vibevm/{asked}-ai-native-mcp) \
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
            "description": "must be `typescript` when given — this server serves one language",
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

/// Build the twelve discipline tools for `root`.
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
            "Bootstrap the discipline surface (conform.toml, specmap.toml, registries, \
             external specs). = `discipline-typescript init`. Never overwrites without \
             `force`.",
            properties! {
                "namespace": {"type": "string", "description": "spec:// namespace; default: the root dir's name"},
                "force": {"type": "boolean", "description": "overwrite init-owned files"},
            },
            |root, args| {
                discipline_cli_typescript::run_init(
                    root,
                    &discipline_cli_typescript::InitOptions {
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
            "The seven-step verification floor: prettier → tsc → tests → eslint → \
             conform → specmap → test-gate, one report, one verdict. = \
             `discipline-typescript floor`. Runs the project's whole toolchain — \
             expect minutes; absent tools hard-fail with the install recipe.",
            properties! {
                "keep_going": {"type": "boolean", "description": "run every step even after a failure"},
            },
            |root, args| {
                discipline_cli_typescript::run_floor(
                    root,
                    &discipline_cli_typescript::FloorOptions {
                        keep_going: bool_arg(args, "keep_going"),
                        quiet: false,
                    },
                )
            },
        ),
        t(
            "conform_check",
            "The ts-tsc conform gate: extract facts through the project's own \
             typescript, run the structural rules, fail on any new finding past the \
             frozen ratchet. = `conform-typescript check`.",
            properties! {
                "scope": {"type": "string", "description": "limit to one root by name"},
                "baseline": {"type": "string", "description": "ratchet file, root-relative (default conform-typescript-baseline.json)"},
            },
            |root, args| {
                conform_cli_typescript::run_check(
                    root,
                    str_arg(
                        args,
                        "baseline",
                        conform_cli_typescript::DEFAULT_TS_BASELINE,
                    ),
                    args.get("scope").and_then(Value::as_str),
                )
            },
        ),
        t(
            "conform_freeze",
            "Rewrite the TS conform ratchet to the current finding set. = \
             `conform-typescript freeze`.",
            properties! {
                "baseline": {"type": "string", "description": "ratchet file, root-relative"},
            },
            |root, args| {
                conform_cli_typescript::run_freeze(
                    root,
                    str_arg(
                        args,
                        "baseline",
                        conform_cli_typescript::DEFAULT_TS_BASELINE,
                    ),
                )
            },
        ),
        t(
            "specmap_check",
            "Rebuild the traceability index through the ts-tsc scanner and byte-compare \
             against the committed specmap.json; the orphan gate blocks. = \
             `specmap-typescript --check`.",
            properties! {},
            |root, _| specmap_cli_typescript::run_specmap_typescript(root, true),
        ),
        t(
            "specmap_write",
            "Regenerate and write specmap.json; the orphan gate reports non-blocking. \
             = `specmap-typescript`.",
            properties! {},
            |root, _| specmap_cli_typescript::run_specmap_typescript(root, false),
        ),
        t(
            "trace_explain",
            "Explain one symbol or spec unit through the index. = \
             `discipline-typescript trace <target>`.",
            properties! {
                "target": {"type": "string", "description": "a symbol or spec:// URI (required)"},
                "json": {"type": "boolean"},
                "prose": {"type": "boolean"},
            },
            |root, args| {
                let Some(target) = args.get("target").and_then(Value::as_str) else {
                    anyhow::bail!("`trace_explain` needs `target`");
                };
                discipline_cli_typescript::run_trace_explain(
                    root,
                    target,
                    bool_arg(args, "json"),
                    bool_arg(args, "prose"),
                )
            },
        ),
        t(
            "test_gate",
            "The xfail-strict test gate over node's TAP output vs the tests baseline. \
             = `discipline-typescript test-gate`. Runs the whole suite — expect \
             minutes.",
            properties! {
                "baseline": {"type": "string", "description": "root-relative baseline (default discipline/registry/tests-baseline.json)"},
            },
            |root, args| {
                discipline_cli_typescript::run_test_gate(
                    root,
                    str_arg(
                        args,
                        "baseline",
                        discipline_cli_typescript::DEFAULT_TESTS_BASELINE,
                    ),
                )
            },
        ),
        t(
            "tripwire",
            "Debt tripwires: which debt-registry entries' watched files moved. = \
             `discipline-typescript tripwire`.",
            properties! {
                "base": {"type": "string", "description": "git base rev to diff against"},
                "debt": {"type": "string", "description": "root-relative debt registry"},
            },
            |root, args| {
                discipline_cli_typescript::run_tripwire(
                    root,
                    args.get("base").and_then(Value::as_str),
                    str_arg(
                        args,
                        "debt",
                        discipline_cli_typescript::DEFAULT_DEBT_REGISTRY,
                    ),
                )
            },
        ),
        t(
            "health",
            "The sweep's fact collector: one JSON snapshot. = `discipline-typescript \
             health`.",
            properties! {
                "out": {"type": "string", "description": "root-relative output (default discipline/health/latest-typescript.json)"},
            },
            |root, args| {
                discipline_cli_typescript::run_health(
                    root,
                    str_arg(args, "out", discipline_cli_typescript::DEFAULT_HEALTH_OUT),
                )
            },
        ),
        t(
            "fast_loop",
            "Per-cell first-signal budget: typecheck+test each cell in isolation. = \
             `discipline-typescript fast-loop`. Expensive.",
            properties! {
                "cell": {"type": "string", "description": "one cell by name"},
                "budget_secs": {"type": "integer", "description": "per-cell budget (default 60)"},
                "enforce_budget": {"type": "boolean"},
            },
            |root, args| {
                discipline_cli_typescript::run_fast_loop(
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
            "Scaffold one cell (dir + index seam + spec marker; the cells dir and seam \
             come from conform.toml's [typescript] table) with rollback on failure. \
             = `discipline-typescript codemod add-cell <cell> <spec-uri>`.",
            properties! {
                "cell": {"type": "string", "description": "cell name (required)"},
                "spec_uri": {"type": "string", "description": "the spec:// unit the cell implements (required)"},
            },
            |root, args| {
                let need = |k: &str| {
                    args.get(k)
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow::anyhow!("`codemod_add_cell` needs `{k}`"))
                };
                discipline_cli_typescript::run_codemod_add_cell(
                    root,
                    need("cell")?,
                    need("spec_uri")?,
                )
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
        assert!(language_mismatch(&json!({"language": "typescript"})).is_none());
        assert!(language_mismatch(&json!({})).is_none());
    }
}
