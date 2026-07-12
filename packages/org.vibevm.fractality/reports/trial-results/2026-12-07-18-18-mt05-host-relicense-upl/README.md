# MT-05 dogfood (continued) — relicense the host root LICENSE.md to UPL-1.0

- **Date:** 2026-07-12
- **Pre-registration:** MT-05 —
  [`../../../fractality/v0.1.0/spec/manual-tests/MT-05-dogfood-relicense.md`](../../../fractality/v0.1.0/spec/manual-tests/MT-05-dogfood-relicense.md)
- **Run:** `01KXBEHEYJCQ1RNJ5657Q31HVA` — worktree mode against the host repo,
  profile `glm` / slot `small`, exit 0, **$0.388743**, ~42 s.
- **Task:** replace the host root `LICENSE.md` (the "EULA placeholder") with the
  canonical UPL-1.0 text + a third-party/refs note; touch nothing else. Owner
  authorisation: RP1 (relicense our EULA → UPL-1.0, verify the root VibeVM,
  minimal acceptance before merge).

## Result — PASS (boss-verified)

- Only `LICENSE.md` changed in the worktree (+ the worker's `result.md`); zero
  foreign edits.
- The UPL body is **byte-identical** to the canonical
  `packages/org.vibevm/wal-workspaces/v0.1.0/LICENSE.md` (`diff` differed only by
  the intended appended note); no EULA/proprietary text remains.
- Merged to `main` as `chore(license): relicense vibevm to UPL-1.0`. Host crates
  inherit it via `license-file.workspace = true`.

## Finding — E-BUG-001 (acceptance false-negative)

The run reported `acceptance: 0/2 ok`, but that was **spurious**:
`findstr /C:"multi word phrase"` lost its quoting through the pod's acceptance
runner — every word was treated as a filename (`FINDSTR: Cannot open to/the/…`,
see `acceptance.log` + `pod.log`). The deliverable was correct; the boss-side
`diff` + `grep` was the real gate. Filed as
[`../../../plans/external/E-BUG-001.md`](../../../plans/external/E-BUG-001.md).

## Files

- `license.diff` — the delivered change (old EULA placeholder → UPL-1.0).
- `acceptance.log`, `pod.log` — the E-BUG-001 evidence.
- `packet.toml`, `status.json`, `usage.json`, `worker-result.md`,
  `worker-stdout.jsonl.gz` — the run record and transcript.
