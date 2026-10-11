# First Agent operating incidents

Owner2026-10-12: continue the VibeVM campaign using available Zap, repair actual next-step blockers, record every observed failure/repair/workaround for later priority analysis. This is operational evidence, not a new acceptance gate.

INCIDENTS.jsonl records one stable incident per distinct occurrence: timestamp, stage, component, category, symptom, observed evidence, impact, cause or labelled hypothesis, action, affected files/commit, effort (unknown when not measured), recurrence/related incident, outcome, and proof of next-step progress. Append updates under the same incident_id. Distinguish operator mistakes, candidate-only defects, runtime/product failures, unresolved, workaround and verified fixes. Never include credentials, pairing URLs, raw personal messages or secret configuration. A source-only fix is not an installed fix; green partial checks are not full acceptance.

Root keeps First Agent authority and acceptance. Subordinate packets must return incident evidence for every encountered blocker/repair; Root merges it into this ledger. Do not broaden repair into fundamental redesign or speculative hardening unless required to unblock actual progression. Preserve the ledger for final promotion before campaign scaffold retirement.
