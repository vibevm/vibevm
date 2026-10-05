# A0-GATE accepted: installed same-subject Rust + Lean proof path

The selected fixture correction was integrated as `50ee2cbf6`, following
A0.2 and A0.3 source acceptance. The final local receipt is
`/fast/git/v/research/2026-10-05-lean-a0-gate-receipt.json` (SHA-256
`6cee770b37204b19f708e27cb69fb7e2e4a2201cd76a005aebc69c46be66faa7`).
Its seven offline-installed Rust/Lean/core slots and 552 recorded files were
checked against their installation receipts. Four declared binaries were
built from those slots. The disposable consumer ran with both canonical
source checkouts hidden and networking disabled.

The isolated runtime transcript records 21 commands with the expected
positive or negative disposition. Installed Lean CLI check/verify and Lean
MCP verify agreed; host joined CLI check/verify and host MCP verify agreed;
the host Lean contribution matched the installed Lean witness. The actual
`requirements` front door returned one authored fact at
`spec://org.example/same-subject/Protocol#identity` with both current
relations: Rust `implements` and Lean `verifies`. The fact has no invented
numeric revision; both carriers bind that absence and exact content hashes,
while a request for revision 1 refuses. Installed Rust conform reported
zero findings on the selected crate and its usage doctest passed 1/1.
Targeted host negatives passed 18/18; the final requirements query
regression passed 1/1 on the integrated NEXT tree as well.

This proves the **selected capability gate**, not a universal dual-language
proof system. The host engine is compiled into the VibeVM binary from its
source-path dependency; it does not dynamically dispatch to the installed
Lean executable when package versions change. Runtime operation did not
need the source checkouts, but a new host build still needs the pinned engine
source. The supported Rust fixture is a native Linux dependency-free library
with exact named libtests; Lean yields G1 model proof and the join states
`correspondence=not_established`. There is no G3 Rust refinement claim,
generic checker plugin framework, cross-package inbound proof coverage,
public release or full product gate.

The historical 36 accepted NEXT contracts remain accepted without replay.
The owner-directed pause of the VibeVM Coordinator may be lifted after the
separate Zap P1-002 `pausing/unsupported` control state is repaired with
truthful execution and no duplicate native turn. A0-GATE acceptance alone
does not manufacture that operational recovery.
