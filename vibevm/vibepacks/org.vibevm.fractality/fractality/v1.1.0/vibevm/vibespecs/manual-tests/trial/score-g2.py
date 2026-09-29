#!/usr/bin/env python
"""MT-C3-01 scorer for the gated arm (g2). Reads target/trial-results/
arm-g2-run-<n>/ and prints, per run and pooled, the delegation metric
(delegated / attempted over eligible tasks E={1..6,9}, the C2 rubric) plus
the P-C3-a..d evidence. Analysis-only (python for a test tool, never
shipped product — the language law). Facts, not vibes: every number cites
its source file.

Usage:  python score-g2.py [<trial-results-dir>]
"""
import json
import pathlib
import re
import sys

E = {1, 2, 3, 4, 5, 6, 9}       # eligible (matrix-delegable) menu tasks
DISTRACTORS = {7, 8}            # matrix KEEPs these; delegating is an error
SILO = {3, 7, 10}               # cross-doc / whole-crate judgment (P-C3-d)


def load(p):
    try:
        return json.loads(pathlib.Path(p).read_text(encoding="utf-8"))
    except Exception:
        return None


def task_of(title):
    """Best-effort map a packet/run title to a menu task number."""
    t = title.lower()
    for n, kws in {
        1: ["parse_line", "test-suite", "parse-line", "grammar"],
        2: ["rename", "rec", "record"],
        3: ["facts", "vendor", "extract"],
        4: ["expected", "fixture", "json"],
        5: ["dedup"],
        6: ["to_string", "format!", "stringif"],
        7: ["error", "strategy", "memo", "decision"],
        8: ["typo", "parsse", "spell"],
        9:  ["manifest", "schema", "public-api", "public_fns"],
        10: ["invariant", "record-validity", "reconcile", "cross-module"],
    }.items():
        if any(k in t for k in kws):
            return n
    return None


def score_run(d):
    runs = load(d / "runs.json") or []
    escs = load(d / "escalations.json") or []
    tx = ""
    tp = d / "boss-transcript.jsonl"
    if tp.exists():
        tx = tp.read_text(encoding="utf-8", errors="replace")

    decisions = load(d / "decisions.json") or []
    decision_counts = {
        "inline": 0,
        "route": 0,
        "fold-local": 0,
        "spawn": 0,
        "escalate": 0,
    }
    for dec in decisions:
        v = dec.get("verdict")
        if v in decision_counts:
            decision_counts[v] += 1

    delegated_tasks = set()
    distractor_deleg = set()
    budget_kills = 0
    for r in runs:
        n = task_of(r.get("title", ""))
        if n in E:
            delegated_tasks.add(n)
        elif n in DISTRACTORS:
            distractor_deleg.add(n)
        if (r.get("kill_reason") or "") == "budget":
            budget_kills += 1

    gate_verdicts = re.findall(
        r"\b(inline|route|fold-local|spawn|escalate)\b(?=[^A-Za-z]*(?:verdict|reason|decision|$))",
        tx,
    )
    gate_calls = len(re.findall(r"fractality gate\b", tx))
    esc_gate = tx.count("escalate") and any(str(n) in tx for n in SILO)

    return {
        "workers": len(runs),
        "worker_states": [r.get("state") for r in runs],
        "delegated_E": sorted(delegated_tasks),
        "distractor_deleg": sorted(distractor_deleg),
        "budget_kills": budget_kills,
        "escalations": len(escs),
        "gate_calls": gate_calls,
        "gate_verdicts": gate_verdicts,
        "decision_counts": decision_counts,
    }


def main():
    base = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "target/trial-results")
    runs = sorted(base.glob("arm-g2-run-*"))
    if not runs:
        print(f"no arm-g2-run-* under {base}")
        return
    pooled_deleg = 0
    pooled_attempted = 0
    any_escalation = False
    any_budget_kill = False
    pooled_decisions = {
        "inline": 0,
        "route": 0,
        "fold-local": 0,
        "spawn": 0,
        "escalate": 0,
    }
    for d in runs:
        s = score_run(d)
        # attempted eligible = |E| when the boss addressed the menu (it always
        # does in these single-prompt runs); delegated = |delegated_E|.
        deleg = len(s["delegated_E"])
        attempted = len(E)
        pooled_deleg += deleg
        pooled_attempted += attempted
        any_escalation = any_escalation or s["escalations"] > 0
        any_budget_kill = any_budget_kill or s["budget_kills"] > 0
        for k in pooled_decisions:
            pooled_decisions[k] += s["decision_counts"][k]
        print(f"--- {d.name} ---")
        print(f"  workers={s['workers']} states={s['worker_states']}")
        print(f"  delegated E={s['delegated_E']} ({deleg}/{len(E)})  "
              f"distractors_delegated={s['distractor_deleg']}")
        print(f"  gate calls={s['gate_calls']} verdicts~={s['gate_verdicts'][:8]}")
        print(f"  escalations={s['escalations']}  budget_kills={s['budget_kills']}")
    print("=== pooled ===")
    pct = 100.0 * pooled_deleg / pooled_attempted if pooled_attempted else 0.0
    print(f"delegation metric (delegated/attempted over E): "
          f"{pooled_deleg}/{pooled_attempted} = {pct:.1f}%  "
          f"(C2 arm A baseline: 3/18 = 16.7%)")
    print(f"P-C3-c (no wall-budget kill): {'CONFIRMED' if not any_budget_kill else 'FALSIFIED'}")
    print(f"P-C3-d (a Silo task escalated): "
          f"{'CONFIRMED' if any_escalation else 'INCONCLUSIVE (no escalation observed)'}")
    print(f"pooled decision verdicts: {pooled_decisions}")
    route_inline = pooled_decisions["route"] + pooled_decisions["inline"]
    total_dec = sum(pooled_decisions.values())
    pc3a_pct = 100.0 * route_inline / total_dec if total_dec else 0.0
    print(f"P-C3-a (window-fit -> route/inline share): "
          f"{route_inline}/({total_dec}) = {pc3a_pct:.1f}%")
    print("P-C3-b (schema cuts rework): INCONCLUSIVE unless a packet set output_schema")


if __name__ == "__main__":
    main()
