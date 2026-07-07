# AI-Native Rust (Discipline v0.2) — boot snippet

Rust code in this project follows the AI-Native Rust guide
(`rust/GUIDE-AI-NATIVE-RUST.md` in this package). Read the guide when
authoring or reviewing structure; per-edit work needs only the card
whose trigger fires.

Card registry for Rust: `cards/INDEX.md` in this package (trigger → card;
the nine executable scaffolds A–I in their Rust shape). This stack ships
its own `cards/` projection — the weak-reader runtime surface for a `.rs`
edit is a Rust card's Band-3 ops block, never another language's.

Standing rules at the surface level:

- Ordinary idiomatic Rust at the token level — no invented dialect.
  Strictness lives in the envelope: newtypes/typestate at seams,
  runnable contracts at use sites, `#[spec]` metadata, per-cell
  fast verification (`cargo test -p <cell>`, < ~60s).
- Cells: one cell = one file-set, single registration point; cells
  import seams + core only, never sibling cells. Ambient coupling is
  forbidden.
- One `thiserror` enum per layer; error messages cite the violated
  `spec://` REQ and a fix surface. `unwrap`/`expect` in domain logic
  is forbidden by default — escape hatch is
  `#[spec(deviates, reason)]` with machinery.
- Every public seam carries one compiled doctest of canonical use.
  Replacing a non-trivial cell requires a differential oracle.
- Uniformity is load-bearing: one idiom per operation; exceptions
  are marked, or they propagate as false training signal.

The shipped toolchain (this stack materialises it; no dev tree needed):
`discipline-rust` — `init` (bootstrap policies + registries), `floor`
(fmt→test→clippy→conform→specmap→test-gate, one exit code), `health`
(the sweep's fact collector), `test-gate` / `tripwire` / `trace` /
`fast-loop` / `codemod` / `ledger`; plus the narrow `conform-rust` and
`specmap-rust` engines, and the agentic type oracle `tcg-rust`
(also served over MCP by `mcp:org.vibevm/rust-ai-native-mcp` — PROP-027)
(persistent enriching `serve` relay + one-shot
`validate`/`scope`/`complete`/`type`/`bench`: check an edit against
in-memory overlays BEFORE writing it, answered by the CONSUMER's own
rust-analyzer with the SAME conform rules as the gate — GUIDE §12, §13
move 8; prerequisite `rustup component add rust-analyzer`; honesty:
the oracle approximates, the floor stays the truth). Run vibe-natively
(`vibe bin exec discipline-rust -- <args>` — PROP-025 lockfile
dispatch; `vibe bin build` pre-builds), from PATH (`cargo install
--path vibedeps/<stack-slot>/crates/discipline-cli-rust`), or in place
via `cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml -p
discipline-cli-rust --bin discipline-rust -- <args>`. Wiring recipe:
GUIDE §13; sweep idioms: GUIDE §14. Procedures as agent skills:
`/discipline-sweep` (recurring), `/terraform-rust` (brownfield
adoption) — `vibe skill install` projects them.
