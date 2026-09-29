# M-16-B.3 selected verification

Observed on the local mutable 1.1.0 `next` checkout after `7f650331b`. All four selected commands exited 0. These are affected target families, not a workspace-wide, native-platform or mutation run.

| Target and filter | Passed | Retained log SHA-256 |
| --- | ---: | --- |
| `cargo test --locked --offline -p vibe-mcp --lib machine_session::` | 11 | `cf42f7312f14948d19e0dcbc6702b442086fd1c409b7f07558a88500429cbf76` (`session.log`) |
| `cargo test --locked --offline -p vibe-orchestrator --lib machine_operation::exchange::tests::` | 4 | `4cc10c197afc6de68382004ccb3e826da658730587cd021120a504f864dc511c` (`orchestrator.log`) |
| `cargo test --locked --offline -p vibe-mcp --lib tools::machine_operation::tests::` | 7 | `23815eaa9dc69446c9a8f09d0621e9492c5eead056b44fea1ab676bb0a3eabc9` (`mcp.log`) |
| `cargo test --locked --offline -p vibe-wire --test machine_session_contract` | 3 | `096b7b66e8bc5ef10c2a6ed1b741badfb66612f72a4f6916de555aeeb6f234c5` (`wire.log`) |

The session family includes malformed/truncated/over-limit frame refusal before dispatch, exact frame round-trip and EOF, negotiated support sets, separate client/context identities, unavailable HOME/container contexts, pre-initialization/foreign/duplicate request refusal, one pure planner exchange and output-failure retention. The orchestrator and MCP families compare the shared planner exchange with owner validation and keep forged or diagnostic domain results out of normal publication. The wire family checks seven closed arms and generated-reader structural refusal.

The tests do not establish a public CLI entry for the frame runner, authenticated physical source bytes, durable replay/cursors, streaming, control, effects, native macOS/Windows execution or all operation-family coverage. Those remain outside this atom. Existing unrelated `vibe-safefs` dead-code and `vibe-cli` manifest-key warnings appear in the logs; no selected test failed.
