# Universal consumer boot ownership — implementation and focused gates

Date2026-11-19. Builder direct implementation under the explicit owner outage instruction; architecture: `CONSUMER-BOOT-RECIPE-02.md`. Source base before this implementation `189950dab`; unrelated root instructions and materialized payload changes remain excluded.

## Architectural disposition

One crate-private policy literal belongs to the existing managed redirect owner and is re-exported only crate-internally. Router rendering inserts it after root/quiet scoping and before optional personal boot, in both static/no-static forms. Existing composition dispatch supports recipes1/2, defaults to2, and shares transport accounting without changing recipe1 bytes. Explicit1 receives an adjoining policy paragraph inside the existing generated replacement range; absent entrypoint and entrypoint=false reuse the existing contract segment. No new public framework, package, source marker, sidecar field, permission or boot command.

Source-only AGENT-LANE remains source-only. Existing native launch transports deliver that lane while preserving ordinary instruction-entrypoint discovery. Read mode retains the same router. Tests inspect actual published instruction files and actual carrier arguments/files rather than falsely asserting the policy was inserted into AGENT-LANE. New Hermes/Pi adapter preservation remains part of their own integration.

The architect inspected the actual source diff, frozen recipe1 golden, composition replacement range, schema/corpus adjustment, router budget and delivery owners without blocking design findings. No design debt is hidden behind the functional result.

## Actual checks

- Focused system-prompt tests:166 passed.
- Managed router tests:38 passed.
- Final real CLI fixture matrix:16 passed, including zero dependencies/no SYSTEM opt-in, all three entrypoints, authored CRLF preservation, byte+mtime-stable repeat, explicitrecipe1 SYSTEM publication/check, read/launch and selected-member/personal/worker behavior.
- Final sequential gates: `cargo fmt --all --check`; `cargo test --offline --locked -p vibe-workspace --lib` (983 passed,1 ignored, plus isolated1-test subprocess pass); `cargo xtask conform check`; `cargo xtask check-codegen`; `cargo xtask wire-diff` — all exit0.
- Wire-diff explicitly reports one changed boot-system-meta corpus, zero schema changes. Pre-publication allowance is not a no-change claim. SP-FMT117 and separate dated `SYS-TRANSPORT-DEPENDENCY-OWNERSHIP` fact record the amendment.

Initial full prompt run had3 failures from old defaultrecipe1/corpus expectations; tests were corrected to preserve their original meaning and the corpus refreshed from actual writer output. Initial router run had2 failures: the intentional wording golden and4176-byte router exceeding the former4096-byte bound. Router golden was deliberately amended; the guard retains its original allowance plus exactly policy length/two separators, rather than dropping the bound. Frozen recipe1 golden was not rewritten and was independently checked by the architect.

Evidence root `/data/git/vibevm-router-reconstruction-20261119/`: `consumer-boot-contract-tests.log`, `consumer-boot-system-tests.log`, `consumer-boot-system-tests-2.log`, `consumer-boot-router-tests.log`, `consumer-boot-router-tests-2.log`, `consumer-boot-cli-matrix-final.log`, `consumer-boot-final-gates.log`. Failed attempts remain retained.

## Publication and remaining boundaries

This receipt covers accepted generator behavior and focused fixtures; canonical SYS-TAILS roots still require ordinary regeneration using the rebuilt current-source Vibe, with authored-byte/idempotence verification. Host-derived delegation-rules neutralization remains a separate targeted materialization task. No whole-host force or hand-edited generated boot/dependency payload is part of this implementation.

Full host release self-check, native provider/cache matrix, Hermes/Pi full parity and USER-PROJECT adoption are not claimed by these focused gates. No model/provider call, host auth/config mutation, external push or publication was performed.
