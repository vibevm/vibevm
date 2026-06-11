# AI-Native Rust (Discipline v0.2) — boot snippet

Rust code in this project follows the AI-Native Rust guide
(`rust/GUIDE-AI-NATIVE-RUST.md` in this package). Read the guide when
authoring or reviewing structure; per-edit work needs only the card
whose trigger fires.

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
