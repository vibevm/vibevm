# Campaign 3 · Ф0 — spikes (report)

_2026-07-11 18:12. Phase Ф0 of the Stage B descent-core plan
(`FRACTALITY-RLM-PLAN-v0.1`). Spikes carry no commits (plan §6); this
report + the state-plan record their outcomes. Floor untouched — no
product code changed._

## What was done

Four seam-probing spikes, to de-risk the riskiest seams before Ф1 code.

- **s1 schema-validate-at-seam — GREEN (ran).** Built a throwaway cargo
  project; jsonschema 0.47.0 compiles clean on this box's rustc 1.93.1
  (99-package lock). Validation works: a valid worker result passes; an
  invalid one (missing required `status`, a number in a string array)
  fails. The violation report — the retry-feedback shape — is
  `at <JSON-Pointer>: <message>`, e.g. `at ``: "status" is a required
  property` and `at `/files_changed/1`: 42 is not of type "string"`.
  API: `jsonschema::validator_for(&schema)` → `is_valid()` /
  `iter_errors()` with `err.instance_path()`.
- **s2 FileRef slice handoff — GREEN (inspection).** Machinery exists and
  is unit-tested: `RefRange::{Slice,Trim,Whole}` + `resolve_against(size)`
  (RFC 7233), `FileRef{fs,path,range,etag,sha256}`. Handoff = a parent's
  result FileRef placed in the child's new `context_from` field (Ф1.1).
  No unknowns.
- **s3 settings-injection promotion (CC) — GREEN (inspection).** The
  capability surface is argv (`--permission-mode` / `--allowed-tools` /
  `--disallowed-tools` from `profile.permissions`) + `--mcp-config`
  broker + a per-worker `CLAUDE_CONFIG_DIR`.
  `profile.permissions.allow_tools` already documents `Bash(fractality *)`
  as "the nesting seam"; promotion is spawning a child whose profile
  carries it. No in-place promotion (§10.2); worker-side hooks are out of
  scope (I5 — workers uninstrumented).
- **s4 escalated-outcome round-trip — DESIGN resolved.** Add a terminal
  `RunState::Escalated` + `EscalationRecord{reason, needs}` on RunRecord;
  the run climbs via the existing `parent` edges to the human at the top
  — a generalization of the D18 question/answer park channel (`question`/
  `answer` fields + `AnswerRule`). Viability proven from existing
  machinery; no new daemon.

## Decisions taken

- **s1's library is jsonschema 0.47.0**, default-features off (inline
  schemas, no remote `$ref`). output_schema validation lands at the
  pod/collect seam (Ф1.2); retry feedback = the `iter_errors` report.
- **All four seams are viable as designed** — Ф1 proceeds with no
  Decision rewrite (§6's "or its Decision rewritten in place" did not
  fire).
- **Delegation reality (field data).** opencode/GLM was tried for s1 and
  failed twice — first an `external_directory` permission reject on a
  nested cargo project (its own `.git`), then a silent launch stall
  (7 min, 0 artifacts). Per the live-observation law (blind-wait is the
  banned anti-pattern) the delegate was killed and s1 was done boss-side.
  Consequence: opencode is unreliable for cargo spikes on this box today;
  discipline-bound Ф1+ code is a boss-keep anyway (seam design), and
  floor/test runs will be backgrounded cargo (a reliable notification
  path), not opencode. Phase-5 playbook datum.

## Left undone / open

- s4's open question — worker expresses escalation via an ask_boss-style
  MCP tool vs a result-status field — deferred to Ф4 (its build phase).
- Real CC settings-injection was not spawn-tested end-to-end; s3 rests on
  inspection + the Campaign 2 precedent that the argv surface works. If
  promotion (Ф3) reveals a gap, it gets its own probe then.

## Косяки / висяки (honest)

- Two wasted delegate launches (~10 min wall clock) before the boss-side
  fallback. The watchdog caught the stall at the 3-min first-output rule,
  but only after a full first failed attempt — the Phase-5 lesson is a
  pre-flight opencode health probe before trusting it with a multi-step
  cargo task.
- The s1 spike is throwaway; the real output_schema validation (Ф1.2)
  must re-earn its own floor-green test — the spike proves the library,
  not our integration.

## Next

Ф1 slice 1 (D-C3-2 packet extensions), starting with `context_from`.
