#!/usr/bin/env python
"""MT-C3-02 scorer for the advisor help/hurt arms. Reads
target/trial-results/advise-<arm>-run-<n>/ (arm in {alone, advised}) and
prints, per run and pooled by arm, the per-task acceptance pass/fail (the
hidden tests in ../advise-assets, dropped into each run's proj-final/tests/
and run with `cargo test`), the ADVISED-ALONE pooled pass-rate delta
(PR-adv-1), the advice-call count per advised run from runs.json (PR-adv-2),
and the acceptance-failure comparison (PR-adv-3). Analysis-only (python for
a test tool, never shipped product — the language law). Facts, not vibes:
every number cites its source file, the score-g.py rule.

Usage:  python score-advise.py [<trial-results-dir>]
"""
import json
import pathlib
import shutil
import subprocess
import sys

# The three menu tasks, each pinned by one hidden acceptance test that ships
# in ../advise-assets/. The scorer copies each into a run's proj-final/tests/
# and runs `cargo test --test <name>`; exit 0 == the task passed acceptance.
SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
ASSETS = SCRIPT_DIR / "advise-assets"
TASKS = [
    (1, "task1_order_test.rs"),
    (2, "task2_count_test.rs"),
    (3, "task3_empty_test.rs"),
]


def load(p):
    """score-g.py rule: json-or-None so a missing/garbage file never crashes."""
    try:
        return json.loads(pathlib.Path(p).read_text(encoding="utf-8"))
    except Exception:
        return None


def is_advice_run(r):
    """An advice-marked bus run. The advised-arm preamble tells the caller to
    write advice packets with `title = "advice-taskN"` (preamble-advised.md),
    so the spawned advisor run carries "advice" in its title; we also accept
    an explicit class/kind/role/purpose marker == "advise" if the fabric sets
    one. Source: runs.json (== ps --json) per run dir."""
    t = (r.get("title") or "").lower()
    if "advice" in t or "advise" in t:
        return True
    for k in ("class", "kind", "role", "purpose", "variant"):
        if str(r.get(k) or "").lower() == "advise":
            return True
    return False


def arm_of(name):
    """advise-alone-run-1 -> 'alone', advise-advised-run-2 -> 'advised'."""
    if name.startswith("advise-alone-"):
        return "alone"
    if name.startswith("advise-advised-"):
        return "advised"
    return None


def score_task(run_dir, test_file):
    """Copy the hidden test into proj-final/tests/ and run cargo test on it.
    Returns (attempted, passed, note). attempted=False when the run produced
    no proj-final to test against (the caller never reached a project)."""
    proj = run_dir / "proj-final"
    if not proj.is_dir():
        return False, False, "no proj-final (caller produced no project)"
    tests_dir = proj / "tests"
    tests_dir.mkdir(exist_ok=True)
    src = ASSETS / test_file
    dst = tests_dir / test_file
    if not src.is_file():
        return False, False, f"missing asset {src}"
    shutil.copyfile(src, dst)
    # `cargo test --test <name>` builds only that one integration-test target
    # (plus the lib it links); exit 0 means every #[test] in the file passed.
    # A compile error — e.g. the worker named the function differently, or
    # broke the lib — is a non-zero exit and counts as a failed acceptance.
    target = test_file[:-3]  # strip ".rs"
    proc = subprocess.run(
        ["cargo", "test", "--test", target, "--no-fail-fast"],
        cwd=str(proj),
        capture_output=True,
        text=True,
    )
    if proc.returncode == 0:
        return True, True, "pass"
    combined = (proc.stderr or proc.stdout or "").strip().splitlines()
    # Prefer an informative cargo/rustc marker line over the trailing echo.
    test_result = next((l.strip() for l in combined if "test result:" in l.lower()), None)
    err_line = next((l.strip() for l in combined if l.lower().startswith("error")), None)
    panic = next((l.strip() for l in combined if "panicked at" in l.lower()), None)
    detail = (test_result or err_line or panic
              or (combined[-1] if combined else f"cargo exit {proc.returncode}"))
    return True, False, "fail: " + detail


def score_run(run_dir):
    """Source: runs.json (advice-call count) + proj-final (per-task verdict)."""
    runs = load(run_dir / "runs.json") or []
    advice_runs = sum(1 for r in runs if is_advice_run(r))
    per_task = {}
    for _n, tf in TASKS:
        attempted, passed, note = score_task(run_dir, tf)
        per_task[tf] = {"attempted": attempted, "passed": passed, "note": note}
    return {
        "arm": arm_of(run_dir.name),
        "workers": len(runs),
        "advice_runs": advice_runs,
        "per_task": per_task,
    }


def main():
    base = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "target/trial-results")
    run_dirs = sorted(d for d in base.glob("advise-*-run-*") if arm_of(d.name))
    if not run_dirs:
        print(f"no advise-<arm>-run-<n> under {base}")
        return

    pooled = {a: {"pass": 0, "attempt": 0, "fail": 0, "runs": 0}
              for a in ("alone", "advised")}

    for d in run_dirs:
        s = score_run(d)
        arm = s["arm"]
        pooled[arm]["runs"] += 1
        print(f"--- {d.name} (arm={arm}) ---")
        print(f"  workers={s['workers']}  advice_runs={s['advice_runs']}  "
              f"(source: runs.json)")
        for _n, tf in TASKS:
            pt = s["per_task"][tf]
            if pt["attempted"]:
                verdict = "PASS" if pt["passed"] else "FAIL"
                pooled[arm]["attempt"] += 1
                if pt["passed"]:
                    pooled[arm]["pass"] += 1
                else:
                    pooled[arm]["fail"] += 1
                print(f"  {tf}: {verdict}  ({pt['note']})")
            else:
                print(f"  {tf}: NOT-ATTEMPTED  ({pt['note']})")

    print("=== pooled by arm (source: proj-final/tests cargo test) ===")
    for arm in ("alone", "advised"):
        p = pooled[arm]
        rate = 100.0 * p["pass"] / p["attempt"] if p["attempt"] else 0.0
        print(f"  {arm:8s}: pass {p['pass']}/{p['attempt']} = {rate:.1f}%  "
              f"(failures={p['fail']}, runs={p['runs']})")

    pa, pv = pooled["alone"], pooled["advised"]
    rate_a = pa["pass"] / pa["attempt"] if pa["attempt"] else None
    rate_v = pv["pass"] / pv["attempt"] if pv["attempt"] else None

    print("=== predictions (frozen in MT-C3-02-advisor-help-hurt.md) ===")
    # PR-adv-1 (help): pooled acceptance-pass rate is higher in ADVISED.
    if rate_a is None or rate_v is None:
        print("PR-adv-1 (advice helps, advised pass-rate > alone): "
              "INCONCLUSIVE (an arm has no attempted task)")
    else:
        delta = rate_v - rate_a
        verdict = ("CONFIRMED" if rate_v > rate_a
                   else "FALSIFIED (advice did not raise the pass rate)")
        print(f"PR-adv-1 (advice helps): alone={rate_a:.3f} "
              f"advised={rate_v:.3f} delta={delta:+.3f} -> {verdict}")

    # PR-adv-2 (mechanism fires): every ADVISED run issued >=1 advise call.
    if pv["runs"] == 0:
        print("PR-adv-2 (every advised run advises): "
              "INCONCLUSIVE (no advised run)")
    else:
        fired = []
        for d in run_dirs:
            if arm_of(d.name) != "advised":
                continue
            runs = load(d / "runs.json") or []
            fired.append(sum(1 for r in runs if is_advice_run(r)))
        verdict = ("CONFIRMED" if all(c >= 1 for c in fired)
                   else "FALSIFIED (an advised run issued no advise call)")
        print(f"PR-adv-2 (every advised run advises): advice-run counts per "
              f"advised run={fired} (source: runs.json) -> {verdict}")

    # PR-adv-3 (no-hurt floor): ADVISED no more acceptance failures than ALONE.
    if pa["attempt"] == 0 or pv["attempt"] == 0:
        print("PR-adv-3 (advised fails <= alone): "
              "INCONCLUSIVE (an arm has no attempted task)")
    else:
        verdict = ("CONFIRMED" if pv["fail"] <= pa["fail"]
                   else "FALSIFIED (advice added hard failures)")
        print(f"PR-adv-3 (no-hurt floor): alone_failures={pa['fail']} "
              f"advised_failures={pv['fail']} -> {verdict}")


if __name__ == "__main__":
    main()
