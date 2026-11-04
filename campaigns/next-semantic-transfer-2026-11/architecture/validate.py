#!/usr/bin/env python3
"""Validate architectural inventory and links, never product implementation."""
import argparse
import collections
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--visualization", type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    data = json.loads((here / "map.json").read_text())
    inventory = json.loads((here.parent / "source-inventory.json").read_text())
    crosswalk = json.loads(
        (here.parent / "research/old113-transfer-crosswalk.json").read_text()
    )
    rows = data["tasks"]
    by_id = {row["id"]: row for row in rows}
    source = {row["id"]: row for row in inventory["tasks"]}
    accepted = {row["old_task_id"] for row in crosswalk["rows"]}
    assert len(rows) == len(by_id) == len(source) == 222
    assert set(by_id) == set(source)
    assert {row["id"] for row in rows if row["prototypeAccepted"]} == accepted
    assert len(accepted) == 113
    assert data["meta"]["sourcePin"] == inventory["source_commit"]
    for row in rows:
        # The architectural judgment is pinned; current status is an explicit
        # visualization overlay, not a rewrite of that judgment on every tick.
        assert row["sourceAssessment"] in {"implemented", "partial", "planned-only", "unknown"}
        for key in ("title", "goal", "provided", "evidenceLevel"):
            assert row[key], (row["id"], key)
        assert "required" in row  # A foundational task may have no new prerequisite.
        hard = row["hardDependencies"]
        assert len(hard) == len(set(hard))
        assert row["id"] not in hard
        assert set(hard) <= set(by_id)
        assert all(edge["id"] in by_id for edge in row["interfaceDependencies"])

    # This is the task-level hard-capability projection. Interface and
    # qualification relations are retained separately, not erased to pass it.
    depth = {}
    active = set()

    def visit(task):
        if task in depth:
            return depth[task]
        assert task not in active, ("hard-capability cycle", task)
        active.add(task)
        level = 1 + max(
            (visit(dep) for dep in by_id[task]["hardDependencies"]), default=-1
        )
        active.remove(task)
        depth[task] = level
        return level

    for task, row in by_id.items():
        assert visit(task) == row["dependencyDepth"]

    layers = {layer["id"]: layer for layer in data["layers"]}
    features = {feature["id"]: feature for feature in data["features"]}
    assert len(layers) == 8 and len(features) == 18
    assert all(row["layer"] in layers and row["feature"] in features for row in rows)
    for layer in layers.values():
        members = [row for row in rows if row["layer"] == layer["id"]]
        assert layer["taskCount"] == len(members)
        assert layer["prototypeAccepted"] == sum(r["prototypeAccepted"] for r in members)
    membership = []
    for feature in features.values():
        membership.extend(feature["taskIds"])
        members = [by_id[task] for task in feature["taskIds"]]
        assert all(row["feature"] == feature["id"] for row in members)
        assert feature["prototypeAccepted"] == sum(r["prototypeAccepted"] for r in members)
        expected = {
            by_id[dep]["feature"]
            for row in members
            for dep in row["hardDependencies"]
            if by_id[dep]["feature"] != feature["id"]
        }
        assert set(feature["buildsOn"]) == expected
    assert len(membership) == 222 and set(membership) == set(by_id)

    hard_count = sum(len(row["hardDependencies"]) for row in rows)
    interface_count = sum(len(row["interfaceDependencies"]) for row in rows)
    assert data["meta"]["hardEdges"] == hard_count == 287
    assert data["meta"]["interfaceEdges"] == interface_count == 313
    assert data["meta"]["sourceAssessmentCounts"] == dict(
        collections.Counter(row["sourceAssessment"] for row in rows)
    )
    if args.visualization:
        ui = json.loads((args.visualization / "data.json").read_text())
        expected = json.loads(json.dumps(data))
        for row in expected["tasks"]:
            row.pop("historicalEdgeDispositions", None)
        if "contentRevision" in ui["meta"]:
            transfers = {row["old_task_id"]: row for row in crosswalk["rows"]}
            for row in expected["tasks"]:
                original = source[row["id"]]
                row["sourceAssessment"] = original["status"]
                overlay = {"source": {
                    "status": original["status"],
                    "evidence": original.get("evidence", []),
                    "missing": original.get("missing", []),
                    "runtimeVerified": original.get("runtime_verified", False),
                }}
                row["transferStatus"] = "outside-old113"
                overlay["transfer"] = {"status": "outside-old113"}
                if row["prototypeAccepted"]:
                    transfer = transfers[row["id"]]
                    row["transferStatus"] = transfer["status"]
                    overlay["transfer"] = {
                        "status": transfer["status"],
                        "acceptedMainCommits": transfer.get("accepted_main_commits", []),
                        "transferredScope": transfer.get("transferred_scope", ""),
                        "unmetOrUnverified": transfer.get("unmet_or_unverified", ""),
                    }
                row["statusOverlay"] = overlay
            for key, field, selected in (
                ("sourceAssessmentCounts", "sourceAssessment", expected["tasks"]),
                ("prototypeAcceptedSourceCounts", "sourceAssessment", [r for r in expected["tasks"] if r["prototypeAccepted"]]),
                ("transferCountsOld113", "transferStatus", [r for r in expected["tasks"] if r["prototypeAccepted"]]),
            ):
                expected["meta"][key] = dict(collections.Counter(r[field] for r in selected))
            status = json.loads((args.visualization / "status.json").read_text())
            assert status["ok"], "updater reports a failed refresh"
            for key in ("refreshedAt", "currentMainHead", "statusSourceUpdatedAt", "contentRevision", "snapshotId", "statusBasis"):
                assert ui["meta"][key] == status[key], ("snapshot mismatch", key)
                expected["meta"][key] = status[key]
            assert ui["meta"]["statusSources"] == status["sources"]
            for key, path in (
                ("architecture", here / "map.json"),
                ("inventory", here.parent / "source-inventory.json"),
                ("crosswalk", here.parent / "research/old113-transfer-crosswalk.json"),
            ):
                assert status["sources"][key]["sha256"] == hashlib.sha256(path.read_bytes()).hexdigest(), ("stale source", key)
            expected["meta"]["statusSources"] = status["sources"]
            expected["meta"]["overlayPolicy"] = "exact task IDs; architecture pins/generatedAt/layers/edges/historical113 unchanged"
            status_script = (args.visualization / "status.js").read_text()
            status_prefix = "window.VIBEVM_ARCHITECTURE_STATUS = "
            assert status_script.startswith(status_prefix) and status_script.endswith(";\n")
            assert json.loads(status_script[len(status_prefix):-2]) == status
        assert ui == expected, "visualization data diverged from canonical map"
        script = (args.visualization / "data.js").read_text()
        prefix = "window.VIBEVM_ARCHITECTURE_DATA = "
        assert script.startswith(prefix) and script.endswith(";\n")
        assert json.loads(script[len(prefix):-2]) == ui
    print(json.dumps({
        "structural_validation": "pass",
        "tasks": len(rows),
        "old_zap_accepted": len(accepted),
        "layers": len(layers),
        "feature_groups": len(features),
        "hard_capability_edges": hard_count,
        "interface_references": interface_count,
        "hard_graph_acyclic": True,
        "visualization_data_matches": bool(args.visualization),
        "product_acceptance": "not established by this structural validation",
    }, ensure_ascii=False))


if __name__ == "__main__":
    main()
