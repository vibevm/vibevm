# Common typed parameters and secret boundaries — source-only design input

Inspected main `c20b6bc8945ad4ff971db4ba8a2e9b1aa8645424` at `/fast/git/vibevm`. Read the named `90-user.xml`, `PROP-065#parameters`, and `PROP-069#PARAMETERS-ARE-VALUES/#SECRET-BOUNDARY`. No product writes, builds, tests, program invocations, state/Git mutations or worker delegation. All findings below are **source-reachable paths and proposed synthetic counterexamples**, not executed runtime leaks. No actual secret file/value was accessed.

## Result

The existing ordinary successful `parameters` projections are already secret-aware. The failure path is not: canonicalization errors, missing-path errors and opaque prompt errors can carry a declared secret answer to both human and JSON CLI errors. A common typed error/result model can close those paths without changing authored or wire schemas.

A claim that the overall secret boundary is then complete would be false. Framework template expansion discards secrecy and feeds paths, durable env entries and agent prompts. More seriously, hosted resume construction copies raw `--set` argv independently of template expansion. A template guard must run during declaration/planning preflight and again at execution, and resume-command construction needs its own answer-aware boundary.

The root's proposed rule—secret values only in the existing ephemeral recipe/shell parameter environment and direct local parameter conditions, no secret interpolation in framework templates—is well supported by the existing “never written anywhere” secret contract. It is still an intentional tightening of currently accepted declarations; the implementation accepts secret `$param` references today. Existing tested legitimate nonsecret templates and secret child-environment delivery can be preserved.

## Existing value contract and reusable owner

Pure validation currently lives at `crates/vibe-core/src/manifest/package/application/wizard.rs:ParameterDecl::canonical`, while app input selection and host facts live at `crates/vibe-app/src/steps/params.rs:resolve_parameters`.

Preserve these exact semantics unless separately changed:

| Kind/rule | Current behavior |
| --- | --- |
| All answer kinds | Any `char::is_control` refuses before type processing. |
| String | Returns raw text, including empty text and ordinary leading/trailing spaces. `required` means a value must be supplied; it does not make an explicitly supplied empty string invalid. |
| Boolean | Trims, ASCII-lowercases; true/yes/on/1 → `true`, false/no/off/0 → `false`. |
| Integer | Trims, parses signed i64, checks inclusive optional min/max, renders canonical decimal. Overflow is invalid. |
| Choice | Exact equality with declared choice value; no trimming/case folding. |
| Path | Rejects trim-empty; otherwise pure canonicalization retains raw text. App host adapter resolves it and optionally checks existence. |
| Answer precedence | Last matching explicit `--set` wins; otherwise recorded value; otherwise attended prompt; otherwise default. Prompt `None` keeps default. Unknown explicit IDs refuse before parameter iteration. |
| Authored defaults | Untagged Bool/Integer/Text TOML values must match declared type; e.g. a string default for integer is refused even if its text could parse. |
| Definition checks | Portable unique IDs, nonblank name, min/max only on integer and min<=max, must_exist only on path, nonempty/unique choice values and labels, choices only on choice, typed/default canonical validity. |

`wizard_validation.rs:validate_parameter` owns the definition checks and uses `canonical` only after checking the authored default's variant. `validate_wizard` checks all parameter rows, not just those selected by a feature. A parameter definition is not a feature ID and should not be lowered into the feature-closure kernel.

Main already has `vibe_core::configuration` for kind-neutral feature closure. The smallest reusable parameter addition is a sibling pure parameter module there:

- validated parameter shape/constraints for the existing five kinds;
- a canonical typed answer (boolean/integer/text with the kind retained, or explicit five-kind variants), plus conversion back to existing canonical text;
- structured value problems carrying **no supplied answer bytes**, e.g. ControlCharacters, EmptyPath, InvalidBoolean, InvalidInteger, OutsideBounds { min,max }, InvalidChoice; allowed values/bounds are definition data;
- pure definition validation reused by the app declaration adapter;
- no environment, filesystem, prompt, persistence, package kind, step execution or consumer-choice authority inside this module.

Keep `ParameterDecl`, `ParameterType`, `ParameterValue` authored serde spelling and `ParameterDecl::canonical` as compatibility entrypoints/adapters. The existing app type/field location can re-export common types if moved; no wire rewrite is required. App input precedence, defaults versus recorded/prompt origin, host path resolution and secret display/persistence policy remain explicit adapters. Origin can be retained internally as Explicit/Recorded/Prompt/Default without adding an origin field to current external schemas. Do not invent an encrypted store, public digest of secret answers or a new agent runtime for this extraction.

A useful concrete split is **pure typed parsing/constraints** versus **classified answer envelope and adapters**. The canonical value parser should not need a secret flag to avoid embedding the raw value in errors; app presentation may render a richer public-input error only after consulting its declaration. That prevents Debug/Display on a shared problem value from becoming a new leak route.

## Exact direct counterexamples and sinks

Use only synthetic canaries not present in the public declaration/default/choice list.

1. Declared secret bool `token`, supplied text `SYN_SECRET_INVALID_BOOL`. `ParameterDecl::canonical` currently returns `` `SYN_SECRET_INVALID_BOOL` is not a boolean... ``. Integer parse and choice mismatch similarly interpolate `raw`.
2. Declared secret integer with `max=8`, answer `00042`. Range failure prints canonical `42`, even though it is not byte-equal to raw `00042`. Redacting only a raw substring would miss it.
3. Declared secret path `must_exist=true`, answer `/synthetic-missing/SYN_SECRET_PATH`. `resolve_parameters` calls host resolution then embeds `path.display()` in “does not exist”. No need to open/read a real secret file to test this boundary.
4. A secret parameter prompt callback returns `Err("transport failed after SYN_SECRET_PROMPT")`. `resolve_parameters` copies the opaque callback string into “the prompt failed: ...”. The library seam permits this, regardless of whether today's Dialoguer errors usually echo answers.

Paths are:

`wizard.rs:canonical` → `steps/params.rs:resolve_parameters` → `StepError::ParameterInvalid { option, parameter, reason }` (`vibe-app/src/error.rs:286`) → propagated AppError/anyhow.

`vibe-cli/src/output/lines.rs:Context::error_with_suffix` emits `format!("{err:#}")` directly in human-quiet mode and in JSON `error` (around line 151); human mode uses `refusal_of`, which starts from the same formatted chain (around line 270). `output/structured.rs` adds `code`, step/work and parked metadata but does not redact the string. Stable `parameter_invalid` classification does not make the message safe.

Minimal error fix should replace secret canonical/path/prompt diagnostics with reason classes, parameter/option IDs and safe remediation. Do not retain the opaque original prompt error as a source chain that a caller's alternate anyhow formatting can expose. Public nonsecret diagnostics may preserve useful input/path detail through the app adapter. Direct calls to `ParameterDecl::canonical` on a secret declaration must be safe too, rather than relying only on CLI formatting.

An additional pre-declaration surface exists: `vibe-cli/src/cli/app.rs:parameter_value` returns `` `{text}` is not ID=VALUE `` for malformed `--set`, and Clap may also print the rejected argument. This is not classified secret input yet; a narrow declared-parameter fix should not claim it redacts arbitrary malformed CLI argv. If included, inspect actual Clap output with synthetic malformed input; changing only the callback's message may not suppress Clap's own invalid-value rendering.

## Existing successes that must stay intact

- `steps/plan_view.rs:Plan::to_wire` omits `value` for a secret parameter and keeps `id`/`secret`.
- `vibe-cli/src/commands/app/plan_text.rs` prints `(secret)` instead of the answer.
- `operations/apply.rs` filters secret answers from `Installation.parameters`.
- `steps/runs.rs:RunRecord::open` writes `plan.to_wire()`, so the ordinary `plan.json.parameters` view inherits that omission.
- `describe.rs` exposes **authored** parameter type/default/choices/labels/bounds/secret marking. These are public definition data, not the user's answer. Do not automatically erase them because the parameter is secret.
- `steps/run.rs:Run::new` builds an ephemeral `VIBE_PARAM_<ID>` vector containing all answers. `steps/kinds.rs:environment` is actually called by Recipe and Shell branches and supplies that vector to those children. Root's proposed rule preserves this existing route; it must not be generalized to every child.
- Direct `Condition::Param` and `ParamEquals` use canonical answers locally. Their displayed condition strings contain authored IDs/comparison literals, not the selected answer. A declared comparison literal remains public definition data even if it happens to equal an answer.

`ParamValue` and `Plan` currently derive Debug, so formatting a classified Plan prints secret values. I found no production plan-debug output sink in the inspected path; this is a dangerous public value surface, not an executed-log claim. A custom redacting Debug for ParamValue is a bounded improvement and automatically fixes Plan's recursive Debug. Raw InstallRequest/CLI argument structs lack declaration context and also derive Debug; no actual production printing was established here, so do not extend the claim to arbitrary debug formatting of raw input containers.

The attended prompt match checks Bool/Choice **before** `_ if parameter.secret`, so secret bool/choice currently use visible Confirm/menu widgets. Put the secret branch first; hidden text must still pass the same bool/integer/choice canonical validator. Public labels, choices and declared defaults remain description material. Preserve prompt-None/default and existing selection precedence; no solver or UI default may manufacture consent.

## Framework templates and output/persistence sinks

`vibe-core::manifest::expand_step_template` is a one-pass grammar with a callback for `StepVar`. It does not recursively expand a returned parameter string. `vibe-app/src/steps/template.rs:expand` currently returns `param.value.clone()` with no secret check or provenance. The guard should inspect parsed `StepVar::Param(id)`, not substring-match `$param`, and should not reparse returned answer text. Escaped `$$param:token` is literal, not a secret reference.

| Template field | Current expansion phase | Source-reachable framework sink |
| --- | --- | --- |
| `when/unless.file_present` | Planning, when that condition is reached | Expanded host filesystem path is queried. Skip reason retains the authored condition spelling. The secret-path use is still interpolation and should be covered if root adopts one consistent template rule. |
| `Copy.to` | Runtime in `steps/kinds.rs` | An absolute expanded destination outside payload is rendered in `StepError::Copy`; IO failures also carry expanded target paths. `Run::settle` writes error text into StepsFile, `phase` calls `task.fail(error.to_string())`, then CLI emits it. A valid relative secret destination writes the answer into filesystem names even without failure. |
| `Env.set` values and `Env.path_prepend` | Runtime `steps/env.rs:env_step` | Expanded values enter durable user environment operations, `EnvEntry`, `state.env`, steps.json and installation lifecycle env. These are framework effects, not child-authored output. |
| inline `Agent.prompt` | Consent preview and runtime | `AgentNotice::Prompt` sends expanded prompt to CLI `agents.rs:show`, which writes each line to stderr even in ordinary JSON mode unless machine-progress suppresses notices. `Envelope::write` writes prompt.md; `keep` copies it into run records; hosted `park` embeds it in outbox task text. |
| contents of package-relative `Agent.prompt_file` | File read + same prompt expansion at preview/runtime | Same sinks as inline prompt. Manifest validation checks only the prompt_file path, not its file contents. |

Minimal concrete examples:

- `to = "$param:token"`, secret answer `/synthetic/SYN_COPY_SECRET`, with a real harmless `from` fixture. The code path reaches an outside-payload error containing the expanded answer; preceding steps may already have run.
- An env step `set = { DEMO = "$param:token" }`. Even if every report message is redacted, the framework still persists the expanded secret value in the environment and its recovery/removal record. Output redaction alone is insufficient.
- An agent prompt `"Use $param:token"`, or a prompt_file with that content. The framework itself displays/writes it before any child can echo anything.

These examples are unexecuted, but the source chain is direct. They do not rely on arbitrary child behavior.

### A guard only in runtime expansion is too late

`steps/plan.rs:plan` currently resolves parameter values, then evaluates conditions/needs and creates steps. It does not pre-expand or policy-check Copy.to, Env fields or prompt-file contents. `operations/mod.rs:install_entered` obtains this plan, installs required applications, and then applies the selected app. `Run::open` creates/clears work state and run records; earlier steps can complete before a later template refuses. Env steps are forced into finish phase by `steps_of`, making a runtime-only refusal especially late.

`agent::admit` calls `prompt_text(...).or_else(|_| raw_prompt(...))`: **every** expansion error is masked as a raw preview. This fallback is legitimate only for a preview depending on a later `$app_root`/`$app_bin`, not for secret-policy refusal or unrelated malformed input.

The bounded fix needs two levels:

1. **Grammar-aware declaration/planning preflight before dependency/application execution.** Extend the existing pure declaration/template-name check to know secret parameter IDs and reject inline secret references in all relevant fields, including dormant/otherwise skipped declarations if the rule is a definition law. In app planning, read selected package-relative prompt_file text and apply the same grammar-aware policy before returning/showing a successful plan. This does not need actual answer values or unavailable `$app_root` paths: for policy inspection, named runtime variables can be accepted as placeholders while parsed secret parameter references refuse.
2. **Runtime guard at `template::expand`.** This catches programmatically assembled declarations and mutable prompt-file changes. Secret policy refusal must remain typed/distinguishable from “runtime variable not available yet”; `agent::admit` may fall back only for the specifically deferred preview case. Do not suppress a secret-policy error with `.or_else(|_| raw_prompt)`.

A preflight read of prompt_file does not freeze an arbitrary mutable package forever. Retaining the exact validated prompt text for execution would strengthen that continuity; otherwise runtime revalidation catches later drift but may do so after earlier effects. State the guarantee honestly and do not add a whole-file/world transaction implicitly.

“Before any effect” also needs a precise scope: the existing `install()` acquires the settings lock before planning, and CLI package resolution occurs earlier. The directly available guarantee is before dependency/app steps, downloads invoked by them, work publication and environment changes. A claim of literally zero filesystem or resolution activity would need examination/reordering beyond this parameter slice.

## Fields and paths outside central template expansion

Do not treat every authored string as a framework template:

- Shell command/update/force/uninstall text is passed to the shell command owner; recipe script is an authored package-relative path; native arguments pass through native installer handling. Recipe and Shell execution branches call the parameter-environment adapter; the Native branch does not call it in the inspected code. Recipe/shell children can print/write supplied values. That is the declared arbitrary-child exception, not a reason to remove their existing environment route or pretend the framework sandboxes them.
- Copy.from, source URLs/checksums, recipe path, prompt_file **path**, step titles and descriptions are authored data in the inspected code, not answer expansion sites. The prompt_file **contents** are an expansion site.
- Direct Param/ParamEquals predicates are local evaluation, distinct from template FilePresent paths. Keep this distinction in the rule.
- The `$param` callback itself emits only “parameter ID has no value” on absence and parser errors concern authored template syntax; it does not normally echo already expanded accumulated text. The dangerous values arise downstream after successful expansion.

## Independent mandatory bypass: hosted resume argv

This path survives a perfect template guard:

`vibe-cli/src/commands/app/agents.rs:AgentSetup::new` captures `std::env::args().skip(1)` → `resume_command` removes only --from-step and --resume → retains raw `--set token=SYN_RESUME_CANARY` → `AgentHooks.resume_command` → hosted branch in `steps/agent.rs:deliver` → `envelope::park` writes it in the outbox task → `ParkedStep.resume` → human `report_parked` and JSON `output/structured.rs:stamp_app_error`.

A secret answer need not be referenced by an agent prompt at all. This is framework-generated command replay, not child output. `agents.rs` even has a test whose expected resume string preserves a nonsecret `--set 'a=b c'`; that public-value behavior is a compatibility control.

The fix should redact/omit classified secret assignments **while argv is still structured**, including both `--set ID=VALUE` and `--set=ID=VALUE` spellings. Do not search/replace secret substrings in a shell-quoted resume string: canonical values can differ from raw argv, and quoting is another grammar. The API currently receives only a formatted resume string, whereas the selected option/plan has secrecy metadata. Root must choose where those two inputs meet; this is an adapter seam, not a reason to put CLI parsing into the common value kernel.

Blindly dropping every `--set` is safe from that sink but loses valid public reconfiguration intent, which may not yet exist in an installation record. If that tradeoff is chosen, report it explicitly. A secret cannot be reconstructed from a persisted record under the contract; resume must require fresh ephemeral input/prompt and explain that without persisting plaintext or a reusable secret digest.

## Is refusing secret templates a clarification or a regression?

The strong existing rule in PROP-065 says secret values are passed to children and not written anywhere; PROP-069 explicitly limits secret answers to ephemeral authorized execution inputs and omits them from framework descriptions/plans/progress/persisted configuration. The source's current acceptance of secret-to-env-record or secret-to-agent-envelope is inconsistent with that rule. Refusing those declarations is therefore a justified clarification/security correction of a broader grammar, not removal of a contractually promised durable secret store.

However, it remains a behavior change for packages relying on the current acceptance. The existing generic `$param:ID` wording does not state an exception for secret values, so the permanent contract should explicitly state the restriction and safe migration path. A package can continue using `VIBE_PARAM_<ID>` in the existing recipe/shell child path; an agent step does not receive that resolved parameter vector, and a secret cannot be supplied via its persisted prompt under the proposed policy. The latter may reduce what an agent step can do, and should not be advertised as unchanged universal secret support.

Source tests found for current behavior use nonsecret `$param:lang` in a copy destination and a secret token through child environment. `cli_app_wizard.rs` deliberately has the child write its environment to a fixture payload and checks that the ordinary installation parameter map omits the token. Preserve that test distinction: child-generated secret output is outside the framework guarantee. I found no test/source fixture explicitly requiring secret `$param` interpolation as supported semantics in the inspected application surfaces.

## Suggested bounded verification for the future implementation

No tests were run here. Useful existing targets/cases:

- `vibe-core --lib` document wizard cases: `parameter_values_are_canonical_and_typed`, `wizard_rules_are_enforced_with_named_reasons`, plus the new pure configuration parameter family. Preserve authored serde roundtrips and boolean/integer/choice/path behavior.
- `vibe-app --lib` parameter family: precedence, unknown/required/type/path errors, fake prompt errors, canonical range values, public versus secret diagnostics, redacting Debug. Use unique synthetic answer canaries not found in definitions.
- `vibe-cli --test cli_app_wizard`: existing real describe→plan→install→record journey; add focused local/offline fixtures for invalid secret bool/int/choice/path through JSON and human errors, and successful secret omission. A shared binary fixture should avoid duplicating the existing server-heavy journey unnecessarily.
- Planner policy tests: inline copy/env/agent/condition fields; prompt-file contents; escaped `$$param` and a nonsecret answer that itself contains `$param:token` (one-pass expansion must not recursively treat it as a reference); skipped/dormant definitions according to the chosen law; legitimate deferred `$app_root` preview remains distinct from denied secret reference.
- No-effect adapter tests: a late secret template refuses before an earlier step's sentinel side effect/dependency installer, and no prompt envelope/env record is written. Account separately for existing lock/resolution setup rather than calling the entire process zero-effect.
- Hosted resume test with a secret --set that is **unused by the prompt**; inspect outbox, JSON/human resume, run records and public arguments. Public --set values remain reproducible; secret resupply requirement is explicit.
- Exact schema checks remain the same: application describe parameters expose authored defaults/choices, application plan parameters retain string value only when nonsecret, installation index `parameters` remains map<string,string> excluding secrets. No schema epoch bump is required by the pure extraction or omission/error correction.

## Scope recommendation

A coherent first common-parameter slice can include the pure typed value/constraint owner and app compatibility adapter, classified error/prompt/Debug fixes, declaration/planning template refusal plus runtime defense, and secret-aware hosted resume construction. Those boundaries are directly connected to the stated secret law and actual consumers. Keep arbitrary child output handling, encrypted persistence, OS process-environment secrecy, generic taint tracking and a new agent execution service outside the promise. If root chooses a smaller error-only atom first, explicitly retain template/resume debt and do not mark the common secret boundary complete.

## Explicit agent-environment boundary (root scope correction)

`steps/run.rs` around lines 514–519 builds the ordinary step environment, then deliberately handles Agent differently: `requirements = self.paths.child_env(&[], ...)`. Its comment says the agent receives the requirements PATH and nothing more of the step environment under `PROP-064#envelope`. `AgentStep` carries that requirements slice, and the spawned agent command receives it; it does **not** receive `self.env`/resolved `VIBE_PARAM_*` answers. This is an existing architectural boundary, not an accidental omission to fill during common-parameter work.

The earlier generic wording “passed to children” in `ParamValue`/`ParameterDecl` comments and the broad step-environment description can suggest more than the actual call graph. `kinds.rs:environment` is invoked by Recipe and Shell, while Native does not use that parameter-vector adapter and Agent explicitly excludes it. Reports and any clarified contract should name the real supported recipients rather than claim every child receives resolved parameters. This says nothing about arbitrary pre-existing ambient environment inherited by a spawned process; that is a different source from this invocation's typed answers.

Therefore the secret-template refusal must **not** be compensated by adding secret parameter environment to agents. Agent prompt/envelope has no approved secret transport after the correction; designing one would require separate authorization/contract. Keep the legitimate recipe/shell environment route, existing agent requirements environment, and ordinary public prompt templates intact. Hosted resume sanitization remains necessary even when the agent never consumes the secret, because it currently copies raw invocation argv into framework output.
