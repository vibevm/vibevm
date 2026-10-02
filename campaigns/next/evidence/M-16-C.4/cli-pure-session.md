# Explicit pure local machine session

`vibe machine session --root <absolute-project-root>` starts the incumbent
byte-framed pure planner over stdio. It enters before ordinary CLI startup,
checks the physical project root, assigns one workspace context and target, and
writes only generated frames to stdout. Its negotiated formats are the existing
session frame, operation envelope and migration plan request/report/error; it
does not issue a fixed lifecycle host, C1 namespace, provider, listener, stream,
replay or control grant. `vibe mcp serve` remains newline JSON-RPC.

The CLI registration adds the `machine` group and `machine session` leaf to
`formats/capability-coverage/coverage-e1.json`. The bounded source byte delta
is `formats/capability-coverage/machine-session-source-delta.json`; the prior
selected source capture remains historical, and the new bytes are not assigned
to that capture.

Focused Linux proof with `cargo +nightly test -p vibe-cli --offline --locked`:
parser 2/2, real binary framed cases 5/5, progress policy 8/8, and the
existing MCP serve child-process regression 1/1. The exact native clap versus
coverage inventory test passed 1/1 after reconciliation. The binary fixture
checked generated frame and envelope decoding, one NoEffects plan, refusal and
malformed-frame exits, EOF, stdout purity, and no HOME/CWD/project mutation.
Worker sandbox tests used a private XFS reflink-seeded target; the accepting
parent reran the inventory case on the primary target. No full gate panel or
mutation campaign was run.

Effectful fixed Validate through CLI still needs the host-owned grant/reattach
seam and process-loss conformance. General lifecycle phases and explicit
force/trace/deploy-profile support remain outside this pure route.
