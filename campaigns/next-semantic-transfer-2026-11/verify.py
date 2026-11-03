#!/usr/bin/env python3
"""Check research coverage mechanically; never certify product acceptance."""

import argparse
import collections
import json
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--close", action="store_true")
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    root = here.parents[1]
    data = json.loads((here / "source-inventory.json").read_text())
    source = data["source_commit"]
    expected = []
    for name in git(root, "ls-tree", "-r", "--name-only", source,
                    "campaigns/next/tasks").splitlines():
        if name.endswith(".json"):
            tasks = json.loads(git(root, "show", f"{source}:{name}"))
            expected.extend(task["id"] for task in tasks["tasks"])
    actual = [task["id"] for task in data["tasks"]]
    if len(actual) != len(set(actual)) or len(expected) != len(set(expected)):
        raise ValueError("duplicate task record identity")
    if set(actual) != set(expected):
        raise ValueError("source task coverage differs from pinned Git input")
    allowed = {"implemented", "partial", "planned-only", "superseded", "unknown"}
    for task in data["tasks"]:
        if task["status"] not in allowed:
            raise ValueError(f"invalid source assessment: {task['id']}")
        if task.get("runtime_verified") is not False:
            raise ValueError("source-only inventory cannot assert runtime verification")
    ET.parse(root / "vibevm/vibespecs/terraforms/NEXT-SEMANTIC-TRANSFER-CAMPAIGN-v0.1.xml")
    for report in ["LOG.md", "RECONNAISSANCE.md"]:
        if not (here / report).read_text().strip():
            raise ValueError(f"missing research record: {report}")
    index = here / "commit-index.json"
    if index.exists():
        history = json.loads(index.read_text())
        # The compact index's commit list is evidence navigation, not review.
        rows = history["rows"]
        listed = [row["sha"] for row in rows]
        wanted = git(root, "rev-list", f"{data['target_commit']}..{source}").splitlines()
        if len(listed) != len(set(listed)) or set(listed) != set(wanted):
            raise ValueError("commit index differs from pinned Git range")
    print(json.dumps({
        "structural_check": "pass",
        "task_records": len(actual),
        "source_assessments": dict(collections.Counter(t["status"] for t in data["tasks"])),
        "commit_index_present": index.exists(),
        "product_acceptance": "not established by this check",
    }, sort_keys=True))
    if args.close:
        # Closure remains a reviewed result, never inferred from a census.
        print("Campaign closure unavailable: semantic history review, feature delivery "
              "and final product gates remain open.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError, ET.ParseError) as error:
        print(f"research coverage check failed: {error}", file=sys.stderr)
        sys.exit(1)
