# Hermes owned runner/session carrier — next bounded implementation

Date2026-11-19. Builder proposal implementing the accepted instance seam and native CLI ingress. No new public gateway framework, class/module patch, copied prompt assembly or operator HOME redirect.

## Proof owner and lifecycle

Use a distinct trusted throwaway proof module under `/data/git/vibevm-native-carrier-20261119/control/` and a clean candidate directory separate from downloaded sources. Python remains proof-only, never a shipped product. Actual `GatewayRunner(config=GatewayConfig())`, authority/listener and actual CLI first-run/exit inputs are already established and reused. The runner is one invocation-owned process in a netless namespace; native profile/auth and terminal behavior remain native.

A small instance-owned turn-context wrapper captures the original bound `_run_agent_build_turn_context`. On every admitted turn it verifies the exact native authority session ID, selected source/platform and invocation identity before supplying a constructor closure. It forwards every unrelated argument unchanged to the native original. Unrelated sessions/workers retain the original AIAgent factory. No module or class factory assignment.

The constructor closure creates the real native AIAgent with native kwargs, then attaches a cleaned instance carrier that captures and forwards the original bound `_build_system_prompt`. Prefix bytes are admitted once for the exact launch/session. Collision/re-attach attempts refuse rather than overwrite another owner. No synthetic substrate helper is included. The native original constructs stable/context/volatile text; the carrier supplies prefix plus exact caller context only.

## Cached-agent admission

Native cache is keyed by session_key and deliberately may reuse an agent for another live session_id. Therefore constructor injection alone is insufficient. The invocation's turn admission must check cached owned binding before every turn: same key/same ID may reuse the attached instance; same key/different ID must refuse or explicitly re-admit a new owned lifecycle before any prompt. Never claim native cache itself supplies exactsession isolation. Preserve native signature/message-count/dead-session invalidation and resource release; no second cache/global eviction.

Keep the instance wrapper attached while its admitted cached session/invocation remains alive. Teardown restores only exact owned instance attributes/listeners after native owned session shutdown; do not detach immediately after construction or touch unrelated cached agents. Restore the runner's original instance attribute on owned invocation teardown.

## Finite no-provider proof

1. Actual CLI admits its native authority session on the private native endpoint. Capture that real session identity, not a fabricated main/first-session heuristic.
2. Through the actual owned runner turn-context/fresh-agent owner, construct a real native AIAgent and invoke native prompt construction without model submission. Full admitted prefix appears once before exact caller/personal context while native stable base remains.
3. Unrelated actual authority session uses the original factory/no prefix. Quiet worker source never inherits the owned factory.
4. Native cached lookup reuses the admitted same-session instance; exactsamekey/different-ID admission refuses before construction/build, while native original cache behavior remains unchanged.
5. Native teardown/CLI exit restores owned attributes; class/module identity unchanged. Preserve warnings and do not turn no-model proof into cache-hit/resource/fork/resume claims.

## Product wiring boundary

After this finite proof and architectural review, integrate through existing Rust harness/composer/agent launcher owners with an explicitly owned Rust/TypeScript native bootstrap boundary. Do not ship a Python prototype, invent a custom UI or serialize credentials. Exact full CLI/options/resources/native workers and personal refresh remain required. Any additional native admission seam necessary to cover cached turns is a precise contract gap for review, not permission to patch shared gateway behavior.
