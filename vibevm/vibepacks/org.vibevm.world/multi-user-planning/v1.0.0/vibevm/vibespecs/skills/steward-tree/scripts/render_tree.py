#!/usr/bin/env python3
"""Render any schema-1 stewardship plan as a static, offline plan tree.

The viewer is derived navigation over `plan.toml`, never a second plan: it
writes only inside `--out`, reads the plan without changing it, and reports the
protocol validator's own verdict so a picture can never contradict the tools.
Display buckets and per-parent counting follow
`plan-hierarchy.xml#TREE-DISPLAY-BUCKETS` / `#TREE-LEAF-COUNTING`.
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import runpy
import sys
import tempfile
import time
import tomllib
from typing import Any


SCHEMA = 1
GENERATOR = "steward-tree/1"
# The package's own graph validator: it layers the acceptance and completion
# rules on top of the goal renderer's structural checks, so the tree reports the
# same verdict the frontier and goal tools act on.
VALIDATOR = Path(__file__).resolve().parents[2] / "steward-parallel/scripts/parallel_frontier.py"

# One bucket per node state; `accepted` and `candidate` never merge, because
# implemented and accepted are different facts.
BUCKETS = (
    "accepted",
    "candidate",
    "active",
    "ready",
    "planned",
    "blocked",
    "deferred",
    "closed",
    "unknown",
)
STATE_BUCKET = {
    "accepted": "accepted",
    "candidate": "candidate",
    "active": "active",
    "ready": "ready",
    "planned": "planned",
    "blocked": "blocked",
    "deferred": "deferred",
    "dropped": "closed",
    "superseded": "closed",
}
CLOSED_BUCKETS = {"accepted", "closed", "deferred"}
KNOWN_KINDS = (
    "portfolio",
    "campaign",
    "phase",
    "workstream",
    "group",
    "atom",
    "gate",
    "horizon",
)
# Keep the dependency surface plural: a plan may later carry another edge kind
# without reopening the renderer or the page.
EDGE_KINDS = ("depends_on",)
NODE_FIELDS = (
    "id",
    "parent",
    "order",
    "kind",
    "title",
    "state",
    "zoom",
    "mandates",
    "acceptance",
    "evidence",
) + EDGE_KINDS
PLAN_FIELDS = (
    "schema",
    "plan_id",
    "revision",
    "root_node",
    "current_node",
    "updated_at",
    "lang",
    "mandate",
    "node",
)
LANGS = ("ru", "en")

STRINGS: dict[str, dict[str, str]] = {
    "ru": {
        "app.title": "Дерево плана",
        "bucket.accepted": "Готово · принято",
        "bucket.candidate": "Сделано · ждёт приёмки",
        "bucket.active": "В работе",
        "bucket.ready": "Можно начинать",
        "bucket.planned": "Запланировано",
        "bucket.blocked": "Заблокировано",
        "bucket.deferred": "Отложено",
        "bucket.closed": "Закрыто без выполнения",
        "bucket.unknown": "Неизвестное состояние",
        "kind.portfolio": "портфель",
        "kind.campaign": "кампания",
        "kind.phase": "фаза",
        "kind.workstream": "поток",
        "kind.group": "группа",
        "kind.atom": "атом",
        "kind.gate": "гейт",
        "kind.horizon": "горизонт",
        "ui.search": "Поиск по id или названию",
        "ui.searchLabel": "Поиск",
        "ui.expandAll": "Развернуть всё",
        "ui.collapseAll": "Свернуть всё",
        "ui.jumpCurrent": "К текущему узлу",
        "ui.theme": "Тема",
        "ui.lang": "Язык",
        "ui.detail": "Узел",
        "ui.nothingSelected": "Выберите узел в дереве.",
        "ui.acceptance": "Критерии приёмки",
        "ui.evidence": "Доказательства",
        "ui.mandates": "Мандаты",
        "ui.dependsOn": "Зависит от",
        "ui.dependents": "Зависят от него",
        "ui.extra": "Дополнительные поля",
        "ui.children": "Дочерние узлы",
        "ui.leaves": "листьев",
        "ui.of": "из",
        "ui.none": "нет",
        "ui.plan": "План",
        "ui.revision": "ревизия",
        "ui.nodes": "узлов",
        "ui.validationOk": "План проходит валидатор протокола.",
        "ui.validationFail": "План не проходит валидатор протокола",
        "ui.validationUnavailable": "Валидатор протокола недоступен",
        "ui.warnings": "Предупреждения",
        "ui.current": "текущий",
        "ui.matches": "совпадений",
        "ui.noMatches": "Ничего не найдено.",
        "ui.path": "Путь",
        "ui.order": "порядок",
        "ui.zoom": "зум",
        "ui.state": "состояние",
        "ui.showClosed": "Показывать закрытые",
        "ui.counts": "Листья по состояниям",
        "ui.sha": "sha256 плана",
    },
    "en": {
        "app.title": "Plan tree",
        "bucket.accepted": "Accepted",
        "bucket.candidate": "Done · awaiting acceptance",
        "bucket.active": "In progress",
        "bucket.ready": "Can start",
        "bucket.planned": "Planned",
        "bucket.blocked": "Blocked",
        "bucket.deferred": "Deferred",
        "bucket.closed": "Closed unfinished",
        "bucket.unknown": "Unknown state",
        "kind.portfolio": "portfolio",
        "kind.campaign": "campaign",
        "kind.phase": "phase",
        "kind.workstream": "workstream",
        "kind.group": "group",
        "kind.atom": "atom",
        "kind.gate": "gate",
        "kind.horizon": "horizon",
        "ui.search": "Search by id or title",
        "ui.searchLabel": "Search",
        "ui.expandAll": "Expand all",
        "ui.collapseAll": "Collapse all",
        "ui.jumpCurrent": "Go to current node",
        "ui.theme": "Theme",
        "ui.lang": "Language",
        "ui.detail": "Node",
        "ui.nothingSelected": "Select a node in the tree.",
        "ui.acceptance": "Acceptance",
        "ui.evidence": "Evidence",
        "ui.mandates": "Mandates",
        "ui.dependsOn": "Depends on",
        "ui.dependents": "Dependents",
        "ui.extra": "Extra fields",
        "ui.children": "Children",
        "ui.leaves": "leaves",
        "ui.of": "of",
        "ui.none": "none",
        "ui.plan": "Plan",
        "ui.revision": "revision",
        "ui.nodes": "nodes",
        "ui.validationOk": "The plan passes the protocol validator.",
        "ui.validationFail": "The plan fails the protocol validator",
        "ui.validationUnavailable": "The protocol validator is unavailable",
        "ui.warnings": "Warnings",
        "ui.current": "current",
        "ui.matches": "matches",
        "ui.noMatches": "Nothing found.",
        "ui.path": "Path",
        "ui.order": "order",
        "ui.zoom": "zoom",
        "ui.state": "state",
        "ui.showClosed": "Show closed",
        "ui.counts": "Leaves by state",
        "ui.sha": "plan sha256",
    },
}

WARNING_TEXT = {
    "NODE_NOT_A_TABLE": "plan.node entry {where} is not a table and was skipped",
    "NODE_ID_INVALID": "plan.node entry {where} has no usable string id and was skipped",
    "NODE_ID_DUPLICATE": "duplicate node id {where}; only the first entry is shown",
    "NODE_PARENT_MISSING": "{where} names a parent that the plan does not define; shown as a root",
    "NODE_PARENT_CYCLE": "{where} sits on a parent cycle; shown as a root",
    "NODE_STATE_UNKNOWN": "{where} has a state outside the protocol vocabulary",
    "NODE_KIND_UNKNOWN": "{where} has a kind outside the protocol vocabulary",
    "NODE_FIELD_TYPE": "{where} has a field of an unexpected type and was normalised",
    "EDGE_TARGET_MISSING": "{where} names a dependency the plan does not define",
    "PLAN_ROOT_UNKNOWN": "root_node {where} is not a node in this plan",
    "PLAN_MULTIPLE_ROOTS": "the plan has {where} root nodes; a conformant plan has one",
    "PLAN_CURRENT_UNKNOWN": "current_node {where} is not a node in this plan",
    "PLAN_NO_NODES": "the plan defines no usable nodes",
    "MANDATE_INVALID": "mandate entry {where} is not a usable table and was skipped",
}


class TreeError(RuntimeError):
    """A typed refusal: bad arguments, unreadable plan or unwritable output."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def jsonable(value: Any) -> Any:
    """Make any TOML value renderable without inventing a type for the page."""
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, (datetime.datetime, datetime.date, datetime.time)):
        return value.isoformat()
    if isinstance(value, list):
        return [jsonable(item) for item in value]
    if isinstance(value, dict):
        return {str(key): jsonable(item) for key, item in value.items()}
    return str(value)


def text_of(value: Any) -> str:
    return value if isinstance(value, str) else ""


def str_list(value: Any) -> tuple[list[str], bool]:
    if isinstance(value, list):
        clean = [item for item in value if isinstance(item, str)]
        return clean, len(clean) != len(value)
    return [], value is not None


def protocol_verdict(plan: dict[str, Any]) -> dict[str, Any]:
    """Ask the protocol's own validator, so the tree cannot outrank the tools."""
    try:
        module = runpy.run_path(str(VALIDATOR))
    except (OSError, ValueError, RuntimeError, SyntaxError) as exc:
        return {"status": "unavailable", "code": "VALIDATOR_UNAVAILABLE", "message": str(exc)}
    try:
        module["validate_plan"](plan)
    except (module["FrontierError"], ValueError) as exc:
        return {"status": "invalid", "code": getattr(exc, "code", "PLAN_INVALID"),
                "message": str(exc)}
    except (TypeError, AttributeError, KeyError, RecursionError) as exc:
        return {"status": "invalid", "code": "PLAN_INVALID",
                "message": f"{type(exc).__name__}: {exc}"}
    return {"status": "valid", "code": None, "message": None}


def normalize(plan: dict[str, Any]) -> dict[str, Any]:
    """Build a display model that survives a plan the validator would reject.

    Nothing here is authority: unknown states, orphans and cycles become visible
    warnings instead of a silent fallback readiness.
    """
    warnings: list[dict[str, Any]] = []

    def warn(code: str, where: Any, node: str | None = None) -> None:
        warnings.append({
            "code": code,
            "node": node,
            "message": WARNING_TEXT[code].format(where=where),
        })

    rows = plan.get("node")
    rows = rows if isinstance(rows, list) else []
    nodes: dict[str, dict[str, Any]] = {}
    order_seen: list[str] = []
    for index, row in enumerate(rows):
        if not isinstance(row, dict):
            warn("NODE_NOT_A_TABLE", f"#{index}")
            continue
        node_id = row.get("id")
        if not isinstance(node_id, str) or not node_id.strip():
            warn("NODE_ID_INVALID", f"#{index}")
            continue
        if node_id in nodes:
            warn("NODE_ID_DUPLICATE", node_id, node_id)
            continue
        state = text_of(row.get("state"))
        kind = text_of(row.get("kind"))
        bucket = STATE_BUCKET.get(state, "unknown")
        if bucket == "unknown":
            warn("NODE_STATE_UNKNOWN", node_id, node_id)
        if kind not in KNOWN_KINDS:
            warn("NODE_KIND_UNKNOWN", node_id, node_id)
        sloppy = False
        lists: dict[str, list[str]] = {}
        for field in ("mandates", "acceptance", "evidence") + EDGE_KINDS:
            lists[field], bad = str_list(row.get(field))
            sloppy = sloppy or bad
        raw_order = row.get("order")
        order = raw_order if isinstance(raw_order, int) and not isinstance(raw_order, bool) else 0
        sloppy = sloppy or order != raw_order
        if sloppy:
            warn("NODE_FIELD_TYPE", node_id, node_id)
        nodes[node_id] = {
            "id": node_id,
            "parent": text_of(row.get("parent")),
            "order": order,
            "kind": kind,
            "title": text_of(row.get("title")) or node_id,
            "state": state,
            "bucket": bucket,
            "zoom": text_of(row.get("zoom")),
            "mandates": lists["mandates"],
            "acceptance": lists["acceptance"],
            "evidence": lists["evidence"],
            "edges": [{"kind": kind_name, "to": target}
                      for kind_name in EDGE_KINDS for target in lists[kind_name]],
            "extra": {key: jsonable(value) for key, value in sorted(row.items())
                      if key not in NODE_FIELDS},
        }
        order_seen.append(node_id)

    if not nodes:
        warn("PLAN_NO_NODES", "")

    for node in nodes.values():
        if node["parent"] and node["parent"] not in nodes:
            warn("NODE_PARENT_MISSING", node["id"], node["id"])
            node["parent"] = ""
    for node in nodes.values():
        seen = {node["id"]}
        cursor = node["parent"]
        while cursor:
            if cursor in seen:
                warn("NODE_PARENT_CYCLE", node["id"], node["id"])
                node["parent"] = ""
                break
            seen.add(cursor)
            cursor = nodes[cursor]["parent"]
    for node in nodes.values():
        for edge in node["edges"]:
            if edge["to"] not in nodes:
                warn("EDGE_TARGET_MISSING", f"{node['id']} -> {edge['to']}", node["id"])

    children: dict[str, list[str]] = {key: [] for key in nodes}
    roots = [key for key in order_seen if not nodes[key]["parent"]]
    for key in order_seen:
        parent = nodes[key]["parent"]
        if parent:
            children[parent].append(key)
    for rows_of in children.values():
        rows_of.sort(key=lambda key: (nodes[key]["order"], key))
    roots.sort(key=lambda key: (nodes[key]["order"], key))

    declared_root = text_of(plan.get("root_node"))
    if declared_root and declared_root not in nodes:
        warn("PLAN_ROOT_UNKNOWN", declared_root)
    elif declared_root in roots:
        roots = [declared_root] + [key for key in roots if key != declared_root]
    if len(roots) > 1:
        warn("PLAN_MULTIPLE_ROOTS", len(roots))
    current = text_of(plan.get("current_node"))
    if current and current not in nodes:
        warn("PLAN_CURRENT_UNKNOWN", current)
        current = ""

    mandates: list[dict[str, Any]] = []
    for index, row in enumerate(plan.get("mandate") if isinstance(plan.get("mandate"), list) else []):
        if not isinstance(row, dict) or not isinstance(row.get("id"), str):
            warn("MANDATE_INVALID", f"#{index}")
            continue
        nodes_of, _ = str_list(row.get("nodes"))
        mandates.append({
            "id": row["id"],
            "text": text_of(row.get("text")),
            "source": text_of(row.get("source")),
            "disposition": text_of(row.get("disposition")),
            "nodes": nodes_of,
        })

    return {"nodes": nodes, "children": children, "roots": roots,
            "current": current, "mandates": mandates, "warnings": warnings}


def count_leaves(model: dict[str, Any]) -> None:
    """Count every parent over its leaf descendants only — see TREE-LEAF-COUNTING.

    A leaf is a node with no children and counts once, in the bucket of its own
    state. No node is counted twice and no parent is counted in its own totals,
    so a parent's buckets always sum to its leaf-descendant total.
    """
    nodes, children = model["nodes"], model["children"]

    def walk(key: str) -> dict[str, int]:
        node = nodes[key]
        if not children[key]:
            counts = {node["bucket"]: 1}
        else:
            counts = {}
            for child in children[key]:
                for bucket, value in walk(child).items():
                    counts[bucket] = counts.get(bucket, 0) + value
        node["counts"] = {bucket: counts[bucket] for bucket in BUCKETS if counts.get(bucket)}
        node["leaves"] = sum(counts.values())
        return counts

    for root in model["roots"]:
        walk(root)
    detached = [key for key in nodes if "counts" not in nodes[key]]
    for key in detached:  # unreachable only if a cycle survived normalisation
        nodes[key]["counts"] = {nodes[key]["bucket"]: 1}
        nodes[key]["leaves"] = 1


def initially_open(node: dict[str, Any], children: list[str], nodes: dict[str, Any]) -> bool:
    """Open the live part of the plan; keep summarised and finished parts folded."""
    if not children:
        return False
    if node["zoom"] in {"summary", "collapsed"}:
        return False
    return any(bucket not in CLOSED_BUCKETS for bucket in node["counts"])


def build_payload(plan: dict[str, Any], raw: bytes, source_name: str, lang: str) -> dict[str, Any]:
    model = normalize(plan)
    count_leaves(model)
    nodes, children = model["nodes"], model["children"]

    order: list[str] = []

    def walk(key: str, depth: int, path: list[str]) -> None:
        node = nodes[key]
        node["depth"] = depth
        node["path"] = list(path)
        node["children"] = children[key]
        node["open"] = initially_open(node, children[key], nodes)
        order.append(key)
        for child in children[key]:
            walk(child, depth + 1, path + [key])

    for root in model["roots"]:
        walk(root, 0, [])
    for key in sorted(nodes):
        if key not in set(order):
            nodes[key].update(depth=0, path=[], children=children[key], open=False)
            order.append(key)

    totals: dict[str, int] = {}
    for root in model["roots"]:
        for bucket, value in nodes[root]["counts"].items():
            totals[bucket] = totals.get(bucket, 0) + value

    return {
        "schema": SCHEMA,
        "generator": GENERATOR,
        "lang": lang,
        "langs": list(LANGS),
        "strings": STRINGS,
        "buckets": list(BUCKETS),
        "edge_kinds": list(EDGE_KINDS),
        "plan": {
            "source_name": source_name,
            "sha256": hashlib.sha256(raw).hexdigest(),
            "schema": jsonable(plan.get("schema")),
            "plan_id": text_of(plan.get("plan_id")),
            "revision": jsonable(plan.get("revision")),
            "updated_at": jsonable(plan.get("updated_at")),
            "root_node": text_of(plan.get("root_node")),
            "current_node": model["current"],
            "node_count": len(nodes),
            "extra": {key: jsonable(value) for key, value in sorted(plan.items())
                      if key not in PLAN_FIELDS},
        },
        "validation": protocol_verdict(plan),
        "warnings": model["warnings"],
        "totals": {bucket: totals[bucket] for bucket in BUCKETS if totals.get(bucket)},
        "roots": model["roots"],
        "order": order,
        "nodes": [nodes[key] for key in order],
        "mandates": model["mandates"],
    }


def embed(payload: dict[str, Any]) -> str:
    """Serialise for a `<script type="application/json">` block, inertly."""
    text = json.dumps(payload, ensure_ascii=True, sort_keys=True, separators=(",", ":"))
    return text.replace("<", "\\u003c").replace(">", "\\u003e").replace("&", "\\u0026")


def render_html(payload: dict[str, Any]) -> bytes:
    page = PAGE.replace("@@LANG@@", payload["lang"])
    page = page.replace("@@TITLE@@", STRINGS[payload["lang"]]["app.title"])
    page = page.replace("@@DATA@@", embed(payload))
    return page.encode("utf-8")


def publish(out: Path, name: str, data: bytes) -> bool:
    """Replace one file inside `out` atomically; never touch anything else."""
    path = out / name
    try:
        if path.exists() and path.read_bytes() == data:
            return False
    except OSError as exc:
        raise TreeError("OUTPUT_UNWRITABLE", f"cannot inspect {path}: {exc}") from exc
    try:
        handle, temporary = tempfile.mkstemp(prefix=f".{name}.new-", dir=out)
        temp_path = Path(temporary)
        try:
            with os.fdopen(handle, "wb") as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temp_path, path)
        finally:
            temp_path.unlink(missing_ok=True)
    except OSError as exc:
        raise TreeError("OUTPUT_UNWRITABLE", f"cannot write {path}: {exc}") from exc
    return True


def read_plan(path: Path) -> tuple[bytes, dict[str, Any]]:
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise TreeError("PLAN_UNREADABLE", f"cannot read {path}: {exc}") from exc
    try:
        plan = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as exc:
        plan = {}
        return raw, {"__parse_error__": f"{exc}"}
    return raw, plan


def resolve_lang(requested: str | None, plan: dict[str, Any], plan_path: Path) -> str:
    """`--lang`, then the plan, then a sibling `settings.toml`, then `ru`."""
    for candidate in (requested, plan.get("lang")):
        if isinstance(candidate, str) and candidate in STRINGS:
            return candidate
    settings = plan_path.parent / "settings.toml"
    try:
        value = tomllib.loads(settings.read_bytes().decode("utf-8")).get("lang")
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError):
        value = None
    return value if isinstance(value, str) and value in STRINGS else LANGS[0]


def render_once(plan_path: Path, out: Path, lang: str | None) -> dict[str, Any]:
    try:
        out.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        raise TreeError("OUTPUT_UNWRITABLE", f"cannot create {out}: {exc}") from exc
    raw, plan = read_plan(plan_path)
    parse_error = plan.pop("__parse_error__", None)
    chosen = resolve_lang(lang, plan, plan_path)
    payload = build_payload(plan, raw, plan_path.name, chosen)
    if parse_error is not None:
        payload["validation"] = {"status": "invalid", "code": "PLAN_UNPARSABLE",
                                 "message": f"invalid UTF-8 TOML: {parse_error}"}
    html_changed = publish(out, "index.html", render_html(payload))
    data_changed = publish(
        out, "plan-tree.json",
        (json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=1) + "\n").encode("utf-8"),
    )
    return {
        "ok": payload["validation"]["status"] == "valid",
        "validation": payload["validation"],
        "warnings": len(payload["warnings"]),
        "plan_id": payload["plan"]["plan_id"],
        "plan_revision": payload["plan"]["revision"],
        "plan_sha256": payload["plan"]["sha256"],
        "nodes": payload["plan"]["node_count"],
        "lang": chosen,
        "totals": payload["totals"],
        "index": str(out / "index.html"),
        "data": str(out / "plan-tree.json"),
        "changed": html_changed or data_changed,
    }


def watch(plan_path: Path, out: Path, lang: str | None, interval: float) -> int:
    """Re-render on changed plan bytes; the plan itself is never written."""
    previous: str | None = None
    while True:
        try:
            digest = hashlib.sha256(plan_path.read_bytes()).hexdigest()
        except OSError:
            digest = None
        if digest is not None and digest != previous:
            try:
                report = render_once(plan_path, out, lang)
            except TreeError as exc:
                print(json.dumps({"ok": False, "code": exc.code, "message": str(exc)}), flush=True)
                return 2
            previous = digest
            print(json.dumps({"event": "rendered", **report}, ensure_ascii=False), flush=True)
        try:
            time.sleep(interval)
        except KeyboardInterrupt:
            return 0


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--plan", type=Path, required=True, help="path to a schema-1 plan.toml")
    result.add_argument("--out", type=Path, required=True, help="output directory; nothing outside it is written")
    result.add_argument("--lang", choices=LANGS, help="initial UI locale; default: plan/settings, else ru")
    result.add_argument("--watch", action="store_true", help="re-render while the plan bytes change")
    result.add_argument("--interval", type=float, default=2.0, help="--watch poll seconds (default 2)")
    return result


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        out = args.out
        out.mkdir(parents=True, exist_ok=True)
        if not out.is_dir():
            raise TreeError("OUTPUT_UNWRITABLE", f"not a directory: {out}")
        if args.interval <= 0:
            raise TreeError("ARGUMENTS_INVALID", "--interval must be positive")
        if args.watch:
            return watch(args.plan, out, args.lang, args.interval)
        report = render_once(args.plan, out, args.lang)
    except OSError as exc:
        print(json.dumps({"ok": False, "code": "OUTPUT_UNWRITABLE", "message": str(exc)}))
        return 2
    except TreeError as exc:
        print(json.dumps({"ok": False, "code": exc.code, "message": str(exc)}))
        return 2
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0 if report["ok"] else 3


PAGE = r"""<!doctype html>
<html lang="@@LANG@@" data-theme="auto">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>@@TITLE@@</title>
<style>
:root{
  --bg:#ffffff; --panel:#f7f8fa; --fg:#15181d; --muted:#5d6672; --line:#dfe3e9;
  --accent:#1d4ed8; --hit:#fff3bf; --radius:8px;
  --b-accepted:#15803d; --b-candidate:#b45309; --b-active:#1d4ed8; --b-ready:#0e7490;
  --b-planned:#6b7280; --b-blocked:#b91c1c; --b-deferred:#7c3aed; --b-closed:#9aa3ae;
  --b-unknown:#be185d;
}
@media (prefers-color-scheme:dark){
  :root:not([data-theme="light"]){
    --bg:#15181d; --panel:#1d2127; --fg:#e8ecf1; --muted:#9aa4b2; --line:#2c323b;
    --accent:#7da2ff; --hit:#4a3f12;
    --b-accepted:#4ade80; --b-candidate:#fbbf24; --b-active:#7da2ff; --b-ready:#3dd6e8;
    --b-planned:#9aa4b2; --b-blocked:#f87171; --b-deferred:#c4b5fd; --b-closed:#6b7482;
    --b-unknown:#f472b6;
  }
}
:root[data-theme="dark"]{
  --bg:#15181d; --panel:#1d2127; --fg:#e8ecf1; --muted:#9aa4b2; --line:#2c323b;
  --accent:#7da2ff; --hit:#4a3f12;
  --b-accepted:#4ade80; --b-candidate:#fbbf24; --b-active:#7da2ff; --b-ready:#3dd6e8;
  --b-planned:#9aa4b2; --b-blocked:#f87171; --b-deferred:#c4b5fd; --b-closed:#6b7482;
  --b-unknown:#f472b6;
}
*{box-sizing:border-box}
html,body{margin:0;padding:0}
body{background:var(--bg);color:var(--fg);font:15px/1.45 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
header{position:sticky;top:0;z-index:5;background:var(--bg);border-bottom:1px solid var(--line);padding:10px 16px}
.bar{display:flex;flex-wrap:wrap;gap:8px;align-items:center}
h1{font-size:16px;margin:0 8px 0 0;font-weight:650}
.meta{color:var(--muted);font-size:12.5px;display:flex;flex-wrap:wrap;gap:4px 12px}
.grow{flex:1 1 180px;min-width:140px}
button,input{font:inherit;color:inherit;background:var(--panel);border:1px solid var(--line);
  border-radius:var(--radius);padding:5px 9px}
button{cursor:pointer}
button:hover{border-color:var(--accent)}
button[aria-pressed="true"]{border-color:var(--accent);color:var(--accent)}
input{width:100%}
:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.banner{margin:10px 16px 0;padding:9px 12px;border:1px solid var(--line);border-left-width:4px;
  border-radius:var(--radius);background:var(--panel);font-size:13.5px}
.banner.ok{border-left-color:var(--b-accepted)}
.banner.bad{border-left-color:var(--b-blocked)}
.banner.warn{border-left-color:var(--b-candidate)}
.banner code{word-break:break-word}
.banner ul{margin:6px 0 0;padding-left:20px}
.totals{display:flex;flex-wrap:wrap;gap:6px;margin:10px 16px 0}
.legend{display:inline-flex;align-items:center;gap:6px;padding:3px 9px;border:1px solid var(--line);
  border-radius:999px;font-size:12.5px;background:var(--panel)}
main{display:grid;grid-template-columns:minmax(0,1fr) minmax(280px,22rem);gap:16px;padding:12px 16px 32px;align-items:start}
@media (max-width:900px){main{grid-template-columns:minmax(0,1fr)}}
.tree{min-width:0}
.row{display:flex;align-items:baseline;gap:7px;padding:3px 6px 3px calc(6px + var(--depth) * 15px);
  border-radius:6px;cursor:default;scroll-margin:70px}
.row[hidden]{display:none}
.row:hover{background:var(--panel)}
.row[aria-selected="true"]{background:var(--panel);box-shadow:inset 0 0 0 1px var(--accent)}
.row.match .title{background:var(--hit);border-radius:3px}
.twisty{flex:0 0 18px;width:18px;padding:0;border:0;background:none;color:var(--muted);text-align:center;line-height:1.3}
.twisty[hidden]{visibility:hidden;display:inline-block}
.dot{flex:0 0 9px;width:9px;height:9px;border-radius:50%;background:var(--bucket);align-self:center}
.title{flex:0 1 auto;min-width:0;overflow-wrap:anywhere;font-weight:450}
.row.parent .title{font-weight:600}
.nid{color:var(--muted);font:12px/1.4 ui-monospace,SFMono-Regular,Menlo,monospace;flex:0 0 auto}
.kind{color:var(--muted);font-size:11.5px;text-transform:lowercase;flex:0 0 auto}
.state{color:var(--bucket);font-size:12px;flex:0 0 auto;white-space:nowrap}
.cur{color:var(--b-active);border:1px solid currentColor;border-radius:999px;font-size:10.5px;padding:0 5px;flex:0 0 auto}
.chips{display:flex;gap:4px;margin-left:auto;flex:0 0 auto}
.chip{font-size:11px;color:var(--bg);background:var(--bucket);border-radius:999px;padding:0 6px;min-width:18px;text-align:center}
@media (max-width:620px){
  .chips{display:none}
  .kind{display:none}
}
aside{position:sticky;top:64px;border:1px solid var(--line);border-radius:var(--radius);
  background:var(--panel);padding:12px;min-width:0}
@media (max-width:900px){aside{position:static}}
aside h2{font-size:14px;margin:0 0 8px}
aside h3{font-size:12px;margin:12px 0 4px;color:var(--muted);text-transform:uppercase;letter-spacing:.04em}
aside ul{margin:0;padding-left:18px}
aside li{overflow-wrap:anywhere;margin:2px 0}
aside dl{margin:0;display:grid;grid-template-columns:auto minmax(0,1fr);gap:2px 8px;font-size:13px}
aside dt{color:var(--muted)}
aside dd{margin:0;overflow-wrap:anywhere}
.link{background:none;border:0;padding:0;color:var(--accent);text-decoration:underline;cursor:pointer;
  font:12px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace}
.muted{color:var(--muted)}
.pre{white-space:pre-wrap;font:12px/1.45 ui-monospace,SFMono-Regular,Menlo,monospace;margin:0}
.sr{position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%)}
</style>
</head>
<body>
<header>
  <div class="bar">
    <h1 id="app-title">@@TITLE@@</h1>
    <span class="grow"><label class="sr" for="q" id="q-label"></label><input id="q" type="search" autocomplete="off"></span>
    <button id="expand" type="button"></button>
    <button id="collapse" type="button"></button>
    <button id="current" type="button"></button>
    <button id="theme" type="button"></button>
    <button id="lang" type="button"></button>
  </div>
  <div class="meta" id="meta"></div>
</header>
<div id="banner"></div>
<div class="totals" id="totals"></div>
<main>
  <div class="tree" id="tree" role="tree" aria-labelledby="app-title"></div>
  <aside id="detail" aria-live="polite"></aside>
</main>
<script type="application/json" id="plan-data">@@DATA@@</script>
<script>
(function(){
  "use strict";
  var DATA = JSON.parse(document.getElementById("plan-data").textContent);
  var NODES = {}, ROWS = {}, DEPENDENTS = {};
  DATA.nodes.forEach(function(n){ NODES[n.id] = n; DEPENDENTS[n.id] = []; });
  DATA.nodes.forEach(function(n){
    (n.edges || []).forEach(function(e){
      if (DEPENDENTS[e.to]) DEPENDENTS[e.to].push({kind: e.kind, from: n.id});
    });
  });

  var lang = DATA.lang, theme = "auto", selected = null, query = "";
  var open = {};
  DATA.nodes.forEach(function(n){ open[n.id] = !!n.open; });

  function store(key, value){ try{ localStorage.setItem("steward-tree:" + key, value); }catch(e){} }
  function load(key){ try{ return localStorage.getItem("steward-tree:" + key); }catch(e){ return null; } }
  var savedLang = load("lang"); if (DATA.langs.indexOf(savedLang) >= 0) lang = savedLang;
  var savedTheme = load("theme"); if (["auto","light","dark"].indexOf(savedTheme) >= 0) theme = savedTheme;

  function t(key){ var table = DATA.strings[lang] || {}; return table[key] || key; }
  function bucketLabel(b){ return t("bucket." + b); }
  function kindLabel(k){ var s = t("kind." + k); return s === "kind." + k ? k : s; }
  function el(tag, cls, text){
    var node = document.createElement(tag);
    if (cls) node.className = cls;
    if (text !== undefined && text !== null) node.textContent = String(text);
    return node;
  }

  /* ---- tree rows ------------------------------------------------------- */
  var tree = document.getElementById("tree");
  function buildRows(){
    tree.textContent = "";
    DATA.nodes.forEach(function(n){
      var row = el("div", "row" + (n.children.length ? " parent" : ""));
      row.setAttribute("role", "treeitem");
      row.setAttribute("aria-level", String(n.depth + 1));
      row.setAttribute("aria-selected", "false");
      row.dataset.id = n.id;
      row.style.setProperty("--depth", String(n.depth));
      row.style.setProperty("--bucket", "var(--b-" + n.bucket + ")");
      var twisty = el("button", "twisty", "");
      twisty.type = "button";
      twisty.tabIndex = -1;
      if (!n.children.length) twisty.hidden = true;
      twisty.addEventListener("click", function(event){
        event.stopPropagation();
        open[n.id] = !open[n.id];
        paint();
      });
      row.appendChild(twisty);
      row.appendChild(el("span", "dot"));
      row.appendChild(el("span", "title", n.title));
      row.appendChild(el("span", "nid", n.id));
      row.appendChild(el("span", "kind", kindLabel(n.kind)));
      row.appendChild(el("span", "state", bucketLabel(n.bucket)));
      if (n.id === DATA.plan.current_node) row.appendChild(el("span", "cur", t("ui.current")));
      var chips = el("span", "chips");
      DATA.buckets.forEach(function(b){
        var value = (n.counts || {})[b];
        if (!value || !n.children.length) return;
        var chip = el("span", "chip", value);
        chip.style.setProperty("--bucket", "var(--b-" + b + ")");
        chip.title = bucketLabel(b);
        chips.appendChild(chip);
      });
      row.appendChild(chips);
      row.addEventListener("click", function(){ select(n.id, false); });
      ROWS[n.id] = row;
      tree.appendChild(row);
    });
  }

  function relabel(){
    DATA.nodes.forEach(function(n){
      var row = ROWS[n.id];
      row.querySelector(".kind").textContent = kindLabel(n.kind);
      row.querySelector(".state").textContent = bucketLabel(n.bucket);
      var cur = row.querySelector(".cur");
      if (cur) cur.textContent = t("ui.current");
      Array.prototype.forEach.call(row.querySelectorAll(".chip"), function(chip, index){
        chip.title = bucketLabel(visibleBuckets(n)[index]);
      });
    });
  }
  function visibleBuckets(n){
    return DATA.buckets.filter(function(b){ return (n.counts || {})[b]; });
  }

  function matches(n){
    if (!query) return false;
    return (n.id + " " + n.title).toLowerCase().indexOf(query) >= 0;
  }

  function paint(){
    var show = {}, hits = 0;
    if (query){
      DATA.nodes.forEach(function(n){
        if (!matches(n)) return;
        hits += 1;
        show[n.id] = true;
        n.path.forEach(function(id){ show[id] = true; });
      });
    } else {
      DATA.nodes.forEach(function(n){
        show[n.id] = n.path.every(function(id){ return open[id]; });
      });
    }
    DATA.nodes.forEach(function(n){
      var row = ROWS[n.id];
      row.hidden = !show[n.id];
      row.classList.toggle("match", query ? matches(n) : false);
      if (n.children.length){
        var expanded = query ? n.children.some(function(c){ return show[c]; }) : !!open[n.id];
        row.setAttribute("aria-expanded", expanded ? "true" : "false");
        row.querySelector(".twisty").textContent = expanded ? "▾" : "▸";
      }
    });
    var noHits = document.getElementById("nohits");
    if (query && hits === 0){
      if (!noHits){ noHits = el("p", "muted", t("ui.noMatches")); noHits.id = "nohits"; tree.appendChild(noHits); }
      else noHits.textContent = t("ui.noMatches");
    } else if (noHits){ noHits.remove(); }
    rove();
  }

  function visibleRows(){
    return DATA.order.map(function(id){ return ROWS[id]; }).filter(function(row){ return !row.hidden; });
  }
  function rove(){
    var rows = visibleRows();
    rows.forEach(function(row){ row.tabIndex = -1; });
    var active = selected && ROWS[selected] && !ROWS[selected].hidden ? ROWS[selected] : rows[0];
    if (active) active.tabIndex = 0;
  }

  function reveal(id){
    var node = NODES[id];
    if (!node) return;
    node.path.forEach(function(ancestor){ open[ancestor] = true; });
  }

  function select(id, scroll){
    if (!NODES[id]) return;
    if (selected && ROWS[selected]) ROWS[selected].setAttribute("aria-selected", "false");
    selected = id;
    reveal(id);
    paint();
    var row = ROWS[id];
    row.setAttribute("aria-selected", "true");
    row.tabIndex = 0;
    if (scroll) row.scrollIntoView({block: "center"});
    row.focus({preventScroll: !scroll});
    detail();
  }

  tree.addEventListener("keydown", function(event){
    var rows = visibleRows();
    var row = event.target.closest ? event.target.closest(".row") : null;
    if (!row) return;
    var index = rows.indexOf(row), id = row.dataset.id, node = NODES[id];
    var keys = {ArrowDown: 1, ArrowUp: -1};
    if (event.key in keys){
      var next = rows[index + keys[event.key]];
      if (next) select(next.dataset.id, false);
    } else if (event.key === "Home" && rows.length){ select(rows[0].dataset.id, false);
    } else if (event.key === "End" && rows.length){ select(rows[rows.length - 1].dataset.id, false);
    } else if (event.key === "ArrowRight"){
      if (node.children.length && !open[id]){ open[id] = true; paint(); }
      else if (node.children.length){ select(node.children[0], false); }
    } else if (event.key === "ArrowLeft"){
      if (node.children.length && open[id] && !query){ open[id] = false; paint(); }
      else if (node.path.length){ select(node.path[node.path.length - 1], false); }
    } else if (event.key === "Enter" || event.key === " "){ select(id, false);
    } else { return; }
    event.preventDefault();
  });

  /* ---- detail pane ----------------------------------------------------- */
  var aside = document.getElementById("detail");
  function list(parent, label, items, render){
    parent.appendChild(el("h3", null, label));
    if (!items.length){ parent.appendChild(el("p", "muted", t("ui.none"))); return; }
    var ul = el("ul");
    items.forEach(function(item){
      var li = el("li");
      render(li, item);
      ul.appendChild(li);
    });
    parent.appendChild(ul);
  }
  function linkTo(parent, id, suffix){
    var button = el("button", "link", id);
    button.type = "button";
    button.addEventListener("click", function(){ select(id, true); });
    parent.appendChild(button);
    if (suffix) parent.appendChild(el("span", null, " " + suffix));
  }

  function detail(){
    aside.textContent = "";
    if (!selected || !NODES[selected]){
      aside.appendChild(el("h2", null, t("ui.plan")));
      var dl = el("dl");
      [["ui.nodes", DATA.plan.node_count], ["ui.revision", DATA.plan.revision],
       ["ui.sha", DATA.plan.sha256]].forEach(function(pair){
        dl.appendChild(el("dt", null, t(pair[0])));
        dl.appendChild(el("dd", null, pair[1]));
      });
      aside.appendChild(dl);
      Object.keys(DATA.plan.extra || {}).forEach(function(key){
        aside.appendChild(el("h3", null, key));
        aside.appendChild(el("p", "pre", stringify(DATA.plan.extra[key])));
      });
      aside.appendChild(el("p", "muted", t("ui.nothingSelected")));
      return;
    }
    var n = NODES[selected];
    aside.appendChild(el("h2", null, n.title));
    var dl = el("dl");
    function pair(label, value){
      dl.appendChild(el("dt", null, label));
      dl.appendChild(el("dd", null, value));
    }
    pair("id", n.id);
    pair(t("ui.state"), bucketLabel(n.bucket) + " (" + (n.state || "—") + ")");
    pair("kind", kindLabel(n.kind) + (n.kind && kindLabel(n.kind) !== n.kind ? " (" + n.kind + ")" : ""));
    pair(t("ui.order"), n.order);
    if (n.zoom) pair(t("ui.zoom"), n.zoom);
    if (n.children.length) pair(t("ui.leaves"), n.leaves);
    aside.appendChild(dl);

    if (n.path.length){
      var p = el("p", "muted");
      p.appendChild(el("span", null, t("ui.path") + ": "));
      n.path.forEach(function(id, index){
        if (index) p.appendChild(el("span", null, " / "));
        linkTo(p, id, "");
      });
      aside.appendChild(p);
    }
    if (n.children.length){
      aside.appendChild(el("h3", null, t("ui.counts")));
      var box = el("p");
      visibleBuckets(n).forEach(function(b){
        var tag = el("span", "legend", n.counts[b] + " · " + bucketLabel(b));
        tag.style.setProperty("--bucket", "var(--b-" + b + ")");
        box.appendChild(tag);
        box.appendChild(document.createTextNode(" "));
      });
      aside.appendChild(box);
    }
    list(aside, t("ui.acceptance"), n.acceptance, function(li, item){ li.textContent = item; });
    list(aside, t("ui.evidence"), n.evidence, function(li, item){ li.textContent = item; });
    list(aside, t("ui.mandates"), n.mandates, function(li, id){
      var m = (DATA.mandates || []).filter(function(row){ return row.id === id; })[0];
      li.appendChild(el("span", "nid", id));
      if (m) li.appendChild(el("span", null, " " + m.text + (m.disposition ? " [" + m.disposition + "]" : "")));
    });
    DATA.edge_kinds.forEach(function(kind){
      var out = (n.edges || []).filter(function(e){ return e.kind === kind; });
      list(aside, t("ui.dependsOn"), out, function(li, edge){
        linkTo(li, edge.to, NODES[edge.to] ? NODES[edge.to].title : "?");
      });
      var back = (DEPENDENTS[n.id] || []).filter(function(e){ return e.kind === kind; });
      list(aside, t("ui.dependents"), back, function(li, edge){
        linkTo(li, edge.from, NODES[edge.from] ? NODES[edge.from].title : "?");
      });
    });
    var extraKeys = Object.keys(n.extra || {});
    if (extraKeys.length){
      aside.appendChild(el("h3", null, t("ui.extra")));
      var edl = el("dl");
      extraKeys.forEach(function(key){
        edl.appendChild(el("dt", null, key));
        edl.appendChild(el("dd", null, stringify(n.extra[key])));
      });
      aside.appendChild(edl);
    }
    if (n.children.length){
      list(aside, t("ui.children"), n.children, function(li, id){
        linkTo(li, id, NODES[id] ? NODES[id].title : "?");
      });
    }
  }
  function stringify(value){
    if (value === null || value === undefined) return "";
    if (typeof value === "object") return JSON.stringify(value, null, 1);
    return String(value);
  }

  /* ---- chrome ---------------------------------------------------------- */
  function banner(){
    var host = document.getElementById("banner");
    host.textContent = "";
    var v = DATA.validation || {};
    var box = el("div", "banner " + (v.status === "valid" ? "ok" : "bad"));
    if (v.status === "valid"){ box.appendChild(el("strong", null, t("ui.validationOk"))); }
    else {
      var label = v.status === "unavailable" ? t("ui.validationUnavailable") : t("ui.validationFail");
      box.appendChild(el("strong", null, label + ": "));
      box.appendChild(el("code", null, (v.code || "") + " " + (v.message || "")));
    }
    host.appendChild(box);
    if ((DATA.warnings || []).length){
      var warn = el("div", "banner warn");
      warn.appendChild(el("strong", null, t("ui.warnings") + " (" + DATA.warnings.length + ")"));
      var ul = el("ul");
      DATA.warnings.forEach(function(w){
        var li = el("li");
        li.appendChild(el("code", null, w.code));
        li.appendChild(el("span", null, " " + w.message));
        if (w.node && NODES[w.node]){ li.appendChild(document.createTextNode(" ")); linkTo(li, w.node, ""); }
        ul.appendChild(li);
      });
      warn.appendChild(ul);
      host.appendChild(warn);
    }
  }
  function totals(){
    var host = document.getElementById("totals");
    host.textContent = "";
    DATA.buckets.forEach(function(b){
      var value = (DATA.totals || {})[b];
      if (!value) return;
      var tag = el("span", "legend");
      tag.style.setProperty("--bucket", "var(--b-" + b + ")");
      tag.appendChild(el("span", "dot"));
      tag.appendChild(el("span", null, bucketLabel(b) + ": " + value));
      host.appendChild(tag);
    });
  }
  function meta(){
    var host = document.getElementById("meta");
    host.textContent = "";
    [DATA.plan.plan_id, t("ui.revision") + " " + DATA.plan.revision,
     DATA.plan.node_count + " " + t("ui.nodes"), DATA.plan.source_name,
     DATA.plan.updated_at || ""].forEach(function(item){
      if (item) host.appendChild(el("span", null, item));
    });
  }
  function chrome(){
    document.getElementById("app-title").textContent = t("app.title");
    document.title = t("app.title") + " — " + DATA.plan.plan_id;
    var q = document.getElementById("q");
    q.placeholder = t("ui.search");
    q.setAttribute("aria-label", t("ui.searchLabel"));
    document.getElementById("q-label").textContent = t("ui.searchLabel");
    document.getElementById("expand").textContent = t("ui.expandAll");
    document.getElementById("collapse").textContent = t("ui.collapseAll");
    var current = document.getElementById("current");
    current.textContent = t("ui.jumpCurrent");
    current.hidden = !DATA.plan.current_node;
    document.getElementById("theme").textContent = t("ui.theme") + ": " + theme;
    document.getElementById("lang").textContent = t("ui.lang") + ": " + lang;
    document.documentElement.lang = lang;
    document.documentElement.dataset.theme = theme;
    meta(); banner(); totals(); relabel(); detail();
  }

  document.getElementById("q").addEventListener("input", function(event){
    query = event.target.value.trim().toLowerCase();
    paint();
  });
  document.getElementById("expand").addEventListener("click", function(){
    DATA.nodes.forEach(function(n){ open[n.id] = true; });
    paint();
  });
  document.getElementById("collapse").addEventListener("click", function(){
    DATA.nodes.forEach(function(n){ open[n.id] = n.depth === 0; });
    paint();
  });
  document.getElementById("current").addEventListener("click", function(){
    if (DATA.plan.current_node) select(DATA.plan.current_node, true);
  });
  document.getElementById("theme").addEventListener("click", function(){
    theme = theme === "auto" ? "light" : theme === "light" ? "dark" : "auto";
    store("theme", theme);
    chrome();
  });
  document.getElementById("lang").addEventListener("click", function(){
    lang = DATA.langs[(DATA.langs.indexOf(lang) + 1) % DATA.langs.length];
    store("lang", lang);
    chrome();
  });
  document.addEventListener("keydown", function(event){
    if (event.key === "/" && event.target.tagName !== "INPUT"){
      event.preventDefault();
      document.getElementById("q").focus();
    }
  });

  buildRows();
  chrome();
  if (DATA.plan.current_node) reveal(DATA.plan.current_node);
  paint();
})();
</script>
</body>
</html>
"""


if __name__ == "__main__":
    sys.exit(main())
