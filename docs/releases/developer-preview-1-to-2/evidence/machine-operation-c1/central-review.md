# M-16-C.1 scoped submission reservation review

The candidate foundation (`7ad768de4`, `a63f23ab3`) reserves an owner-selected run ID under the existing lifecycle lease, initializes the fixture domain journal with that same identity, re-reads it, durably marks `journal-ready`, and only then invokes the fixture effect. A repeated matching key attaches to the retained journal outcome; a different request/plan digest refuses. Expiry and explicit retirement retain compact key/run tombstones. The index contains namespace scope, hashes, run ID and ready state, not an execution result. It does not wrap the existing default lifecycle prelude or any CLI/MCP submit yet; those are C.2/C.4 obligations.

Independent review found that different keys could reserve the same `domain_run_id` before either journal existed. `967e5dfb2` closes that hole across all namespaces under the same lease and refuses a persisted index already containing duplicate run IDs. The PROP-080 reservation law now says this explicitly at req r3. Tests cover two keys sharing a proposed run before journal initialization, a malformed persisted duplicate, an unissued but syntactically valid namespace token, and post-publication uncertainty for both reservation and ready-marker writes. Neither uncertain write returns `Created` or successful ready; a retry re-reads the same key/run rather than allocating a second effect.

The fixture service proof (`893701557`) checks the durable ready marker from inside the effect callback, races eight identical submitters and observes one counted effect, retries after a lost response to the same run, refuses a preexisting/conflicting journal, and returns `RecoveryRequired` without another effect when the retained result is unavailable after namespace retirement. The reservation publication and ready-marker uncertainty tests are on the lifecycle index; the orchestrator fixture uses the same typed errors and does not turn them into an invocation.

| Selected check | Result | Log SHA-256 |
| --- | --- | --- |
| `cargo test --locked --offline -p vibe-lifecycle --lib state::submission::tests` | 12 passed | `40aaae076e36718dd9ae70ab8e8e49d579122585b35700a69fdbaadfaf21b673` ([log](reservation.log)) |
| `cargo test --locked --offline -p vibe-orchestrator --lib machine_submission::tests` | 9 passed | `e8941fefc4903e2a1c071315e6c82766e357efad1ac5c3bc02a28e24501b6f5c` ([log](fixture-service.log)) |

`cargo fmt -p vibe-lifecycle -p vibe-orchestrator -- --check`, XML parsing and the exact local campaign structure check passed. No mutation suite or broad workspace gate ran.

**Accepted C.1 boundary:** a host-scoped durable reservation and fixture journal-ready fence precede the fixture effect; matching concurrent/retried submissions share the same domain identity, while key/run collisions and uncertain publication refuse. Real lifecycle/build/deploy/publication journal recovery, long-lived operation ownership, and CLI/MCP/local-API adapter routing remain required in C.2/C.4. This proof does not claim public cross-surface duplicate-effect protection.
