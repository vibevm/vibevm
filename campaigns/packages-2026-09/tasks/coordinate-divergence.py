#!/usr/bin/env python3
"""One `name@version` coordinate, two contents — measured.

Usage:
    python tasks/coordinate-divergence.py

`qualified-naming` states the law this checks: *never reuse a `name@version`
coordinate for different content — a coordinate that meant one artifact must never
mean another.* `packages/<group>/<name>/v<ver>/` is the shipped artifact and
`vibedeps/<kind>-<name>/<ver>/` is what this consumer received; where the same
coordinate holds different bytes, the law is broken in this tree.

Slots are matched by PACKAGE IDENTITY, not by version number. A first version of
this script globbed `packages/*/*/v<ver>/<tail>` and matched `go-ai-native@0.1.0`
against `addressable-specs@0.1.0` — every package that happens to sit at the same
version number. The count it produced was meaningless and looked exactly like a
finding.
"""
import pathlib, hashlib, re, sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
KIND = re.compile(r"^(flow|stack|mcp|feat|tool)-")


def main():
    pkgdirs = {p.name: p for grp in (ROOT / "packages").iterdir() if grp.is_dir()
               for p in grp.iterdir() if p.is_dir()}
    tot = ident = diff = 0
    per, unmatched = {}, []
    for dep in sorted((ROOT / "vibedeps").iterdir()):
        if not dep.is_dir():
            continue
        pk = pkgdirs.get(KIND.sub("", dep.name))
        if not pk:
            unmatched.append(dep.name)
            continue
        for verdir in (d for d in dep.iterdir() if d.is_dir()):
            twin = pk / ("v" + verdir.name)
            if not twin.is_dir():
                continue
            for f in verdir.rglob("*.md"):
                t = twin / f.relative_to(verdir)
                if not t.is_file():
                    continue
                tot += 1
                if hashlib.sha256(t.read_bytes()).hexdigest() == hashlib.sha256(f.read_bytes()).hexdigest():
                    ident += 1
                else:
                    diff += 1
                    per[pk.name] = per.get(pk.name, 0) + 1
    if not tot:
        print("REFUSED: no coordinate matched in both trees — an empty comparison is not a clean one")
        return 1
    print(f"markdown files at the same (package, version) in packages/ and vibedeps/: {tot}")
    print(f"  byte-identical                     : {ident}")
    print(f"  DIFFERENT content, same coordinate : {diff}")
    print(f"  packages affected                  : {len(per)}")
    print(f"  vibedeps slots with no package twin: {unmatched or 'none'}")
    for k, v in sorted(per.items(), key=lambda kv: -kv[1]):
        print(f"    {k}: {v}")
    return 1 if diff else 0


if __name__ == "__main__":
    sys.exit(main())
