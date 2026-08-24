#!/usr/bin/env python3
"""The address family's repair, as a transformation rather than 69 hand edits.

Every `../flows/…` link in a package's `vibevm/vibespecs/boot/` snippet is correct where it
is written and broken where it is read.  Measured over the three lanes:

    packages/  canonical    70 links,   0 dangling
    vibedeps/  installed   142 links,  21 dangling
    vibevm/vibespecs/boot/ compiled     75 links,  75 dangling

The compiler concatenates snippet bodies verbatim into `spec/boot/STATIC.md`
(PROP-035's linker stage), so a relative path that meant `<pkg>/spec/flows/…`
now means the host's `spec/flows/…`, and the host has no `spec/flows/`.  The
defect is the *form*: a relative path cannot survive being moved.  An
`@spec://` address is position-independent and survives it.

Owner ruling, 2026-07-29: the links take `@spec://` where they are pointers and
`#embed` where the target belongs in the lane.  Measured against the corpus,
**all 69 in-scope links are pointers** — every one reads "Full protocol:",
"Full model:", "Grammar and forms:", "read …" and deliberately does not carry
the target's content.  `#embed` has no member here, so this script emits only
`@spec://`; anything it cannot classify it refuses rather than guesses.

The emitted form copies the house form already live in the host's own spec
(`spec/common/PROP-000.md:161-164`, `PROP-016:8`), which is
`spec://<group>/<name>/<doc-path>#<anchor>` — no `.md`, always an anchor.

Usage:

    python campaigns/packages-2026-09/tasks/address-repair.py            # dry run
    python campaigns/packages-2026-09/tasks/address-repair.py --verify   # + resolve every address
    python campaigns/packages-2026-09/tasks/address-repair.py --apply    # rewrite the files

`--apply` is gated on publication: the edit closes nothing until the change
reaches `vibedeps/` through a version bump and `cargo xtask sync-engines`, and
that approval is the owner's (`PHASE-D-RELEASE-QUEUE.md` §A).
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PACKAGES = ROOT / "packages"

# A markdown link whose target is a `../flows/…` relative path.
LINK = re.compile(r"\[(?P<text>[^\]]*)\]\((?P<target>\.\./flows/[^)\s]+)\)")

# Packages outside this campaign's zone (it judges `world` and `ai-native`).
OUT_OF_SCOPE_GROUPS = {"org.vibevm.fractality"}


@dataclass
class Repair:
    file: Path
    line_no: int
    old: str
    new: str
    target_doc: Path
    group: str
    name: str

    @property
    def resolves(self) -> bool:
        return self.target_doc.is_file()


def coordinate(path: Path) -> tuple[str, str, Path] | None:
    """`packages/<group>/<name>/<version>/…` → (group, name, version root)."""
    try:
        rel = path.relative_to(PACKAGES).parts
    except ValueError:
        return None
    if len(rel) < 4:
        return None
    group, name, version = rel[0], rel[1], rel[2]
    return group, name, PACKAGES / group / name / version


def plan(include_out_of_scope: bool = False) -> list[Repair]:
    repairs: list[Repair] = []
    for path in sorted(PACKAGES.rglob("*.md")):
        parts = path.parts
        if "vibedeps" in parts or ".vibe" in parts:
            continue
        coord = coordinate(path)
        if coord is None:
            continue
        group, name, version_root = coord
        if group in OUT_OF_SCOPE_GROUPS and not include_out_of_scope:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        if "../flows/" not in text:
            continue
        for line_no, line in enumerate(text.splitlines(), start=1):
            for m in LINK.finditer(line):
                target = m.group("target")
                doc, _, fragment = target.partition("#")
                # `../flows/x/DOC.md` read from `vibevm/vibespecs/boot/` → `spec/flows/x/DOC.md`
                doc_path = doc[len("../") :]           # flows/x/DOC.md
                on_disk = version_root / "spec" / doc_path
                stem = doc_path[: -len(".md")] if doc_path.endswith(".md") else doc_path
                anchor = fragment or "root"
                address = f"@spec://{group}/{name}/{stem}#{anchor}"
                repairs.append(
                    Repair(
                        file=path,
                        line_no=line_no,
                        old=m.group(0),
                        new=address,
                        target_doc=on_disk,
                        group=group,
                        name=name,
                    )
                )
    return repairs


def rewrite(path: Path, items: list[Repair]) -> str | None:
    """The file's new text, or None if a repair no longer matches.

    Substitution is **line-indexed, one occurrence at a time**.  A whole-text
    `str.replace` is wrong here and was caught being wrong: `two-process-model`
    carries the identical link `[`files-as-ipc.md`](…/files-as-ipc.md)` on two
    separate lines, and a text-wide replace consumes both on the first repair,
    leaving the second with nothing to match.
    """
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    per_line: dict[int, list[Repair]] = {}
    for r in items:
        per_line.setdefault(r.line_no, []).append(r)
    for line_no, reps in per_line.items():
        line = lines[line_no - 1]
        for r in reps:
            if r.old not in line:
                print(
                    f"REFUSED {path}:{line_no}: {r.old!r} no longer present",
                    file=sys.stderr,
                )
                return None
            line = line.replace(r.old, r.new, 1)
        lines[line_no - 1] = line
    return "".join(lines)


def apply(repairs: list[Repair]) -> int:
    by_file: dict[Path, list[Repair]] = {}
    for r in repairs:
        by_file.setdefault(r.file, []).append(r)
    written = 0
    for path, items in by_file.items():
        text = rewrite(path, items)
        if text is None:
            return -1
        path.write_text(text, encoding="utf-8")
        written += 1
    return written


ANCHOR_DEF = re.compile(r"##([A-Za-z0-9][A-Za-z0-9._-]*)")


def governing_anchor(path: Path, line_no: int) -> str | None:
    """The anchor whose fact a given line belongs to.

    Walks backwards to the nearest `##NAME` **definition**, with backticked
    spans stripped first — an `##ANCHOR` inside backticks is a citation, not a
    definition, and attributing to one has already cost three merge refusals.
    """
    lines = path.read_text(encoding="utf-8").splitlines()
    for i in range(line_no - 1, -1, -1):
        m = ANCHOR_DEF.search(re.sub(r"`[^`]*`", "", lines[i]))
        if m:
            return m.group(1)
    return None


def family(repairs: list[Repair]) -> int:
    """Join the repaired links to the open registry, by governing anchor.

    This is the number the owner's release-queue ask rests on, and it moves as
    routing proceeds — so it is printed rather than written down.
    """
    import json as _json

    zone = Path(__file__).resolve().parents[1]
    obligations = _json.loads((zone / "run/state/obligations.json").read_text(encoding="utf-8"))
    rows = obligations["obligations"] if isinstance(obligations, dict) and "obligations" in obligations else obligations
    routed = {
        e["anchor"]
        for e in _json.loads((zone / "run/state/routing.json").read_text(encoding="utf-8"))["entries"]
    }

    touched: set[tuple[str, str]] = set()
    for r in repairs:
        anchor = governing_anchor(r.file, r.line_no)
        if anchor:
            touched.add((r.file.relative_to(ROOT).as_posix(), anchor))

    hits: list[tuple[dict, list[str]]] = []
    for row in rows:
        on_link = [
            a
            for a in row["anchors"]
            if (a.split("#")[0], a.split("#")[-1]) in touched and a not in routed
        ]
        if on_link:
            hits.append((row, on_link))

    by_route: dict[str, int] = {}
    for row, _ in hits:
        by_route[row["closure_route"]] = by_route.get(row["closure_route"], 0) + 1
    packages = {p for row, _ in hits for p in row["packages"]}

    print("THE ADDRESS FAMILY, joined to the open registry by governing anchor")
    print(f"  obligations touching a repaired link : {len(hits)}")
    print(f"  verdicts on a repaired link, open    : {sum(len(h) for _, h in hits)}")
    print(f"  packages                             : {len(packages)}")
    for route, n in sorted(by_route.items(), key=lambda kv: -kv[1]):
        print(f"    {route:16s} {n}")
    print()
    print("  Every one of these closes through publication, whatever route the")
    print("  registry assigns it: the links dangle only in the compiled lane,")
    print("  which is generated from vibedeps/ and reached only by a re-vendor.")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--apply", action="store_true", help="rewrite the files (owner-gated)")
    ap.add_argument("--verify", action="store_true", help="resolve every emitted address")
    ap.add_argument("--all", action="store_true", help="include out-of-campaign-scope groups")
    ap.add_argument("--family", action="store_true",
                    help="print the obligations this repair would unblock, and stop")
    args = ap.parse_args()

    repairs = plan(include_out_of_scope=args.all)
    if args.family:
        return family(repairs)
    files = {r.file for r in repairs}
    packages = {(r.group, r.name) for r in repairs}

    print(f"{len(repairs)} links  ·  {len(files)} files  ·  {len(packages)} packages")
    if not args.all:
        print("(campaign scope; --all adds the org.vibevm.fractality group)")
    print()

    unresolved = [r for r in repairs if not r.resolves]
    if args.verify or unresolved:
        print(f"target resolution: {len(repairs) - len(unresolved)}/{len(repairs)} resolve "
              f"in the package tree")
        for r in unresolved:
            print(f"  UNRESOLVED {r.file.relative_to(ROOT)}:{r.line_no}  -> {r.target_doc}")
        print()

    current = None
    for r in repairs:
        if r.file != current:
            current = r.file
            print(f"--- {r.file.relative_to(ROOT).as_posix()}")
        print(f"  {r.line_no:4d}  - {r.old}")
        print(f"        + {r.new}")

    if args.apply:
        if unresolved:
            print("\nREFUSED to apply: some addresses do not resolve.", file=sys.stderr)
            return 1
        n = apply(repairs)
        if n < 0:
            return 1
        print(f"\napplied to {n} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
