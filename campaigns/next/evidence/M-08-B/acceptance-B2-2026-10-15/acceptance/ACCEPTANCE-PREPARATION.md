# M08B2 acceptance preparation — no phase executed

Two standalone helpers compile exit0 against pinned immutable733 cached libraries. No Cargo/product build/test, store open, API mutation, process action or acceptance. Candidate original bothSatisfied/ReportedCompleted and Report/Patch bytes unchanged. Four candidate source hashes and every worker-artifacts entry verified, including raw failed fixture attempts.

Seven captured plans: V1 four fresh cases; V2 three unique upper child cases (not six summed summaries); V3/V4 two fresh each. Total11 fresh. V5 successfully validates exact matching reuse of two accepted B1 cases, original CLI/MCP executed=false; it is not a new test pass. V6 scoped format and V7 whitespace are static checks, not additional Rust cases. Helpers keep captured group argv and bind separate actual child commands/results/logs. Evidence ObservedPass for V5 means successful reuse validation ONLY, stated in method limitations and report.

Captured safe_stop.verifier is immutable V1 (verification.m08b2). Root full SafeBoundary attestation includes actual V2 public Plantrace/current four hashes and all seven groups; no verifier substitution. All original criterion requirement IDs are unchanged. Producer report is observed subject; Root independent review is retained separately by VerificationResult. No product bug reproduced; no product fix/production/native/vendor/cross-platform/codegen claim.

## Required Root inputs before execution

ROOT-REVIEW-PROPOSAL.json remains approved=false. Independent review and exact5 retirement were pending at preparation; nullable hashes deliberately block helper execution. Root must populate and independently check the three review report/sourcehash entries plus ROOT-RETIREMENT.json, preserve exact hashes, then write immutable ROOT-REVIEW.json approved=true. Never infer collection/retirement from local report files. Helpers require actual canonicalSucceeded/exactcandidate/collected-known-tail and retirement of original4knownPIDs plus exact fifth.

Current missing evidence hashes:
- /fast/git/v/research/2026-10-15-m08b2-review/REPORT.md
- /fast/git/v/research/2026-10-15-m08b2-review/REPORT.json
- /fast/zap-managed/2026-10-15-m08b2/ROOT-RETIREMENT.json

## Sequential Root-only recipe

Execute each phase separately and inspect exit before advancing. Do not execute these helpers while the original process tree is alive.

```sh
/fast/zap-managed/2026-10-15-m08b2/accept-candidate-local inspect
```

Safe helper takes exactly five original retired PIDs. Root may obtain the exact fifth from its verified receipt without guessing:

```python
from pathlib import Path
import json, subprocess
root = Path("/fast/zap-managed/2026-10-15-m08b2")
rows = json.loads((root / "ROOT-RETIREMENT.json").read_text())["after"]
assert len(rows) == 5 and all(x["exists"] is False for x in rows)
subprocess.run([str(root / "safe-candidate-local"), *[str(int(x["pid"])) for x in rows]], check=True)
```

Then, individually:

```sh
/fast/zap-managed/2026-10-15-m08b2/accept-candidate-local open-review /fast/zap-managed/2026-10-15-m08b2/ROOT-REVIEW.json
/fast/zap-managed/2026-10-15-m08b2/accept-candidate-local evidence /fast/zap-managed/2026-10-15-m08b2/ROOT-REVIEW.json
/fast/zap-managed/2026-10-15-m08b2/accept-candidate-local stage /fast/zap-managed/2026-10-15-m08b2/ROOT-REVIEW.json
/fast/zap-managed/2026-10-15-m08b2/accept-candidate-local work /fast/zap-managed/2026-10-15-m08b2/ROOT-REVIEW.json
/fast/zap-managed/2026-10-15-m08b2/accept-candidate-local inspect
```

The safe helper performs normal VerificationClaimed/ResultPassed/SafeStateSafe. Remaining phases use ordinary WorkTransitionActive→Candidate, seven EvidenceAdjudicated records, StageAcceptedFunctional and WorkAccepted with current exact basis and retained idempotency guards. No seed/issuer/DB primitive bypass. Final canonical WorkAccepted/work_stateAccepted is completion evidence; RuntimeUnreviewed projection is separate. Count remains80 until Root runs actual acceptance.
