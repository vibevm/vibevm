# Why ordinary Redbook installation does not yet ask for its variants

**The pinned prototype cannot take an ordinary attended `vibe install` from an unanswered Redbook variant to a completed install without another caller preparing the choice host.** It has useful choice evaluation, terminal controls and apply code, and it can install already-authored explicit choices through the ordinary CLI. The missing part is the local install composition that obtains the candidate definitions, asks the questions, retains the answers as current consumer intent and continues the existing install operation.

This is a static causal trace, not a runtime reproduction or acceptance decision. Prototype: `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`; main comparison: `1448ed5cc33a4fadbce9d8331bf5f2d96052aec3`. All reads use these Git objects. Readable copies of the 23 inspected files are under [/tmp/nst-choice-source](/tmp/nst-choice-source/PINS.txt). No tests, builds, services, network requests or DB operations were run.

## The actual paths

### Ordinary `vibe install`, no prepared-host flags

`commands/install/direct.rs:137` checks `args.choice.choice_host`. When absent, it follows the ordinary `run_with_scope` → preparation/lease/trace → `execute_prepared` path. That path builds the normal package source and calls the install planner; it does not call the choice UI or construct a `ConfigurationChoiceHost`.

The install planner builds the complete workspace root/declaration union, reads current manifest `consumer_configuration`, and captures the authored request through the normal source transaction before checking freshness (`vibe-install/src/plan.rs:360`, `authored_configuration.rs:11`, `plan/transaction.rs:10`). Thus prepared host plumbing is **not necessary for a manifest that already contains a complete explicit configuration**. The G2 production CLI test runs ordinary `--json --offline install --assume-yes --path … --registry …`, without any choice-host flag, and its MUP/WAL cases assert successful installed locks, retained manifest selections and boot output (`vibe-cli/tests/m_05_g_2_cases.rs:34,59`). That is the strongest contrary evidence to a blanket “configured install is unavailable” claim.

It does not supply the missing answers:

- Redbook's `continuity` group is `exactly-one`, members `mup`/`wal`, displayed default `mup`. `additions` and the guarded `mup-additions`/`wal-additions` groups select optional dependencies. These are real package declarations (`redbook/v1.1.0/vibe.toml:93` onward).
- With a declared consumer configuration but omitted `continuity`, the configured solver returns NeedsSelection. The real CLI G2 test asserts command failure, a diagnostic containing `continuity`, unchanged manifest and no `vibe.lock`/dependency tree (`m_05_g_2_cases.rs:45`). There is no ordinary-install handler that converts that outcome into terminal questions and retries the same prepared request.
- With no consumer/domain configuration for a package carrying checked choices, the legacy source route refuses with `package-owned choices require current consumer/domain input` (`vibe-package-source/src/cells.rs:307`). `--features` on the legacy request does not constitute the missing consumer/domain choice contract. The Redbook F1 source/planner tests explicitly prove the unbound refusal (`vibe-install/src/m_05_f_1_cases.rs:165` onward).
- `--assume-yes` is application confirmation. It is not consent to choose `mup`, nor permission to invent a domain/instance or default receipt.

So an ordinary user with an unanswered Redbook dependency sees a refusal/selection diagnostic, not a variant wizard. An ordinary user whose manifest already contains the exact selected variant can install it through the real path.

### `vibe install --choice-host … --choice-capture …`

The actual install flag is **`--choice-capture`**, not `--capture`. Clap makes `choice_host` and `choice_capture` mutually required. `--choice-request` requires `--shown-definition`, which requires the host; `--allow-defaults` also requires that shown identity. `--apply-choices` opts into applying rather than only returning the prepared graph (`cli/pkg.rs:298`). `--capture` belongs to the discovery-host interface, not ordinary install choice preparation.

With the host flag, `direct::run` returns through `choices::run` immediately. This bypasses the ordinary direct-install composition and depends on the host's retained workspace/source and application policy. The client rejects package arguments, `--git`, global mode and feature overrides because they could contradict the host's frozen root/request (`choices.rs:236`).

The reachable sequence is:

1. Connect to the supplied Unix-domain socket and send `Questions { capture }` (`choices.rs:211`). The non-Unix implementation returns “prepared choice endpoint unavailable on this platform”.
2. Check host root against `--path` and its offline posture against the effective caller/user-config posture. An offline caller refuses an online or unproved host before evaluation/preparation/apply (`choices.rs:269`).
3. In attended mode—non-JSON, non-unattended, terminal stdin and stderr—print the shown definition/defaults and call the shared `Evaluate`. The UI consumes the returned eligible members/cardinality; it does not recompute guards. It asks one radio/checkbox question, adds an explicit answer with `cli:explicit-tty-answer`, and reevaluates before the next question (`choices.rs:261,311` onward).
4. `Prepare` checks shown identity, unchanged consumer/non-choice skeleton, explicit default authorization and the evaluator outcome; it binds the normalized source, plans reconfiguration and retains the pending plan (`configuration_choice.rs:306`).
5. Only when `--apply-choices` is present does the client ask “Apply this prepared configuration?” or accept `--assume-yes` for this final confirmation. `Apply` requires host application policy, the retained matching plan and a workspace lease, then delegates to existing install apply (`choices.rs:434`; `configuration_choice.rs:447`). Without `--apply-choices`, even an attended successful answer sequence only prepares a graph.

Unattended/non-TTY operation never prompts. Without an explicit shown-definition identity the client returns the questions. With that identity and explicit answers or `--allow-defaults`, it can prepare; otherwise it retains NeedsSelection. This is useful machine behavior, not an ordinary human bootstrap.

### Where that host comes from

`ConfigurationChoiceHost::new` requires a `PreparedConfigurationDiscovery`, a normalized consumer request with admitted candidate definitions, a live `InstallSource`, the install/root request and format. Application additionally requires `.with_application(integrity, hooks)` (`configuration_choice.rs:226,286`). Serialized answers cannot construct these source/definition objects.

The only `ConfigurationChoiceHost::new` call found by a pinned-tree search is in `configuration_choice_fixture.rs:98`, the shared test fixture. That fixture manually builds the real source, candidate/normalized definitions, captured manifest resource, admission and application policy. The CLI test itself binds a Unix listener and serves the host. `serve_cli` explicitly accepts an already supplied connection and performs **no listener, process or implicit preparation** (`configuration_choice.rs:478`). There is no production `vibe install` host bootstrap in the inspected pinned tree.

`vibe show features --discover --prepare` is not that bootstrap. It captures the named cached manifests but calls `PreparedConfigurationDiscovery::admit(capture, vec![])`; ordinary discovery explicitly reports unavailable normalized authority instead of fabricating receipts (`commands/show/features.rs:31`). A supplied discovery host can project an existing admission, but cannot authorize configuration application. Likewise, the standard MCP context starts with `configuration_choice: None`; an embedding caller must use `with_configuration_choices`, and the tool refuses when it is missing (`vibe-mcp/src/context.rs:43,409`; `tools/configuration_choice.rs:95`).

## What the positive tests establish—and what they leave open

The E4 CLI test `production_install_machine_and_tty` exercises the real parsed install command/dispatch and a PTY, not merely a serializer. It uses a real prepared source host, answers a terminal selection, requests application and checks the stored lock request, including explicit empty `extras` and no fabricated default receipt (`choices.rs:23–199`). The service tests check shown-default consent, real planned graph/apply, unknown/conflicting answers, no writes on missing selection, stale shown identity, preserved dormant choice/provenance and guard-sensitive controls (`configuration_choice.rs:39` onward). These are substantive working parts.

However, the test parent supplies the host and capture through environment variables, constructs the socket and owns the serving loop. Its source fixture starts with a simple `[requires.packages]` manifest **without authored `consumer_configuration`**. The test checks choices persisted in the lock; it does not prove an ordinary fresh process can discover the variant, author its durable input, install it, then reproduce/reconfigure it without the external host.

A further necessary join is visible statically: `Prepare` changes the normalized in-memory selection, but `plan_reconfiguration` rereads the current manifest/workspace and invokes the ordinary planner. `authored_configuration.rs:62` refuses when the source-bound request differs from the current authored manifest. Therefore a host prepared over an existing incomplete/different authored row cannot simply submit changed answers and expect that row to update: it reaches this mismatch unless the legitimate owner has also staged the matching current authored change. The choice host contains no such authored-intent staging operation. This is an inference from the calls and equality guard, not an executed failure case.

Similarly, host application preserves the original request/default receipts in the lock but does not itself synthesize `consumer_configuration` into a manifest. The apply path merges package roots; it does not copy arbitrary host choice answers into authored rows (`apply.rs:385–460`). A later ordinary install on a configured lock with no current authored configuration or retained normalized source explicitly refuses exact replay (`plan.rs:296`). The current test does not prove that later ordinary invocation. Simply wiring the radio buttons into install would leave this persistence/replay seam unresolved.

The governing E4 source text itself requires real normal exposure and states that “unwired opt-in surfaces” do not establish completed production exposure (`PROP-003-dep-evolution.xml:948–953`). Its requirements explain why the existing host fixture is useful evidence but insufficient for the ordinary user journey.

## Necessary domain guarantees versus avoidable manual setup

The following checks have direct semantic purposes and must survive local integration:

- Bind questions to the exact candidate source/content/definition and original consumer/domain/instance. Otherwise the user can answer one version's question and install a different interpretation.
- Let the existing evaluator own cardinality, guards, conflicts and resulting effects; recompute after answers. Preserve omitted, explicit empty, exact/additive and dormant selections separately.
- Keep displayed defaults distinct from permission to use them. `--assume-yes` cannot answer semantic questions, and JSON/non-TTY cannot silently select a first/default member.
- Bind complete workspace roots and current authored intent; keep independent roots when a Redbook branch stops requiring a dependency.
- Recheck source/file preconditions under the existing lease/transaction before applying; preserve rollback/recovery and hooks/integrity policy.
- Retain original choices and exact default receipts so replay/update does not reinterpret resolved package bytes as user intent.

Those guarantees do **not** intrinsically require the person running local `vibe install` to start a daemon, bind a socket, copy a capture token, supply a shown-definition hash or hand-author low-level admission objects. In the local CLI, the existing trusted command composition can obtain and hold those objects itself. Capture/shown identities remain useful internal consistency and machine-protocol fields; manually moving them between commands is a property of the present embedding transport, not a dependency-solver requirement.

There is an important boundary: a displayed raw discovery JSON or arbitrary client request must still not be treated as source admission. Removing user-visible host ceremony means connecting the real source/preparation owner, not weakening that check.

## Existing main seams and the concrete missing connections

Current main already has the local authority and interaction owners needed to make a bounded integration:

- The ordinary direct-install command owns root resolution, lifecycle lease, selected manifest, effective offline posture, source construction, trace/finalization and the post-durability world callback. Its orchestrator `execute_prepared` builds the package source at `install/mod.rs:324`.
- `CliConfirmGate` asks about the selected destination/count and suspends progress while prompting. It respects project versus explicitly selected user-project scope and rejects missing TTY when confirmation is required (`commands/install/confirm.rs:79–115`). Its port is deliberately only `confirm_install(packages)` (`ports.rs:172`); it cannot currently return feature answers.
- Main's CLI already has themed `choose`, `choose_opt` and `choose_many` controls, cancellation and unified question rendering (`output/prompts/ask.rs:61,113`). E4's raw `dialoguer` calls and pretty-printed protocol JSON need not become a separate permanent interaction style.

The needed work is the missing local wiring at these owners, not a new planning system:

1. During normal install preparation, discover a choice-bearing candidate and form the real normalized consumer request/admitted definitions from the existing source/catalog and complete workspace declarations. An unanswered choice must reach a structured result that the local command can handle rather than falling out as an opaque failure.
2. Feed those typed questions to the local attended controls; return the same questions without terminal input for machine/unattended calls. Preserve explicit default consent and all evaluator semantics.
3. Stage the user's answers through the current authored-intent owner and existing recoverable transaction, with exact before-images/preconditions, before planning/applying the resulting desired world. This must resolve the source-bound-versus-authored equality guard and make a subsequent ordinary install/update retain the same intent. Do not mutate the manifest merely to preview questions.
4. Continue through the ordinary install confirmation, apply, trace/finalization and durable-world path with the selected request. Reusing the choice host's evaluation logic is possible; jumping wholesale to its early-return transport branch would discard ordinary command ownership unless those connections are preserved explicitly.

Concrete user outcome to close: an attended `vibe install` sees “Continuity: MUP / WAL” and currently active optional additions, makes explicit selections, sees the selected install destination/changes, confirms once, then gets installed slots/boot and reproducible retained intent. Today it can achieve the installed result only by already authoring the full configuration or by arranging the prepared host. The requested improvement is exactly the connection between those existing working halves.

The decisive future checks are correspondingly narrow: ordinary PTY install with no host/capture flags; omitted selection in JSON/non-TTY remains typed and write-free; explicit answer/default distinction; user cancellation before product writes; stale definition/manifest refusal; switch MUP→WAL→MUP preserving dormant choices and independent roots; a second ordinary fresh-process install/update retaining the authored choice. These are missing-path checks, not a claim that they passed in this inspection.
