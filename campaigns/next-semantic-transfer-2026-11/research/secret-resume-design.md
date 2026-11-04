# Secret-safe hosted resume seam — bounded source-only design

Target: `/fast/git/vibevm` main `c20b6bc8945ad4ff971db4ba8a2e9b1aa8645424`, continuing the preceding quiet audit. No product writes, builds, runtime probes, state/Git edits or new workers. All scenarios are static counterexamples, not executed failures. This report proposes an internal Rust hook/context change; root owns design and implementation.

## Recommended smallest single-root design

Add an optional **typed resume formatter** to `AgentHooks`, retain the existing literal hook as a separately constrained compatibility path, and bind the root plan's answer classification into the copied operation context **before entering requirements**. The formatter owns the original CLI argv as a vector and receives only classification metadata, never answer values from the library.

Concrete internal/public Rust shapes (names proposed, no serde or new wire document):

```rust
pub struct ResumeParameter {
    pub id: String,
    pub secret: bool,
}

pub struct ResumeScope {
    pub package: String,
    pub option: String,
    // Present resolved answers only, classified without their values.
    pub parameters: Vec<ResumeParameter>,
}

pub struct ResumeRequest<'a> {
    pub root: &'a ResumeScope,
    pub current: &'a ResumeScope,
}

pub enum ResumeUnavailable {
    UnclassifiedArgument,
    IncompleteRootSet,
    UnsupportedInvocation,
}

pub type ResumeFormatter<'a> =
    &'a dyn Fn(&ResumeRequest<'_>) -> Result<String, ResumeUnavailable>;
```

Extend `AgentHooks<'a>` with:

```rust
pub resume_formatter: Option<ResumeFormatter<'a>>,
pub resume_root: Option<&'a ResumeScope>,
// Existing field retained:
pub resume_command: Option<&'a str>,
```

`AgentHooks` remains `Clone + Copy + Default`. OperationContext already owns AgentHooks by value, so it needs **no additional direct field**, and dependency contexts already copy it. `ResumeScope` is owned temporary metadata in the operation stack; it must not include raw or canonical values, reusable secret hashes, or references to the complete ParamValue objects. Existing Plan.parameters is enough to construct it: a valid explicit --set answer necessarily appears there because unknown IDs/refused values prevent a successful plan. Omitted optional parameters are not represented as having an answer. Public defaults are not argv and need no fabricated --set.

This is a real operational hook, not `on_plan`, `notify`, progress, output mode, an observer side effect, a global RefCell or a callback-dependent custody protocol. Quiet/JSON/no-plan-rendering operations get the same classification.

## Exact binding points and data flow

### Global/single-app install

`operations/mod.rs:install_entered` currently:

1. loads index/storage, chooses option and admits operation;
2. obtains `plan_option(...)`;
3. calls `deps::install_required(ctx, &plan, ...)`;
4. calls `apply::apply(ctx, ...)`.

Between 2 and 3, construct `let classification = ResumeScope::from_plan(&plan)` and a scoped OperationContext whose AgentHooks uses:

- the inherited `resume_root` if it is already Some;
- otherwise a borrow of this local classification.

Pass that scoped context to both dependency installation and apply. Keep the local classification alive until they return. A small private helper can reborrow the Copy context at the local lifetime. Do not overwrite an inherited root with the currently visited dependency plan.

### Update

`operations/update.rs:update_locked` chooses the actual option using either explicit choice or the previous installation's option, constructs Previous and then calls `plan_option`. Bind the root classification **after that actual new plan** and before `deps::install_required`; use the scoped context for apply. Classifying all candidate options before this point would mislabel answers when options disagree on secrecy.

### Modify

`operations/modify.rs:modify` intentionally calls `plan_option` with a `quiet` context (`on_plan=None`), decorates warnings and later optionally shows the plan. Bind the classification after the actual plan independent of whether it is shown, and before dependency installation/old-choice removal/new apply. Inherit an existing root scope if supplied. This is the concrete reason an `on_plan`-based secret map is unsound.

### Recursive requirements

`operations/deps.rs:install_required` creates a dependency InstallRequest from its selected requirement option, with default parameter requests, then constructs:

```rust
let nested = OperationContext { progress: &progress, ..*ctx };
```

Once the caller has bound `ctx.agent.resume_root`, this existing copy carries the root classification unchanged through all recursion depths. Each recursive install still makes its own current plan. When it reaches an agent step, `steps/agent.rs:deliver` constructs a separate current ResumeScope from `cx.plan` and calls the formatter with `{ root: inherited_root_or_current, current }`.

The **root** scope classifies argv because the raw invocation's --set belongs to the root operation. The **current** scope identifies the actually parked package and secret values that may need re-entry there. Do not flatten them by unqualified parameter ID.

Concrete counterexamples the distinction must cover:

- Root `api-token` is secret; dependency declares no parameter with that ID. Dependency parks first. Looking only at dependency params would preserve the root's raw secret --set.
- Root `mode` is public; dependency has a secret `mode` obtained from a prompt/default. Raw --set mode=public-value belongs to the root and must be preserved; unioning dependency secret IDs into the argv filter would wrongly discard valid public intent.
- Root and dependency both have secret values with the same ID. Any resupply guidance must identify package/option scope; a single root --set cannot inject a dependency's missing answer under the current API.

No parameter environment is added to agents. `Run::step` continues to pass only `self.paths.child_env(&[], ...)` as requirements environment to the agent path; recipe/shell parameter environment is unaffected.

## CLI formatter ownership and exact argv algorithm

`vibe-cli/src/commands/app/agents.rs:AgentSetup` currently builds a formatted resume string in `new` from `std::env::args`. Replace that eager string with an owned closure that captures the argv vector and a small invocation-scope classification supplied by the CLI call site. `hooks()` lends the closure as `resume_formatter`; it should stop setting the unsafe eager literal for the CLI path.

An owned `Box<dyn Fn(&ResumeRequest<'_>) -> Result<String, ResumeUnavailable>>` is sufficient if storing a closure is simpler than a separate formatter struct. It captures only argv plus operation kind/scope. No function needs to parse a shell-quoted command back into tokens.

For a supported, completely classified single-root invocation:

1. Iterate original argv in order. Preserve ordinary arguments byte-for-byte in memory.
2. Remove --from-step and its following operand, --from-step=..., and an existing --resume as today.
3. Recognize both paired `--set`, `ID=VALUE` and attached `--set=ID=VALUE`. Split the assignment at the **first** `=` exactly as the CLI parser does, trim the ID as that parser does, and never inspect/normalize the value for secrecy.
4. Classify the ID using **root scope**. Secret: omit the entire argument occurrence. Public: retain the original token(s) exactly. All duplicate occurrences of a secret ID must be omitted, even those overridden by the last --set; all duplicate public occurrences retain their original order and current last-wins behavior.
5. An unclassified/malformed set in a purported successful single-root command is a typed unavailable result, not an assumption that the value is public. Do not include the rejected raw token in the error.
6. Add --resume only for a CLI form that actually accepts it.
7. Apply the existing shell quoting function only after filtering. Values containing spaces, quotes, `=`, shell metacharacters or strings equal to another parameter ID remain literal values; substring replacement is forbidden.

No normal display/JSON/progress callback participates. The resulting string is the only command passed to Envelope::park and ParkedStep.resume. A formatter failure should become safe unavailable guidance or a fixed typed error; never catch it and fall back to the raw literal. The closed ResumeUnavailable enum intentionally avoids an arbitrary String error that could re-echo argv.

Root/current classification can also generate **safe outbox prose** listing scoped secret IDs whose values were present and must be resupplied. This is metadata/presence, not a new external schema. Keep ParkedStep.resume a command when one is available; do not insert executable dummy secret values such as `--set token=<REENTER>` because a string parameter would happily accept and use that placeholder. If using the current existing prose fallback when no automatic command is available, label it as unavailable/manual reconstruction rather than pretending it is executable.

## CLI route inventory and completeness boundaries

| Entry | Current construction | Required policy |
| --- | --- | --- |
| `global::install` | One app required by argument check; AgentSetup before library option selection; same hooks used for plan-only and mutation | Formatter may capture argv early. Actual root classification comes from library plan before requirements. Plan-only never parks. |
| `global::update_one` | Receives resolved app, builds request and AgentSetup; actual option can come from prior record | Same late plan binding. For ordinary explicit single-app update, preserve nonsecret raw --set. |
| `lifecycle::update_all` | Calls update_one for several packages while original argv still says `--all` | Treat as batch, not one fully classified root. See below. |
| `project::install/update -> materialise -> install_pending` | The same `Materialise.request` (including --set) is applied to every pending root app. One AgentSetup reused across loop; nested requirements get default requests | Each app operation needs a fresh root binding, not the previous loop's root. Original argv can affect later app roots; incomplete cohort classification cannot safely authorize replay. |
| `portable::import -> install_row` | One AgentSetup for the import; each row builds request.parameters from the imported row, not --set argv | No ordinary --set input in AppImportArgs. Still bind a fresh root per row and current scope for nested requirements. Import grammar currently lacks --resume. |
| `lifecycle::modify` | Structured ModifyRequest has params; one AgentSetup before library chooses new/prior option; plan observer is deliberately suppressed then invoked later | Late root binding is mandatory. Modify grammar currently lacks --resume and library modify constructs Start::Fresh. |

### Shared argv across batch roots is a separate completeness issue

A project command can apply one `--set shared=SYN_CANARY` to root A, where shared is public, and root B, where shared is secret. If A's dependency parks before B is planned, A-only metadata is insufficient to prove that emitting the raw assignment respects the whole invocation's secret marking. Unioning *current dependency* IDs is not a solution: it confuses scopes and still does not know future roots.

Smallest sound bounded policy: mark AgentSetup's invocation as `SingleRoot` or `BatchRoots` using parsed call-site knowledge, not raw string heuristics. If a batch invocation carries --set and its complete selected root classification is not available, return `IncompleteRootSet` and explain that automatic replay is unavailable. **Do not silently drop all public --set**, and do not claim preservation of a complete replay that was never classified.

For fully automatic batch replay, the real selected-option classifications of **all recipients of those shared arguments** must be supplied before any recipient executes/parks. Secret wins only across that actual shared-argv recipient cohort, not across arbitrary dependency scopes or all dormant installer options. Current CLI/library flow does not preplan/retain that cohort. Adding a full preflight-and-consume batch plan is more work than the minimal callback/root-context fix and should be chosen explicitly; independently re-selecting options twice is not equivalent when prompts or state can change.

This is an honest unavailable disposition, not preservation of all batch functionality. If root requires the complete public replay journey for batch now, the callback can take `roots: &[ResumeScope]` plus an explicit completeness marker instead of one root, but the missing production owner that collects/consumes the cohort still must be implemented. Do not invent completeness from the first plan shown.

## Existing literal library hook compatibility

`AgentHooks.resume_command: Option<&str>` is public Rust API and currently accepts an opaque caller-formatted string. The framework cannot recover a trustworthy argv/ID mapping from it, and cannot safely sanitize it by comparing against canonical answer text.

Recommended bounded compatibility rule:

- Typed formatter present: use it; never fall back to the literal after a formatter refusal.
- No formatter and no secret values in root/current classified context: preserve the supplied literal exactly as today, including normal nonsecret library integrations.
- No formatter but secret values present: do not persist an unclassified literal. Use a fixed manual-resume instruction/refusal and name only scoped secret IDs that need new input. A caller that has its own safe formatter can adopt the typed hook.

This tightens the old literal path for secret-bearing calls even when an individual literal happens to be harmless. State that compatibility cost; do not falsely call an opaque string structurally safe. Alternatively root could explicitly define literals as caller-attested already-safe data, but then the guarantee is weaker and library callers—not the framework—own that branch. The CLI must use the typed formatter in either design.

Adding optional fields to a public struct preserves callers using `..AgentHooks::default()` but requires updating exhaustive struct literals. In-tree search found the CLI's exhaustive literal and a test literal using defaults; external Rust source compatibility is not absolute. There is no new serialized schema or hook protocol epoch merely from these fields.

## Remaining resume-state limits (do not hide them)

### Parameter equality is not currently a resume precondition

`StepsFile` stores package, option, features and reached steps. `steps/resume.rs:start_point` compares those three identity fields and step statuses; it does **not** persist or compare parameters. Secret answers are absent from installation records, and the run plan wire intentionally records only their IDs/secret=true without values.

Dropping a secret from a generated command means required secrets usually cause a fresh prompt/refusal; **optional** secrets may silently become absent/default on the next plan. The current state cannot determine whether that equals the original invocation. Even a newly supplied value cannot be checked for equality to the prior secret without a separately designed proof; adding a reusable digest would violate the current secret boundary's intent.

Therefore this formatter fix establishes safe serialization, not complete semantic replay for optional secrets. Safe task prose must require resupply of present secret IDs. Strong enforcement needs an exact, value-free presence/input-requirement record bound to the resumed operation, or another explicit continuation contract; that is additional state design. Do not opportunistically read the “latest” cached plan as authority. Also note that prompted nonsecret answers not in argv are not necessarily reproducible for a first uncommitted install; preserving existing nonsecret --set is narrower than capturing every wizard answer.

### A parked dependency currently does not inherit Start::Resume

`deps::install_required` creates its dependency request with `..InstallRequest::default()`, hence Start::Fresh. Root replay with --resume does not propagate that mode. A dependency that parked before it acquired an index record is installed again; Run::open on Fresh clears its old work directory, potentially deleting the answer just written for that parked step.

The typed formatter/context change does not fix this lifecycle defect. A truthful automatic continuation for a parked dependency needs either targeted continuation of that dependency or a deliberate dependency resume policy bound to existing state. Do not make a formatter claim that a sanitized root command now necessarily resumes the child successfully.

### Modify/import command grammar is not resumable as currently printed

AppCommand::Modify has AppChoiceArgs and keep_work, not AppStepArgs; AppImportArgs likewise has no resume flag. The old builder blindly appends --resume to both. Library modify reconstructs a default Fresh InstallRequest and performs old-choice removal; import_row also defaults to Fresh. A callback should return UnsupportedInvocation for automatic replay of those forms unless a separately designed canonical continuation command is available. Hiding this by dropping the unknown --resume flag would restart behavior, not resume it.

Project/bulk-update replay may also revisit already completed roots; the old whole-command string is not a proven operation continuation. These limitations predate secrecy extraction and should be retained as explicit debt or fixed deliberately, not folded into new parameter kernel authority.

## Exact candidate implementation perimeter

For the minimal safe serialization seam, without claiming complete resume-state repair:

1. `vibe-app/src/agents/hooks.rs` plus agents/lib re-exports: ResumeParameter/Scope/Request, closed formatter refusal, two optional AgentHooks fields; legacy hook policy documentation.
2. `vibe-app/src/operations/mod.rs`, `operations/update.rs`, `operations/modify.rs`: construct safe root classification from the successful actual plan and reborrow a scoped context before dependencies/apply. Add one helper to avoid three differing inheritance rules.
3. `operations/deps.rs`: ensure the existing context copy retains the inherited root; tests for nested classification. No new response observer.
4. `steps/agent.rs:deliver`: create current classification, invoke formatter once before writing task/resume output, choose safe typed-unavailable/literal disposition. Pass only sanitized result to Envelope::park/ParkedStep; add safe resupply prose at the outbox owner if needed.
5. `vibe-cli/src/commands/app/agents.rs`: capture argv structurally, filter ID assignments, quote after filtering, avoid raw value error text; hooks lend this formatter.
6. `commands/app/global.rs`, `project.rs`, `portable.rs`, `lifecycle.rs`: pass exact invocation-kind/scope to AgentSetup; single-root versus batch, and unsupported modify/import replay are explicit. No agent parameter-environment widening.

No wire schema change, generic taint engine, encrypted store, on_plan dependency, command-string reparsing, secret digest or new external agent service is needed for that bounded seam.

## Focused acceptance evidence to request when implementation is authorized

- Formatter unit tests: paired/attached --set, repeated secret occurrences including overridden ones, public last-wins order, values with `=`/spaces/quotes, from-step removal, one resume flag, unknown ID refusal without raw echo.
- Real operation test with quiet/on_plan=None: root secret omitted when a dependency parks; root public ID survives a same-named secret dependency parameter. Include at least two dependency levels.
- Single-app update/modify actual selected-option classification: previous and new options disagree on secrecy; only chosen option governs root argv.
- Batch with future recipient secret and current recipient public: no raw replay until classification complete; unavailable is explicit, not silent loss of public --set.
- Literal hook no-secret compatibility and secret-bearing safe refusal; typed formatter failure never falls back to unsafe literal.
- Inspect generated outbox and JSON/human ParkedStep.resume using unique synthetic canaries, with no secret references in agent prompt. This isolates argv leakage from template leakage.
- Test required/optional secret omission separately and report current continuation limits; do not assert successful semantic replay merely because the string is redacted.

No runtime acceptance is claimed by this report.
