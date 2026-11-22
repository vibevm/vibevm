# Ready Hermes gateway: real Vibe composition and loss-of-binding decision

Independent architect research, 2026-11-19. Companion to `HERMES-ALTERNATIVE-NATIVE-PLUGIN-20261119.md`. Native pin `4ae451d4126327c871fd084e60c045dec0c85911`. No product change, upstream patch, real provider, harness change or new builder task. Not acceptance of unattended/shared-gateway product parity.

## What actually ran

Stock Hermes `gateway.run` in a new synthetic home, private `bwrap --unshare-all` sandbox, native ticketed discovery and owner-side plugin RPC, stock `hermes_cli.main -z … --resume` clients, instrumented loopback HTTP model. Host runtime/dependencies/binary read-only. The only model was a local stub; the label `synthetic-model` does not identify a paid provider. Research uses the stub by default. If a live smoke test is separately authorized, select an explicitly admitted lightweight model (owner examples: Haiku/Luna), not a costly default or automatic expensive fallback; do not change the operator profile.

The actual Vibe binary `2e44cf25a5a09194163e9bf87c862807e24d85728076f04e9ef0e112242762bc` generated the native `boot/local/AGENT-<sha>.md` during an informational Hermes launch on a synthetic project. The fixture did not substitute `prompt compose`'s API `SYSTEM` artifact or synthesize the framing itself. It checked the immutable filename digest, project text once, fresh personal V2 once after project, stale personal V1 absent. Artifact: **311777 bytes**, SHA256 `20a5be78fc2ab3a72dcb3c5378cd3f3de00284abed339b0e211582be04d602d9`. The RPC specified no arbitrary file path; the fixture knew its own project artifact. A healthy request modifier inserted the exact bytes before native `# Project Context` on one text wire shape. This is an experimental insertion point, not a production tier contract, evidence of native pre-compaction budgeting, independently generated per-worktree composition, or a cache measurement.

| Actual case | Observed main inference | Result |
|---|---|---|
| Bound managed session | 1 POST; full native artifact exactly once in system content | Normal local reply, exit 0 |
| Second stock client, same creator, same saved session, no new Vibe binding | 1 POST; full artifact **also present** | Session-only binding fails invocation isolation. This is not evidence of cross-principal access. |
| Binding revoked after RPC acknowledgement but before the turn | 0 chat completions; 1 separate `/api/show` metadata POST | `pre_llm_call` returned explicit block, CLI showed refusal **and exited 0**. Not zero HTTP or a machine-readable failure status. |
| Binding lost after pre-turn validation, at the request middleware | 1 chat completion, **zero copies** of the artifact | Request modifier raised; native client received normal local response, exit 0. Required fail-closed property falsified. |
| Ordinary unbound session on the same gateway | 1 chat completion; zero copies | Normal local reply, exit 0 |
| Proposed must-understand field on native `prompt.submit` | 0 HTTP requests during submit; no later captured inference for that fixture input | Unmodified owner refused `invalid_params` as source predicts. It did not drop the unknown field and proceed. Not a positive implementation of the proposed field. |

The first run failed its strict zero-*all*-HTTP assertion on the early veto. It is retained, not rewritten as a pass. Subsequent runs distinguished model metadata from chat completions and exercised the late failure and control. Source anchors: `gateway/session_controls.py:882` validates submission fields; `hermes_cli/gateway_chat_view.py:346` allocates `input_id` inside the stock client; `gateway/session_plugin_rpc.py:37` authorizes the scoped control call; `agent/turn_context.py:1362` handles explicit turn veto; `agent/turn_api_request.py:135–175` schedules optional request middleware after cache decoration; `hermes_cli/middleware.py:162–215` defaults to the base provider path on absent/failed execution callbacks. A second optional middleware can compensate for some faults while loaded, but does not make the host require the plugin. No test of TUI, retries, cross-process worker exclusion for this new design, or cold no-gateway startup is claimed.

## Recommended change, not authorized implementation

Keep plugin RPC, native ticket and native frontend. Move the **obligation** out of the optional callback: it belongs to a single authenticated submission at the authority. Minimal proposal: advertise a versioned optional `required_prompt_contribution` capability. A Vibe-managed client must explicitly request it on the submitting connection or bind it through an equivalent supported launch handoff; never infer intent from session ID, cwd, prompt text or client name. No silent retry without the field on an older gateway. The exact stock CLI cannot be assumed to send this field: a native client hook/option or an admitted submitter interface is needed; an earlier binding RPC on some other connection is insufficient.

Have the owner resolve a bounded immutable contribution for that turn's actual cwd, confirm creator/profile, digest, invocation and allowed audience, and keep it separate from the base prompt cached for other clients of the same session. Only non-secret references in durable admission metadata; no caller-chosen host file, duplicated personal artifact in shared history or raw content in diagnostics. Either pin an irrevocable snapshot for the lifetime of that admitted turn with an explicit revocation cutoff, or honor pre-dispatch revocation; define it once rather than introduce a check/use gap. Missing/expired/mismatched/unloaded required contribution -> structured refusal and nonzero managed CLI result, not an assistant message with exit 0. Apply before token accounting/cache layout, preserve native base/volatile tail and distinguish native auxiliary/child calls. Every admission/retry/recovery must enforce the same rule or refuse; zero-inference is not automatically zero network traffic.

Keep this in research until one candidate passes: good + neighboring client of the same session; late failure/reload with zero managed inference and explicit failed outcome; correct native request formatting, budgeting and lifecycle under retry/compaction, and exactly scoped child handling. Existing gateway only; cold terminal start remains a separate decision. No generic provider proxy, credentials changes, fabricated model refusal, cloning an operator profile, or weakening native authorization.

## Reproduction record

Architect-only research directory (not builder-owned): `/data/git/vibevm-architect-hermes-proof-UnE6SVQH/`. Run `bash sandbox.sh` where the same staged binary/dependencies exist; the child fixture writes only sandbox tmpfs. This reproduces stock limitations, not a production patch.

- `driver.py`: `a912c1537ff94ee96647f3eec9e991a92b0c228107e053bd3db0806aa570bf50`
- `plugin.py`: `ac206925380ea12da1293acc1ceaaa84e63785a392d89f9bf5954c621011e7f0`
- `sandbox.sh`: `d6f35330109c27fdbde8fdf792132b35c06755a7d2f0743aad5534ab12f1e9c4`
- Original strict failing receipt `experiment-1.log`: `22f53ef3da2b4be8ddef0006194a91c83076e23c4b4977d5a4fc9b0586564497`; classified result `experiment-2.log`: `6f8c9a0fb2a3fe17a2bf5456cb657d0aa4a51ac9770531b0612cd129d9435c67`; final `experiment-3.log`: `3120f94d2297fd5dbccedbce13aea22d13ca9cfb88a57f6040fd8c8fa6c203fe`.
