#!/usr/bin/env python3
"""Read-only NEXT campaign checks and cold-execution packets (temporary tooling)."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import shlex
import subprocess
import sys
import tomllib
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
HOME = ROOT / "campaigns/next"
NS = "{https://vibevm.org/spec/1}"
TERMINAL = {"accepted", "deferred", "dropped", "superseded"}
TASK_FIELDS = {"id", "title", "goal", "read_paths", "write_paths", "steps",
               "positive_cases", "negative_cases", "checks", "acceptance",
               "safe_stop", "commit_subject", "notes"}
RENDERER = ROOT / ("vibevm/vibedeps/org.vibevm.world.multi-user-planning/1.0.0/"
                   "vibevm/vibespecs/skills/steward-goal/scripts/render_goal.py")


class Refusal(ValueError):
    pass


def need(condition, message):
    if not condition:
        raise Refusal(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def within(path):
    candidate = (ROOT / path).resolve()
    need(candidate.is_relative_to(ROOT), f"outside repository: {path}")
    return candidate


def temporary(path, manifest):
    value = os.path.normcase(str(within(path).relative_to(ROOT))).replace("\\", "/")
    zones = [os.path.normcase(z).replace("\\", "/") for z in manifest["temporary_zones"]]
    return any(value == z.rstrip("/") or (z.endswith("/") and value.startswith(z))
               for z in zones)


def require_unique(rows, what):
    ids = [r["id"] for r in rows]
    need(len(ids) == len(set(ids)), f"duplicate {what} identity")
    return {r["id"]: r for r in rows}


def effective_dependencies(nodes, key):
    dependencies, seen = [], set()
    while key:
        need(key in nodes and key not in seen, "invalid ancestor chain")
        seen.add(key)
        dependencies.extend(nodes[key]["depends_on"])
        key = nodes[key]["parent"]
    return list(dict.fromkeys(dependencies))


def combined_graph(nodes):
    """Edges name prerequisites, including a parent's required children."""
    graph = {key: effective_dependencies(nodes, key) for key in nodes}
    for key, node in nodes.items():
        parent = node["parent"]
        if parent and node["state"] not in {"deferred", "dropped", "superseded"}:
            graph[parent].append(key)
    return graph


def acyclic(graph):
    active, done = set(), set()

    def visit(key):
        need(key in graph, f"missing prerequisite {key}")
        need(key not in active, f"completion/dependency cycle through {key}")
        if key in done:
            return
        active.add(key)
        for dependency in graph[key]:
            visit(dependency)
        active.remove(key)
        done.add(key)

    for key in graph:
        visit(key)


def validate_state(plan):
    module = runpy.run_path(str(RENDERER))
    try:
        module["validate_plan"](plan)
    except module["GoalError"] as error:
        raise Refusal(str(error)) from error
    nodes = {n["id"]: n for n in plan["node"]}
    root = plan["root_node"]
    need(nodes[root]["parent"] == "", "root must have no parent")
    for key in nodes:
        cursor, seen = key, set()
        while cursor != root:
            need(cursor not in seen, "parent cycle")
            seen.add(cursor)
            cursor = nodes[cursor]["parent"]
            need(cursor in nodes, f"node outside root subtree: {key}")
    acyclic(combined_graph(nodes))
    for key, node in nodes.items():
        if node["state"] == "accepted":
            need(bool(node["evidence"]), f"accepted node without evidence: {key}")
            for dependency in effective_dependencies(nodes, key):
                need(nodes[dependency]["state"] in TERMINAL,
                     f"accepted node has open dependency: {key} -> {dependency}")
            for child in nodes.values():
                if child["parent"] == key:
                    need(child["state"] in TERMINAL,
                         f"accepted parent has open child: {key} -> {child['id']}")
    mandates = {m["id"]: m for m in plan["mandate"]}
    allowed = {"owned", "accepted", "deferred", "duplicate", "superseded",
               "declined", "out-of-scope"}
    for key, mandate in mandates.items():
        need(mandate["disposition"] in allowed, f"bad mandate disposition: {key}")
        need(mandate["nodes"], f"unowned mandate: {key}")
        for node_id in mandate["nodes"]:
            need(key in nodes[node_id]["mandates"], f"one-way mandate link: {key}")
    for node in nodes.values():
        for key in node["mandates"]:
            need(node["id"] in mandates[key]["nodes"], f"one-way node mandate: {key}")
    return nodes


def load_plan(context):
    if context:
        binding = tomllib.loads((context / "binding.toml").read_text(encoding="utf-8"))
        need(Path(binding["worktree_root"]).resolve() == ROOT, "context belongs to another worktree")
        branch = subprocess.run(["git", "symbolic-ref", "-q", "HEAD"], cwd=ROOT, capture_output=True, text=True)
        need(branch.returncode == 0 and binding["revision_selector"] == branch.stdout.strip(), "context branch differs")
        common = subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], cwd=ROOT, check=True, capture_output=True, text=True)
        need(Path(binding["git_common_dir"]).resolve() == Path(common.stdout.strip()).resolve(), "context repository differs")
    path = context / "plan.toml" if context else HOME / "plan.seed.toml"
    return tomllib.loads(path.read_text(encoding="utf-8"))


def load_tasks(manifest):
    result = {}
    groups = work_packages(manifest)
    refinements = manifest.get("refinements", [])
    require_unique(groups + refinements, "work-package")
    for parent in groups + refinements:
        path = within(parent["tasks_file"]) if "tasks_file" in parent else HOME / "tasks" / (parent["id"] + ".json")
        if parent in refinements:
            need(parent["id"] in result, "refinement parent must be an already declared task")
        raw = path.read_bytes()
        need(digest(raw) == manifest["task_files_sha256"][parent["id"]], "task contract changed; update shared/local plan witnesses explicitly")
        package = json.loads(raw.decode("utf-8"))
        need(package["id"] == parent["id"], "task filename/identity mismatch")
        need(2 <= len(package["tasks"]) <= 4, f"unexpected task decomposition: {parent['id']}")
        for index, task in enumerate(package["tasks"], 1):
            need(set(task) == TASK_FIELDS, f"task fields differ: {task.get('id')}")
            need(task["id"] == f"{parent['id']}.{index}", "child IDs must be stable and ordered")
            for key in ("title", "goal", "safe_stop", "commit_subject"):
                need(isinstance(task[key], str) and task[key].strip(), f"empty {key}: {task['id']}")
            for key in TASK_FIELDS - {"id", "title", "goal", "safe_stop", "commit_subject"}:
                need(isinstance(task[key], list), f"not a list: {task['id']}.{key}")
                need(all(isinstance(x, str) and x.strip() for x in task[key]), f"invalid list: {task['id']}.{key}")
                if key != "notes":
                    need(bool(task[key]), f"empty task contract: {task['id']}.{key}")
            need(re.match(r"^(feat|fix|docs|test|refactor|chore|build|ci|perf|style|revert)(\([^)]+\))?: .+", task["commit_subject"]),
                 f"invalid planned subject: {task['id']}")
            need(task["id"] not in result, f"duplicate task {task['id']}")
            result[task["id"]] = task
    validate_check_recipes(result)
    return result


def validate_check_recipes(tasks):
    """Catch broad literal defaults, not prove or execute a selected recipe."""
    obsolete = (r"(?:^|[.;]\s+)listed checks intentionally use unfiltered crate suites(?:\.|$)",
                r"^use full listed crate tests by default(?:\.|$)")
    for key, task in tasks.items():
        for note in task["notes"]:
            need(not any(re.search(pattern, note, re.IGNORECASE) for pattern in obsolete),
                 f"obsolete broad verification default: {key}")
        for command in task["checks"]:
            text = command.strip()
            if re.match(r"^(?:bash\s+)?(?:\./)?tools/self-check\.sh\b", text):
                raise Refusal(f"ordinary task cannot prescribe the full product panel: {key}")
            if not re.match(r"^cargo(?:\.exe)?\s+(?:\+\S+\s+)?test(?:\s|$)", text):
                continue  # SELECT BEFORE DISPATCH and conditional prose are not commands.
            args = shlex.split(text)
            args = args[:args.index("--")] if "--" in args else args
            need(not any(arg.split("=", 1)[0] in
                         {"--workspace", "--all", "--all-targets", "--all-features",
                          "--tests", "--bins", "--examples", "--benches"}
                         for arg in args), f"broad Cargo test default: {key}")
            selected = "--lib" in args or "--doc" in args
            for index, arg in enumerate(args):
                name = ""
                named = arg in {"--test", "--bin"} or arg.startswith(("--test=", "--bin="))
                if arg in {"--test", "--bin"} and index + 1 < len(args):
                    name = args[index + 1]
                elif arg.startswith(("--test=", "--bin=")):
                    name = arg.split("=", 1)[1]
                if named:
                    need(name and not name.startswith("-") and not any(c in name for c in "*?["),
                         f"Cargo test needs a concrete named compile target: {key}")
                    selected = True
            need(selected, f"Cargo test must select a compile target before listing/filtering: {key}")


def verification_policy(manifest):
    path = manifest.get("verification_policy")
    need(isinstance(path, str) and path.strip(), "verification-policy binding missing")
    policy = read_json(within(path))
    fields = {"schema", "ordinary_gate_mode", "full_panel_gate", "baseline_gate",
              "workstream_gates", "policy_spec", "binding_required", "rules",
              "workstream_acceptance", "baseline_acceptance", "final_acceptance"}
    need(isinstance(policy, dict) and set(policy) == fields,
         "verification-policy fields differ")
    need(type(policy["schema"]) is int and policy["schema"] == 1,
         "unsupported verification-policy schema")
    need(policy["ordinary_gate_mode"] == "affected-targets" and
         policy["binding_required"] is True, "verification binding must remain required")
    need(policy["full_panel_gate"] == "NEXT-CLOSE.3" and
         policy["baseline_gate"] == "NEXT-P0.2", "verification gate identity differs")
    expected = ["NEXT-GATE-" + row["id"] for row in manifest["milestones"]]
    need(policy["workstream_gates"] == expected and len(set(expected)) == 17,
         "verification workstream gate coverage differs")
    need(policy["policy_spec"] == "spec://org.vibevm.world/multi-user-planning/flows/"
         "multi-user-planning/verification-selection#root", "verification policy authority differs")
    for key in ("workstream_acceptance", "baseline_acceptance", "final_acceptance"):
        need(isinstance(policy[key], str) and policy[key].strip(),
             f"verification acceptance missing: {key}")
    need(isinstance(policy["rules"], list) and policy["rules"] and
         all(isinstance(rule, str) and rule.strip() for rule in policy["rules"]),
         "verification rules missing")
    return policy


def validate_verification_policy(manifest, nodes, tasks):
    """Validate scheduled obligations; no binding or test verdict is established."""
    policy = verification_policy(manifest)
    groups = [row["id"] for row in manifest["milestones"]]
    obsolete = ("Run actual full tools/self-check.sh", "Pass the coherent full panel",
                "From the next checkout run Git Bash tools/self-check.sh")
    for key in groups + policy["workstream_gates"] + [policy["baseline_gate"], "NEXT-P0-GATE"]:
        need(not any(line.startswith(obsolete) for line in nodes[key]["acceptance"]),
             f"obsolete unconditional full panel remains: {key}")
    for key in groups + policy["workstream_gates"]:
        need(key in nodes and policy["workstream_acceptance"] in nodes[key]["acceptance"],
             f"targeted workstream acceptance missing: {key}")
    for key in (policy["baseline_gate"], "NEXT-P0-GATE"):
        need(key in nodes and policy["baseline_acceptance"] in nodes[key]["acceptance"],
             f"targeted baseline acceptance missing: {key}")
    key = policy["full_panel_gate"]
    need(key in nodes and policy["final_acceptance"] in nodes[key]["acceptance"],
         f"comprehensive final acceptance missing: {key}")
    validate_check_recipes(tasks)
    return policy


def work_packages(manifest):
    """Additional campaign obligations do not rewrite the frozen baseline graph."""
    supplements = manifest.get("supplemental_work_packages", [])
    need(isinstance(supplements, list), "supplemental work packages must be a list")
    for group in supplements:
        need(isinstance(group, dict) and
             all(isinstance(group.get(key), str) and group[key].strip()
                 for key in ("id", "owner", "tasks_file")),
             "invalid supplemental work package")
        need(isinstance(group.get("depends_on"), list) and
             all(isinstance(key, str) and key for key in group["depends_on"]),
             "invalid supplemental prerequisites")
    return manifest["atoms"] + supplements


def package_ancestor(manifest, task_id):
    """Resolve a refined child through its declared baseline/supplemental owner."""
    groups = {row["id"]: row for row in work_packages(manifest)}
    key = task_id
    while "." in key:
        key = key.rsplit(".", 1)[0]
        if key in groups:
            return groups[key]
    raise Refusal(f"task has no declared work-package ancestor: {task_id}")


def prerequisite_reachable(nodes, key, prerequisite):
    """A dependency on a group also waits for that group's required children."""
    graph = combined_graph(nodes)
    pending, seen = list(graph[key]), set()
    while pending:
        current = pending.pop()
        if current == prerequisite:
            return True
        if current not in seen:
            need(current in graph, f"missing prerequisite {current}")
            seen.add(current)
            pending.extend(graph[current])
    return False


def source_contracts(manifest, ledger):
    units = {u["id"]: u for u in manifest["units"]}
    status = require_unique(ledger["unit"], "unit")
    clauses = require_unique(ledger["clause"], "clause")
    expected = {a for u in units.values() for a in u["anchors"]}
    need(set(status) == set(units), "unit inventory drift")
    need(set(clauses) == expected, "clause inventory drift: counts alone are insufficient")
    for key, unit in units.items():
        row = status[key]
        need(row["state"] in {"active", "retired"}, f"bad unit state: {key}")
        path = within(unit["path"])
        if row["state"] == "active":
            need(path.is_file(), f"active source missing: {key}")
            need(digest(path.read_bytes()) == row["source_sha256"], f"source contract drift: {key}; replan explicitly")
            tree = ET.parse(path)
            for anchor in unit["anchors"]:
                found = list(tree.iter(NS + anchor))
                need(len(found) == 1, f"source anchor missing/ambiguous: {anchor}")
                need(digest("".join(found[0].itertext()).encode()) == clauses[anchor]["source_sha256"],
                     f"source clause drift: {anchor}")
        else:
            need(not path.exists(), f"retired source still present: {key}")
            for anchor in unit["anchors"]:
                validate_promotion(clauses[anchor], manifest)
    return units, status, clauses


def validate_promotion(clause, manifest):
    key = clause["id"]
    need(clause["state"] == "promoted", f"unpromoted clause: {key}")
    need(clause.get("meaning_review") and clause.get("reviewer"), f"semantic review absent: {key}")
    need(re.fullmatch(r"[0-9a-f]{40}", clause.get("acceptance_commit", "")), f"accepted commit absent: {key}")
    need(clause.get("targets") and clause.get("evidence"), f"destination/evidence absent: {key}")
    for target in clause["targets"]:
        need(not temporary(target["path"], manifest), f"promotion targets scaffolding: {key}")
        path = within(target["path"])
        need(path.is_file(), f"permanent target missing: {path}")
        need(digest(path.read_bytes()) == target["sha256"], f"permanent target changed: {key}")
        anchor = target.get("anchor", "")
        need(anchor, f"permanent normative anchor absent: {key}")
        if path.suffix == ".xml":
            tree = ET.parse(path)
            matches = [e for e in tree.iter() if e.tag == NS + anchor or e.get("id") == anchor]
            need(len(matches) == 1, f"permanent anchor missing/ambiguous: {key} {anchor}")
        else:
            need("{#" + anchor + "}" in path.read_text(encoding="utf-8"), f"explicit markdown anchor missing: {key}")
    for evidence in clause["evidence"]:
        need(not temporary(evidence["path"], manifest), f"evidence would be deleted: {key}")
        path = within(evidence["path"])
        need(path.is_file() and digest(path.read_bytes()) == evidence["sha256"], f"evidence missing/changed: {key}")
    result = subprocess.run(["git", "merge-base", "--is-ancestor", clause["acceptance_commit"], "HEAD"], cwd=ROOT, capture_output=True)
    need(result.returncode == 0, f"acceptance commit not in current ancestry: {key}")
    for witness in clause["targets"] + clause["evidence"]:
        blob = subprocess.run(["git", "show", clause["acceptance_commit"] + ":" + witness["path"]],
                              cwd=ROOT, capture_output=True)
        need(blob.returncode == 0 and digest(blob.stdout) == witness["sha256"],
             f"witness not bound to accepted commit: {key} {witness['path']}")


def validate_coverage(manifest, tasks, plan):
    nodes = validate_state(plan)
    need(set(tasks).issubset(nodes), "task contract missing from plan")
    parents = {a["id"] for a in manifest["atoms"]}
    expected_nodes = set(tasks) | parents | {m["id"] for m in manifest["milestones"]} | set(manifest["auxiliary_nodes"])
    need(set(nodes) == expected_nodes, "local/shared node inventory differs; merge/replan by stable IDs")
    need(set(manifest["baseline_graph"]) == parents and len(parents) == 86, "baseline parent coverage lost")
    for parent in work_packages(manifest):
        need(parent["id"] in nodes, f"work-package parent missing: {parent['id']}")
        need(nodes[parent["id"]]["depends_on"] == parent["depends_on"], f"work-package prerequisites changed: {parent['id']}")
        need(nodes[parent["id"]].get("contract_sha256") == manifest["task_files_sha256"][parent["id"]], "work-package contract witness stale")
        owner = parent.get("owner", parent.get("milestone"))
        need(owner in nodes and nodes[parent["id"]]["parent"] == owner,
             f"wrong work-package owner: {parent['id']}")
    for key in tasks:
        parent, suffix = key.rsplit(".", 1)
        need(nodes[key]["parent"] == parent, f"wrong task parent: {key}")
        need(nodes[key].get("contract_sha256") == manifest["task_files_sha256"][parent], f"task contract witness stale: {key}")
        if int(suffix) > 1:
            previous = parent + "." + str(int(suffix) - 1)
            need(previous in nodes[key]["depends_on"], f"child predecessor missing: {key}")
        need(set(manifest["extra_dependencies"].get(key, [])).issubset(nodes[key]["depends_on"]),
             f"resource/authority prerequisite missing: {key}")
    optional = set(manifest["optional_nodes"])
    for key, node in nodes.items():
        if node["state"] in {"deferred", "dropped", "superseded"}:
            need(key in optional, f"required node cannot silently disappear: {key}; replan shared disposition explicitly")
    if nodes["NEXT-EXECUTION-AUTHORITY"]["state"] != "accepted":
        for key, node in nodes.items():
            if key in tasks or key.startswith("NEXT-P0"):
                need(node["state"] not in {"active", "candidate", "accepted"},
                     f"planning-only hold forbids started campaign work: {key}")
    if any(group["owner"] == "NEXT-PREVIEW" for group in manifest.get("supplemental_work_packages", [])):
        need(manifest.get("preview_control"), "preview-control binding missing")
    if manifest.get("preview_control"):
        validate_preview_control(manifest, nodes, tasks)
    validate_verification_policy(manifest, nodes, tasks)
    return nodes


def preview_control(manifest):
    path = manifest.get("preview_control")
    need(isinstance(path, str) and path.strip(), "preview-control path missing")
    control = read_json(within(path))
    need(isinstance(control, dict) and type(control.get("schema")) is int and control["schema"] == 1,
         "unsupported preview-control schema")
    need(control.get("state") == "planned", "preview control is a planning contract")
    need(control.get("from_release") == "developer-preview-1" and
         control.get("to_release") == "developer-preview-2", "preview release pair differs")
    for key, policy in (("baseline", "capture_before_first_implementation"),
                        ("target", "seal_after_scaffold_removal")):
        row = control.get(key)
        need(isinstance(row, dict) and row.get("state") == "pending_capture" and
             row.get("policy") == policy, f"invalid preview {key} capture policy")
    docs = control.get("documentation")
    need(isinstance(docs, dict) and docs.get("state") == "awaiting_owner_source" and
         isinstance(docs.get("gate"), str) and docs["gate"],
         "invalid preview documentation binding")
    for key in ("contract_unit", "bootstrap_gate", "final_gate", "retirement_node",
                "post_removal_gate", "workstream_acceptance", "closure_acceptance"):
        need(isinstance(control.get(key), str) and control[key].strip(),
             f"invalid preview-control {key}")
    rules = control.get("task_rules")
    need(isinstance(rules, list) and rules and
         all(isinstance(rule, str) and rule.strip() for rule in rules),
         "preview task rules missing")
    homes = control.get("permanent_homes")
    need(isinstance(homes, dict) and homes, "preview permanent homes missing")
    for label, path in homes.items():
        need(isinstance(label, str) and label and isinstance(path, str) and path.strip(),
             "invalid preview permanent home")
        need(not Path(path).is_absolute() and not re.match(r"^[A-Za-z]:", path),
             "preview permanent home must be repository-relative")
        need(not temporary(path, manifest), f"preview permanent home targets scaffolding: {label}")
    return control


def validate_preview_control(manifest, nodes, tasks):
    """Prove scheduled obligations only, never actual release/change-record truth."""
    control = preview_control(manifest)
    units = {unit["id"]: unit for unit in manifest["units"]}
    unit_id = control["contract_unit"]
    need(unit_id in units, "preview contract unit missing")
    unit = units[unit_id]
    owner = unit["owner"]
    need(owner in nodes and nodes[owner]["parent"] == "NEXT", "preview workstream missing")
    for key in ("bootstrap_gate", "final_gate", "retirement_node", "post_removal_gate"):
        need(control[key] in nodes, f"preview gate missing: {key}")
    docs_gate = control["documentation"]["gate"]
    need(docs_gate in nodes, "preview documentation gate missing")
    need(control["retirement_node"] == "RETIRE-" + unit_id,
         "preview retirement identity differs")
    for gate in unit.get("retire_after", [owner]):
        need(gate in nodes and gate in nodes[control["retirement_node"]]["depends_on"],
             "preview retirement does not wait for its declared workstream gate")
    for workstream in [owner] + [row["id"] for row in manifest["milestones"]]:
        need(control["workstream_acceptance"] in nodes[workstream]["acceptance"],
             f"preview workstream acceptance obligation missing: {workstream}")
    for closing in ("NEXT", "NEXT-CLOSE", control["post_removal_gate"]):
        need(control["closure_acceptance"] in nodes[closing]["acceptance"],
             f"preview closure acceptance obligation missing: {closing}")
    mandate = "NEXT-PREVIEW-MIGRATION-DOCS"
    need(all(mandate in nodes[key]["mandates"] for key in (owner, "NEXT")),
         "preview owner mandate missing from root/workstream")
    for parent in manifest["atoms"]:
        first = parent["id"] + ".1"
        need(control["bootstrap_gate"] in effective_dependencies(nodes, first),
             f"preview bootstrap prerequisite missing: {first}")
    supplements = manifest.get("supplemental_work_packages", [])
    need(supplements and all(group["owner"] == owner for group in supplements),
         "preview supplemental owner differs")
    for group in supplements:
        for key in tasks:
            if key.startswith(group["id"] + "."):
                need(prerequisite_reachable(nodes, key, "NEXT-EXECUTION-AUTHORITY"),
                     f"preview task lacks execution authority prerequisite: {key}")
    bootstrap_parent = control["bootstrap_gate"].rsplit(".", 1)[0]
    need(bootstrap_parent + ".1" in tasks, "preview baseline setup task missing")
    need("NEXT-P0-GATE" in effective_dependencies(nodes, bootstrap_parent + ".1"),
         "preview baseline setup must wait for Phase 0")
    closing = effective_dependencies(nodes, "NEXT-CLOSE.1")
    need(control["retirement_node"] in closing and control["final_gate"] in closing,
         "preview final/retirement gate missing from closure")
    need(control["post_removal_gate"] == "NEXT-CLOSE.3" and
         "NEXT-CLOSE.2" in effective_dependencies(nodes, control["post_removal_gate"]),
         "preview target seal must follow scaffold removal")
    return control


def check(manifest, context):
    for source in manifest["baseline_files"]:
        need(digest(within(source["path"]).read_bytes()) == source["sha256"], "frozen baseline altered")
    tasks = load_tasks(manifest)
    plan = load_plan(context)
    nodes = validate_coverage(manifest, tasks, plan)
    parents = {a["id"] for a in manifest["atoms"]}
    need(set(manifest["baseline_graph"]) == parents and len(parents) == 86, "baseline parent coverage lost")
    for parent in manifest["atoms"]:
        need(parent["id"] in nodes, f"baseline parent missing: {parent['id']}")
        need(nodes[parent["id"]]["depends_on"] == parent["depends_on"], f"baseline prerequisites changed: {parent['id']}")
    for key, task in tasks.items():
        need(nodes[key]["parent"] == key.rsplit(".", 1)[0], f"wrong task parent: {key}")
    ledger = tomllib.loads((HOME / "promotion.toml").read_text(encoding="utf-8"))
    units, status, clauses = source_contracts(manifest, ledger)
    return {"ok": True, "mode": "authoring-structure", "product_gates_executed": False,
            "baseline_work_packages": len(parents),
            "baseline_implementation_tasks": sum(key.rsplit(".", 1)[0] in parents for key in tasks),
            "supplemental_tasks": sum(key.rsplit(".", 1)[0] in
                                      {row["id"] for row in manifest.get("supplemental_work_packages", [])}
                                      for key in tasks),
            "implementation_tasks": len(tasks),
            "plan_nodes": len(nodes), "contract_units": len(units), "source_clauses": len(clauses),
            "retired_units": sum(x["state"] == "retired" for x in status.values()),
            "execution_hold": nodes["NEXT-EXECUTION-AUTHORITY"]["state"],
            "preview_change_records_verified": False,
            "verification_bindings_executed": False,
            "open_owner_decisions": [d["id"] for d in manifest["decisions"] if d["owner_state"] == "open"]}


def ready_nodes(plan):
    nodes = validate_state(plan)
    children = {n["parent"] for n in nodes.values()}
    ready = []
    for node in nodes.values():
        if node["id"] in children or node["state"] not in {"planned", "ready", "active"}:
            continue
        dependencies = effective_dependencies(nodes, node["id"])
        if all(nodes[d]["state"] in TERMINAL for d in dependencies):
            ready.append(node)
    return sorted(ready, key=lambda n: (n["order"], n["id"]))


def resolved_contract_units(manifest, selected):
    ledger = tomllib.loads((HOME / "promotion.toml").read_text(encoding="utf-8"))
    _, states, clauses = source_contracts(manifest, ledger)
    resolved = []
    for unit in selected:
        if states[unit["id"]]["state"] == "active":
            resolved.append({"id": unit["id"], "state": "active", "path": unit["path"], "anchors": unit["anchors"]})
        else:
            resolved.append({"id": unit["id"], "state": "retired",
                             "permanent_targets": [t for a in unit["anchors"] for t in clauses[a]["targets"]]})
    return resolved


def task_packet(manifest, task_id, context):
    tasks = load_tasks(manifest)
    if task_id in tasks:
        result = dict(tasks[task_id])
        parent = package_ancestor(manifest, task_id)
        supplemental = parent in manifest.get("supplemental_work_packages", [])
        result["supplemental_parent" if supplemental else "baseline_parent"] = parent
        owner = parent["owner"] if supplemental else parent["milestone"]
        result["contract_units"] = resolved_contract_units(manifest, [u for u in manifest["units"] if u["owner"] == owner])
        result["standing_contract"] = "vibevm/vibespecs/terraforms/NEXT-IMPLEMENTATION-CAMPAIGN.xml"
        if task_id in manifest.get("dispatch_bindings", {}):
            result["dispatch_binding_required"] = manifest["dispatch_bindings"][task_id]
        result["execution_rules"] = [
            "Central reads complete local plan; worker uses an explicitly commissioned quiet packet and exact files only.",
            "Promote the applicable product law before code/tests cite it; never cite a temporary campaign anchor from permanent code.",
            "Future paths/types/tests in notes are proposed; missing prerequisites or changed baseline require refinement, not invented proof.",
            "Select the compile target before listing/filtering tests; bind affected proof before dispatch and refresh it against the actual diff.",
            "Only actual reviewed evidence moves candidate to accepted; this tool never updates acceptance."]
    else:
        plan = load_plan(context)
        result = next((n for n in plan["node"] if n["id"] == task_id), None)
        need(result is not None, f"unknown task: {task_id}")
        result = dict(result)
    if manifest.get("preview_control"):
        control = preview_control(manifest)
        units = [unit for unit in manifest["units"] if unit["id"] == control["contract_unit"]]
        need(len(units) == 1, "preview contract unit missing/ambiguous")
        result["change_control"] = {
            "from_release": control["from_release"], "to_release": control["to_release"],
            "protocol": resolved_contract_units(manifest, units)[0],
            "permanent_homes": control["permanent_homes"],
            "bootstrap_gate": control["bootstrap_gate"], "final_gate": control["final_gate"],
            "documentation_input_gate": control["documentation"]["gate"],
            "post_removal_gate": control["post_removal_gate"],
            "task_rules": control["task_rules"],
            "status_note": "These are planned obligations. Actual captures, change records and documentation evidence belong to the permanent transition index; this packet verifies none of them."}
    policy = verification_policy(manifest)
    result["verification"] = {
        "state": "requires-binding", "policy": manifest["verification_policy"],
        "policy_spec": policy["policy_spec"], "rules": policy["rules"],
        "full_panel_gate": policy["full_panel_gate"],
        "mode": "full-product" if task_id == policy["full_panel_gate"] else policy["ordinary_gate_mode"],
        "binding_fields": ["affected_claims_and_contract_consumers", "package_and_compile_target",
                           "existing_regressions_and_new_cases", "test_names_or_reviewed_filter",
                           "exact_commands_and_flags", "relevant_input_closure",
                           "fixtures_toolchain_platform_environment", "expected_nonzero_coverage",
                           "build_link_and_run_cost", "reuse_and_escalation_conditions"],
        "bindings_executed": False,
        "status_note": "The coordinator resolves choices, available commands, expected coverage and cost before execution. This packet is a planned obligation; it neither selects real tests nor proves a run."}
    result["ready"] = task_id in {n["id"] for n in ready_nodes(load_plan(context))}
    result["status_note"] = "Readiness is a dependency check, not authorization or verification of physical inputs."
    return result


def retirement_check(manifest, context, unit_id):
    need(context is not None, "retirement requires the actual contributor context, not the seed")
    plan = load_plan(context)
    nodes = validate_coverage(manifest, load_tasks(manifest), plan)
    ledger = tomllib.loads((HOME / "promotion.toml").read_text(encoding="utf-8"))
    units, _, clauses = source_contracts(manifest, ledger)
    selected = [units[unit_id]] if unit_id else list(units.values())
    for unit in selected:
        for anchor in unit["anchors"]:
            validate_promotion(clauses[anchor], manifest)
        for parent in unit.get("retire_after", [unit["owner"]]):
            need(nodes[parent]["state"] == "accepted", f"owning workstream still open: {parent}")
    tracked = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
                             cwd=ROOT, check=True, capture_output=True).stdout.split(b"\0")
    needles = [u["path"].encode() for u in selected] + [u["id"].encode() for u in selected]
    hits = []
    for raw in tracked:
        if not raw:
            continue
        rel = raw.decode("utf-8")
        if temporary(rel, manifest):
            continue
        path = within(rel)
        if path.is_file():
            payload = path.read_bytes()
            if any(n in payload for n in needles):
                hits.append(rel)
    need(not hits, "permanent inputs reference temporary units: " + ", ".join(hits))
    return {"ok": True, "structural_retirement_eligibility": True, "unit_count": len(selected),
            "deleted_files": [], "warning": "No deletion or semantic acceptance performed. Run permanent gates after explicit removal; final proof must not depend on this tool."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["check", "frontier", "task", "retirement-check"])
    parser.add_argument("task_id", nargs="?")
    parser.add_argument("--context", type=Path)
    parser.add_argument("--unit")
    args = parser.parse_args()
    try:
        manifest = read_json(HOME / "manifest.json")
        if args.command != "check":
            validate_coverage(manifest, load_tasks(manifest), load_plan(args.context))
        if args.command == "check":
            result = check(manifest, args.context)
        elif args.command == "frontier":
            result = {"ok": True, "ready": [{"id": n["id"], "title": n["title"]} for n in ready_nodes(load_plan(args.context))]}
        elif args.command == "task":
            need(args.task_id, "task requires an ID")
            result = task_packet(manifest, args.task_id, args.context)
        else:
            result = retirement_check(manifest, args.context, args.unit)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0
    except (Refusal, OSError, ValueError, KeyError, ET.ParseError) as error:
        print(json.dumps({"ok": False, "reason": str(error)}, ensure_ascii=False))
        return 2


if __name__ == "__main__":
    sys.exit(main())
