#!/usr/bin/env python3
"""coverage.py — the suite's coverage gate (PROP-075 `#VIBENVM-SUITE`).

Every `(subcommand, flag)` pair in the extracted `inventory.json` must be
covered by at least one **passing** differential case or appear in
`exclusions.toml` as an owner-approved entry. Nothing here reads or writes
an opinion: coverage is derived from the command lines the cases actually
ran, and only from cases that passed.

usage: coverage.py [--inventory PATH] [--exclusions PATH] [--results DIR]
                   [--quiet]

exit:  0  every pair is covered or owner-approved as a gap
       2  coverage is incomplete, or the exclusion file is not what the
          owner approved (an unapproved row, a row the extraction yields
          no pair for, or a row outside the closed set)
       3  a harness error — a file is missing or unreadable

Run as `python3 -I coverage.py …`; it imports only the standard library.
"""

from __future__ import annotations

import argparse
import json
import re
import shlex
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent

# **The closed set is empty, and that is the whole statement.** The
# owner's twelve approved items of 2026-11-10 are all implemented in
# `org.vibevm.nvm/nvm` (PROP-075 `#VIBENVM-NO-EXCLUSIONS`), so there is
# no accepted gap left and `exclusions.toml` is gone. The machinery
# stays, because the property it enforces is now stronger than it was:
# this constant is the lock on the list, so a row added to a re-created
# `exclusions.toml` is **not** in the closed set and the gate fails on
# it — an exclusion cannot be reintroduced without the owner reopening
# this set, which is a change to this file and not to a data file beside
# it.
OWNER_APPROVED: frozenset[tuple[str, str]] = frozenset()

APPROVED_ITEM_COUNT = 0

# Subcommand spellings the extraction never yields, by design: an exclusion
# row carrying one is reported as recorded, not required.
RECORDED_ONLY = ("(environment)", "(internal)")

DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


class GateFailure(Exception):
    """A coverage or exclusion-file failure: exit 2."""


class HarnessError(Exception):
    """A missing or unreadable input: exit 3."""


def load_inventory(path: Path) -> dict:
    try:
        with path.open(encoding="utf-8") as handle:
            inventory = json.load(handle)
    except OSError as exc:
        raise HarnessError(f"cannot read {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise HarnessError(f"{path} is not JSON: {exc}") from exc
    if inventory.get("schema") != 1:
        raise HarnessError(f"{path}: unsupported schema {inventory.get('schema')!r}")
    return inventory


def load_exclusions(path: Path) -> list[dict]:
    # No file at all is the expected state: every surface the owner once
    # approved as a gap is implemented, so there is nothing to exclude
    # (PROP-075 `#VIBENVM-NO-EXCLUSIONS`). It is not a harness error and
    # it is not silence either — `report` says so.
    if not path.exists():
        return []
    try:
        with path.open("rb") as handle:
            data = tomllib.load(handle)
    except OSError as exc:
        raise HarnessError(f"cannot read {path}: {exc}") from exc
    except tomllib.TOMLDecodeError as exc:
        raise HarnessError(f"{path} is not TOML: {exc}") from exc
    if data.get("schema") != 1:
        raise GateFailure(f"{path}: unsupported schema {data.get('schema')!r}")
    rows = data.get("exclusion", [])
    if not isinstance(rows, list):
        raise GateFailure(f"{path}: [[exclusion]] must be a list of rows")
    return rows


def check_rows(rows: list[dict], path: Path) -> None:
    """Every row approved by the owner, dated, and inside the closed set."""
    failures: list[str] = []
    seen: set[tuple[str, str]] = set()
    items: set[str] = set()
    for index, row in enumerate(rows, start=1):
        where = f"{path.name} row {index}"
        subcommand = row.get("subcommand")
        flag = row.get("flag")
        if not isinstance(subcommand, str) or not isinstance(flag, str):
            failures.append(f"{where}: needs string `subcommand` and `flag`")
            continue
        where = f"{where} ({subcommand} {flag})".rstrip()
        if row.get("approved_by") != "owner":
            failures.append(
                f"{where}: approved_by is {row.get('approved_by')!r}, not \"owner\" "
                "— an unreviewed exclusion cannot pass as reviewed"
            )
        approved = row.get("approved")
        if not isinstance(approved, str) or not DATE.match(approved):
            # TOML may already have parsed a bare date into a date object.
            if not DATE.match(str(approved)):
                failures.append(f"{where}: `approved` is not an RFC 3339 date")
        if not str(row.get("reason", "")).strip():
            failures.append(f"{where}: carries no reason")
        if not str(row.get("item", "")).strip():
            failures.append(f"{where}: names no approved item")
        else:
            items.add(str(row["item"]))
        key = (subcommand, flag)
        if key in seen:
            failures.append(f"{where}: duplicate row")
        seen.add(key)
        if key not in OWNER_APPROVED:
            failures.append(
                f"{where}: not one of the owner's approved exclusions "
                "(PROP-075 #VIBENVM-EXCLUSIONS-APPROVED closed the list)"
            )
    for missing in sorted(OWNER_APPROVED - seen):
        failures.append(
            f"{path.name}: owner-approved exclusion {missing[0]} {missing[1]} "
            "is missing from the file"
        )
    if items and len(items) != APPROVED_ITEM_COUNT:
        failures.append(
            f"{path.name}: {len(items)} approved items, not {APPROVED_ITEM_COUNT} "
            "— the owner's approved set is closed and currently empty"
        )
    if failures:
        raise GateFailure("\n".join(failures))


def expand_exclusions(
    rows: list[dict], pairs: set[tuple[str, str]], path: Path
) -> tuple[set[tuple[str, str]], list[dict]]:
    """Resolve the rows against the extracted pairs.

    A row with an empty flag excludes the whole subcommand; `subcommand =
    "*"` excludes that flag wherever the extraction found it; a row whose
    subcommand is `(environment)` or `(internal)` is recorded and matches
    nothing. Any other row that matches no pair is a gate failure — it
    would be an exclusion for a surface the pinned tree does not have.
    """
    excluded: set[tuple[str, str]] = set()
    recorded: list[dict] = []
    failures: list[str] = []
    for row in rows:
        subcommand, flag = row["subcommand"], row["flag"]
        if subcommand in RECORDED_ONLY:
            recorded.append(row)
            continue
        if subcommand == "*":
            hit = {pair for pair in pairs if pair[1] == flag}
        elif flag == "":
            hit = {pair for pair in pairs if pair[0] == subcommand}
        else:
            hit = {(subcommand, flag)} & pairs
        if not hit:
            failures.append(
                f"{path.name}: exclusion {subcommand} {flag} matches no extracted "
                "pair — the pinned tree yields no such surface"
            )
        excluded |= hit
    if failures:
        raise GateFailure("\n".join(failures))
    return excluded, recorded


def canonical_map(inventory: dict) -> dict[str, str]:
    mapping: dict[str, str] = {}
    for entry in inventory["subcommands"]:
        mapping[entry["name"]] = entry["name"]
        for alias in entry["aliases"]:
            mapping[alias] = entry["name"]
    return mapping


def pairs_of_command(line: str, canonical: dict[str, str]) -> set[tuple[str, str]]:
    """The inventory pairs one command line exercises.

    The tool under test is the leading `nvm` token, so only a line that
    begins with it counts. Flags are the tokens directly after the
    subcommand, which is nvm's own rule for its optional arguments
    ("must appear directly after `nvm install`"), and that rule is what
    keeps a later `node -v` out of the accounting.
    """
    try:
        tokens = shlex.split(line, comments=True)
    except ValueError:
        tokens = line.split()
    if not tokens or tokens[0] != "nvm" or len(tokens) < 2:
        return set()
    spelling = tokens[1]
    name = canonical.get(spelling, spelling)
    # The spelling as written covers the inventory's row for that spelling —
    # an alias is a pair of its own, and running it is what proves it exists.
    # Flags are recorded against the canonical name, which is where the
    # extraction records them: the alias enters the same branch.
    found = {(spelling, ""), (name, "")}
    for token in tokens[2:]:
        if token == "--" or not token.startswith("-"):
            break
        found.add((name, token.split("=", 1)[0]))
    return found


def read_results(results: Path) -> list[dict]:
    """Read run.sh's per-case records: `status` and one `command` per line."""
    if not results.is_dir():
        raise HarnessError(f"no results directory at {results}")
    cases = []
    for record in sorted(results.glob("*.tsv")):
        status = "unknown"
        commands: list[str] = []
        for raw in record.read_text(encoding="utf-8").splitlines():
            key, _, value = raw.partition("\t")
            if key == "status":
                status = value.strip()
            elif key == "command":
                commands.append(value)
        cases.append({"name": record.stem, "status": status, "commands": commands})
    return cases


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--inventory", type=Path, default=HERE / "inventory.json")
    parser.add_argument("--exclusions", type=Path, default=HERE / "exclusions.toml")
    parser.add_argument(
        "--results",
        type=Path,
        default=None,
        help="run.sh's per-case records; without it only the exclusion file is checked",
    )
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args(argv)

    try:
        inventory = load_inventory(args.inventory)
        pairs = {(p["subcommand"], p["flag"]) for p in inventory["pairs"]}
        rows = load_exclusions(args.exclusions)
        check_rows(rows, args.exclusions)
        excluded, recorded = expand_exclusions(rows, pairs, args.exclusions)

        covered: set[tuple[str, str]] = set()
        failed_cases: list[str] = []
        cases: list[dict] = []
        if args.results is not None:
            cases = read_results(args.results)
            for case in cases:
                if case["status"] != "pass":
                    failed_cases.append(case["name"])
                    continue
                canonical = canonical_map(inventory)
                for command in case["commands"]:
                    covered |= pairs_of_command(command, canonical) & pairs

        required = pairs - excluded
        uncovered = sorted(required - covered)

        if not args.quiet:
            print(f"inventory:  {len(pairs)} pairs from nvm {inventory['nvm_tag']}")
            print(f"excluded:   {len(excluded)} pairs, {len(rows)} owner-approved rows")
            print(f"            ({len(recorded)} recorded, not required)")
            if args.results is None:
                print("results:    none given — exclusion file checked only")
            else:
                passed = len(cases) - len(failed_cases)
                print(f"cases:      {passed} passed, {len(failed_cases)} not passed")
                print(f"covered:    {len(covered & required)} of {len(required)} required pairs")
        if uncovered:
            print(f"UNCOVERED {len(uncovered)} pair(s):", file=sys.stderr)
            for subcommand, flag in uncovered:
                print(f"  nvm {subcommand} {flag}".rstrip(), file=sys.stderr)
            if failed_cases:
                print(
                    "note: these cases did not pass, so they cover nothing: "
                    + ", ".join(sorted(failed_cases)),
                    file=sys.stderr,
                )
            return 2
        if not args.quiet:
            print("coverage:   complete")
        return 0
    except GateFailure as exc:
        print(f"coverage: {exc}", file=sys.stderr)
        return 2
    except HarnessError as exc:
        print(f"coverage: {exc}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
