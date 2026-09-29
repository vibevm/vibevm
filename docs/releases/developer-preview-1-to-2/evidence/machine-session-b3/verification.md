# M-16-B.3 selected verification

Observed on the local mutable 1.1.0 `next` checkout after `7f650331b`. All four selected commands exited 0. These are affected target families, not a workspace-wide, native-platform or mutation run.

| Target and filter | Passed | Retained log SHA-256 |
| --- | ---: | --- |
| `cargo test --locked --offline -p vibe-mcp --lib machine_session::` | 11 | `7a7c0429ec75c0e63e7166c8db748aa8b2528858a64f83bd12b09c1351523bdd` (`session.log`) |
| `cargo test --locked --offline -p vibe-orchestrator --lib machine_operation::exchange::tests::` | 4 | `f711b36977921ef85a37aa943cb753592f41519e2c3ec14e4092b51319d58d20` (`orchestrator.log`) |
| `cargo test --locked --offline -p vibe-mcp --lib tools::machine_operation::tests::` | 7 | `7e61c9b7ce08ad101ff3f8080e79ae783fd777f671cab72bcb176f57d8104440` (`mcp.log`) |
| `cargo test --locked --offline -p vibe-wire --test machine_session_contract` | 3 | `313943e52c2a1fc807abc889a18bcb76be014e03abafe65f3baa1b212115e6a8` (`wire.log`) |

The session family includes malformed/truncated/over-limit frame refusal before dispatch, exact frame round-trip and EOF, negotiated support sets, separate client/context identities, unavailable HOME/container contexts, pre-initialization/foreign/duplicate request refusal, one pure planner exchange and output-failure retention. The orchestrator and MCP families compare the shared planner exchange with owner validation and keep forged or diagnostic domain results out of normal publication. The wire family checks seven closed arms and generated-reader structural refusal.

The tests do not establish a public CLI entry for the frame runner, authenticated physical source bytes, durable replay/cursors, streaming, control, effects, native macOS/Windows execution or all operation-family coverage. Those remain outside this atom. Existing unrelated `vibe-safefs` dead-code and `vibe-cli` manifest-key warnings appear in the logs; no selected test failed.
