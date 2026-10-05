#!/usr/bin/env python3
"""Check or synchronize the exact-pinned authored Lean checking engine."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

PACKAGE = Path(__file__).resolve().parents[1]
SOURCE = PACKAGE.parents[1] / "lean-ai-native-lang" / "v0.1.0" / "crates" / "lean-ai-native-check"
DESTINATION = PACKAGE / "crates" / "lean-ai-native-check"


def files(root):
    return {p.relative_to(root).as_posix(): p for p in root.rglob("*") if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--record", action="store_true")
    args = parser.parse_args()
    if not SOURCE.is_dir():
        raise SystemExit(f"missing authored engine: {SOURCE}")
    if not args.check and not args.record:
        shutil.copytree(SOURCE, DESTINATION, dirs_exist_ok=True)
    source, copied = files(SOURCE), files(DESTINATION)
    if source.keys() != copied.keys():
        raise SystemExit("engine file-set drift")
    rows = []
    for name, path in sorted(source.items()):
        data = path.read_bytes()
        if data != copied[name].read_bytes():
            raise SystemExit(f"engine source drift: {name}")
        rows.append({"path": name, "sha256": hashlib.sha256(data).hexdigest()})
    receipt = {"schema": 1, "source": "lang:org.vibevm.ai-native/lean-ai-native-lang", "version": "0.1.0", "files": rows}
    encoded = json.dumps(receipt, indent=2) + "\n"
    path = PACKAGE / "engine-vendor-receipt.json"
    if args.check:
        if not path.is_file() or path.read_text() != encoded:
            raise SystemExit("engine vendor receipt drift")
    else:
        path.write_text(encoded)
    print(f"checking engine matches: {len(rows)} files")


if __name__ == "__main__":
    main()
