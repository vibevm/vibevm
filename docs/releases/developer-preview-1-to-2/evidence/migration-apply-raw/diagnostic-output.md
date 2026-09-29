# Generic executed diagnostic output

Writer stopped. Exact owners: tools/output.rs487lines; lib.rs540lines; new tests/diagnostic_output.rs97lines. No tool registry, context, handler, workspace, schema or Cargo/dependency changes.

ToolOutput has a private structured_present bool. Existing ok/executed/executed_failure constructors settrue; new executed_failure_text(text) setsfalse, mandatory text/isErrortrue and internal Value::Null placeholder. has_structured()->bool exposes rendering mode. Existing structured()->&Value and into_parts()->(Value,String,bool) signatures stay compatible; their internal null is not a absence test. Generic dispatcher records has_structured before consuming the triple and inserts structuredContent only when true. No tool-name/domain branches exist.

Affected source citations: ToolOutput implements P015#tools req4, new constructor implements EXECUTED-DIAGNOSTIC-FAILURE req1, Server implements #server req3; new testscope/verifies bind diagnostic req1. Module documentation describes diagnostic-only executed failure and unchanged explicit null. Existing preflight channels remain unchanged. PROP044 SURFACES req2 governs how the domain later supplies labelled unvalidated evidence; this generic atom does not serialize or validate a domain report.

Two selected exact integration cases bound before compilation, --locked --offline -p vibe-mcp --test diagnostic_output, jobs4/sharedtarget, -- --exact:

- executed_diagnostic_failure_omits_structured_content
- explicit_null_structured_outputs_retain_the_field

Both EXIT0, each1passed/0failed/1filtered. An unrelated neutral testtool dispatches through real Server and actual MemoryTransport (test-only Arc/Mutex wrapper permits output observation without a product transport accessor). Diagnostic result has exact mandatory text/isErrortrue and key absence. Null case separately checks all3existing constructors and structuredContent:null key presence, with respective success/failure flags and text. Testcontext uses absenttemp project/cache and proves no entries created. No actual migration/provider handler ran.

Raw logs: /tmp/vibevm-next-M13-D1-diagnostic-output-text-tests.log and -null-tests.log. Existing safefs warnings retained, no diagnostic-owner failure. Targeted skip_children rustfmt3owners EXIT0; a documentation grammar correction between exact runs changed no behavior. No other70tests, check/codegen/broad gates, Git/boot/steward/public or user-path operations. Root retains domain transport integration and acceptance.
