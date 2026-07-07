# tcg-bench — the Rust oracle baseline (differential corpus + latency)

_Recorded 2026-07-07 on this box: rust-analyzer 1.93.1 (the stable
toolchain component), rustc/cargo 1.93.1, `tcg-rust` from the 0.5.0
slot, project root `research/rust-demo`. Raw numbers:
[`bench-rust-2026-07-07-baseline.json`](bench-rust-2026-07-07-baseline.json);
harness: `tcg-rust bench --corpus research/tcg-bench/corpus-rust`.
The AGENTIC-TCG-RUST-PLAN §4.1/§4.2 predictions check against THIS
file._

## Result: agreement 9/9 — five compiler classes, two clean controls, the enrichment case, and the documented gap all hold

| case | oracle (r-a) | mapped | cargo check | verdict |
|---|---|---|---|---|
| 01-clean-disk | — | — | — | PASS |
| 02-clean-add (pure overlay, file never on disk) | — | — | — | PASS |
| 03-type-mismatch (Cyrillic content) | E0308 | E0308 | E0308 | PASS |
| 04-unresolved-name | E0425 | E0425 | E0425 | PASS |
| 05-wrong-arity | **E0107** | E0061 | E0061 | PASS |
| 06-newtype-privacy | *(silent)* | — | **E0423** | PASS — **the documented gap holds** |
| 07-unwrap-in-domain | — | — | — | PASS — `no-unwrap-in-domain` surfaces non-baselined via enrichment |
| 08-unknown-field | **E0559** | E0609 | E0609 | PASS |
| 09-missing-fields | E0063 | E0063 | E0063 | PASS |

The mapping table earned its keep twice (arity: r-a `E0107` vs rustc
`E0061`; unknown-field: r-a `E0559` vs rustc `E0609`) — existence-grain
agreement holds THROUGH the table, never by accident of identical
codes.

## The documented gap, pinned

Case 06 constructs `GuestName`'s private inner from another cell.
rust-analyzer 1.93.1's native diagnostics are SILENT on privacy even
with experimental diagnostics enabled; cargo check refuses with
`E0423` (the use-imported tuple-constructor shape; the module-path
form of the same defect is `E0603` — the Phase-0 spike measured that
one). The case asserts the asymmetry AS its expectation: the day a
future rust-analyzer starts reporting privacy, this case flips red
and the gap list gets re-curated instead of silently rotting. The
demo's brand rule stays compiler-checkable at the FLOOR — the gap is
the oracle's, and the corpus says so out loud (TCG-ORACLE-RUST §5).

The curation detail that produced one interim FAIL, recorded for the
next author: the farewell cell already imports `GuestName`, so a
seeded duplicate `use` added `E0252` noise to the cargo set — corpus
content must extend the REAL file, not restate its imports.

## Latency vs the posted targets (ORACLE-RUST §8)

| metric | posted target | measured |
|---|---|---|
| cold init → quiescent | < 15 s | **2 535 ms** |
| warm `validate` p50 | < 500 ms | **< 1 ms** (27 samples) |
| warm `validate` p95 | — | **65 ms** |

No target moved; every prediction in §4.2 holds with an order of
margin or more. (Cache-cold sysroot indexing measured 14.7 s in the
Phase-0 spike — still under the target; the bench box was OS-warm.)

## Honest limits

- n = 9 curated cases on a dependency-free demo tree; borrow-check
  subtleties, trait-solver edges, and macro-heavy code are the named
  open delta class (ORACLE-RUST §5) and no case claims them.
- The oracle's answers pass through r-a's EXPERIMENTAL diagnostics
  set, deliberately enabled (ORACLE-RUST §3); the floor
  (`discipline-rust floor` → cargo check) remains the truth.
- Latency on large consumer workspaces is unmeasured here; the
  product's 60 s first-request ceiling and the degraded flag carry
  that risk (R1).
