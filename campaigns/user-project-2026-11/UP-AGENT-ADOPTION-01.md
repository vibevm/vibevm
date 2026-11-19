# USER-PROJECT — real-agent adoption and durable-memory journey

Genre: owner task / architectural acceptance brief. Date2026-11-19.
Status: planned; must reach a justified completed disposition before NST closure. Mechanical USER-PROJECT acceptance remains accepted in its actual scope; this additional behavioral criterion is not already proven by those tests.

## Owner mandate

> в задаче USER-PROJECT непонятно, насколько протестировано, что агенты на самом деле используют user project. Наши промты могут быть неверными. Я бы предложил добавить еще одну задачу в USER-PROJECT (которую конечно нужно доделать до окончания NST) - проверить на реальных агентах, насколько они склонны создавать user project при попытке сохранить какую-то информацию, локальную для проекта или для машины. Для этого можно использовать запускалку claude-ds, или если там будет отказ в доступе по API из-за лимитов или чего-то другого - то запускалку cc2 (это обертка над claude code с правильными параметрами, и она принимает на вход те же параметры что и обычный claude code). Ну и сделать какой-то временный проект-пример с использованием vibevm, на котором ты будешь всё это проверять. Если вдруг окажется, что user project как бы механически работает, но агенты не склонны его звать - тебе нужно продумать как изменить наши загрузочные инструкции, чтобы это всё произошло (соответственно пересоздат проект с новыми параметрами, и так проверять несколько раз, чтобы хотя бы поверхностно пользовательский сценарий был пройден)

## Outcome

In a fresh temporary VibeVM example project, a real agent encountering a natural request to remember project-local personal information or machine-local information selects the intended USER-PROJECT destination itself, safely initializes it when absent if needed, persists a usable record, and recovers that information in a separate fresh session. A human should not need to name the internal USER-PROJECT feature or dictate the storage command to make the product work.

Architect owns instruction design, prompt/control design, interpretation and contract review. Builder owns isolated fixtures, authorized execution, evidence capture and implementation integration. This task does not authorize access to real private corp inputs, ordinary auth stores, credential/endpoint changes, external publication or writes in the peer checkout.

## Execution lane and isolation

Use claude-ds first. The owner explicitly permits cc2 as the scoped alternative when an actual access/API refusal, exhausted limit or comparable availability failure prevents the primary lane. Record that failure before switching. cc2 accepts ordinary Claude Code parameters according to the owner; verify the actual launcher invocation without dumping its contents, credentials or environment. Behavioral failure is a test result, not a reason to switch models until one passes. This exception does not authorize other launchers, credential rotation, unbounded retries or bypassing a denied harness action.

Before any external run enforce the existing filesystem/process/network and credential-isolation boundary, isolated HOME/config, allowlisted environment and synthetic-sentinel checks. Approved execution-service credentials retain only their actual narrow grants. Do not copy normal harness config/auth or expose host directories just to make startup work. Missing isolation/runtime authority is a concrete blocker, not a pass or a reason to substitute an unapproved route.

Separate supervisor control from test-subject context. A non-central worker follows its explicit context-mode packet. The subject agent receives the admitted temporary project's authentic generated boot/entrypoint and allowed synthetic user environment, not the central coordinator's private plan or an evaluation rubric telling it the expected command. Apply the local full-boot requirement to medium/large execution work, but do not contaminate a behavioral adoption test with the host repository's instructions or central custody. If these requirements cannot be made consistent, resolve the packet boundary before launch rather than silently claiming a valid experiment.

All new example/task/worktree roots are under /data. Use a new owned empty test root per independent trial or changed instruction variant. Never reset the user's real personal project or clean unrelated retained runs.

## Finite baseline scenarios

Use synthetic facts only. Record exact natural user prompts and expected destination semantics before execution; do not reveal grading criteria or instruct the agent to invoke USER-PROJECT in the baseline.

1. **Project-local personal preference:** ask the agent to remember a synthetic personal workflow fact for this example project only, explicitly not as a shared repository rule. Expected: appropriate project-specific personal storage; do not overwrite tracked/shared instructions or leak into another project.
2. **Machine-local fact:** ask it to remember a synthetic machine/tool-path preference for future sessions on this machine, not as a universal project specification. Expected: appropriate personal machine/global storage under current product policy, not a shared project commit.
3. **Absent personal project:** at least one baseline starts without the isolated personal project. Expected: actual safe initialization and write through the supported USER-PROJECT flow rather than a promise to remember, an arbitrary home file or the harness's unrelated auto-memory.
4. **Fresh-session recall:** close the writing session and launch a genuinely fresh session with the same admitted isolated user context and project. Ask a question requiring the saved fact without repeating it in the prompt. Expected: actual loading/recall from the intended record, verified by tool/output evidence and stored bytes, not from continued conversation history.
5. **Boundary control:** a second example project must not inherit the first project's project-specific preference; shared/global machine data may apply only per the governing policy. A read-only ordinary boot must not initialize a missing personal project merely by starting.

These are completed user journeys, not a statistical benchmark. At least one real baseline for each distinct destination and a fresh recall/control is required; do not multiply equivalent prompts or platforms without a concrete defect. Use existing hermetic mechanical proofs for unchanged safety cases rather than rediscovering every negative path through a model.

## Blind baseline script (supervisor only)

Keep this script, the answer key, execution logs and other trials outside every subject mount. Give the subject only its authentic example-project boot and the single request for its current turn. Before starting, choose two independent synthetic values in the supervisor and substitute them only in the corresponding write requests. Do not encode them in project names, launch arguments for recall, shared files, or the recall questions. No hints after an incorrect answer.

| Session | Project | Request to the subject |
|---|---|---|
| P-write | A | “For my own reviews of this project I use the label `<project label>`. Please remember that for next time. It isn't a team convention.” |
| M-write, new history | A | “On this machine I keep my scratch exports in `<machine path>`, whichever project I am working on. Please remember that for next time.” |
| A-read, new history | A | “What label do I use for my own reviews of this project, and where do I keep scratch exports on this machine? Say if either is unknown.” |
| B-read, new history | B | Same question as A-read. The machine path may follow the user; the project-A label must not be applied to B. |

Use the same isolated user across these sessions. Start P-write without a personal project and retain a before/after check that simply opening the example did not create it. Judge each answer alongside the actual stored record, its scope and evidence of loading: a correct guess or ordinary harness auto-memory is not adoption. Classify an isolation/access failure before a subject can act as **unrun**, not as rejection of the product by an agent.

## Observe behavior, not only successful commands

Capture source/boot variant, exact generated entrypoint/lane hashes, launcher/version as observed, isolated context, prompts, tool actions, created files, record/index contents, session separation and final outcome. Classify destination selection, initialization, persistence and recall independently. A mechanically valid command after the operator tells the agent the answer is useful diagnosis but not autonomous adoption PASS. An agent claiming it saved a fact without bytes/evidence is not PASS. Provider access failure is distinct from agent choice failure.

Do not infer general reliability from a few runs. The report can establish a surface-level real scenario and identify observed failure patterns; it must not invent an adoption percentage from convenience samples.

## Repair instructions if adoption fails

Inspect the actual loaded instructions and user-visible path that produced the failure. Architect proposes a bounded change at the canonical boot/instruction owner: explain when information belongs in shared project specs versus project-specific personal state versus machine/global personal state, the supported first-write/init path and later loading. Avoid contradictory instructions directing the agent to unrelated harness auto-memory, and avoid teaching a hardcoded test phrase.

Preserve read-only boot, private/shared separation, exact selected-node authority and opt-out behavior. Do not solve adoption by putting personal values into shared SYSTEM prefixes or silently auto-initializing every boot. Product code changes require a real mechanism defect; do not add another memory subsystem for a prompting issue.

For each changed instruction variant recreate the example project from the declared inputs and run fresh writing/recall scenarios. Keep failed baseline and candidate evidence. Use an initial baseline plus up to two focused instruction revisions as the initial bounded episode; this is a planning bound, not a waiver if acceptance is still unmet. If no coherent user journey is achieved, report the specific unresolved cause and replan with the owner rather than entering an unbounded recursive prompt-tuning loop. Never repeat unchanged runs simply because the session or account changed.

## Acceptance and integration

- Real agent baseline and final revised behavior distinguish actual feature selection from guided mechanical execution.
- Both destination meanings and missing-project first write are exercised; fresh-session recall and cross-project boundary are demonstrated with synthetic data.
- Failure-driven canonical instruction changes, if needed, are reviewed, generated through the normal owner and tested on newly created projects. Actual loaded instruction hashes are recorded.
- No private data/credential leakage, ordinary user-state mutation, unauthorized fallback or fake agent/provider substitution.
- Final finite report includes successful, failed, access-blocked and unrun cases, exact candidate pins and known sampling limits; surface-level proof is not advertised as universal model reliability.
- Add the actual child under existing USER-PROJECT, make its completion a declared prerequisite of NST closure, and update source acceptance/evidence before the task-tree renderer. Previously accepted mechanical children remain intact; reopen only the affected parent/new criterion.
- Maintain NST-PHYSICAL-ACCEPTANCE.md where real attended/human claims change. This task does not silently waive remaining external-client or native-harness integration evidence.

The current owner route remains complete SYS-PROMPT, then semantically assess and finish the remaining NST obligations, including this task and the ordered global repair. No new competing campaign is introduced.
