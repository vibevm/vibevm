#!/usr/bin/env python3
"""Synchronize neutral source crates from the local package catalog.

This command never changes an authored source package.
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil


PACKAGE = Path(__file__).resolve().parents[1]
SOURCE = PACKAGE.parents[1] / "core-ai-native" / "v1.1.0"
CRATES = (
    "core-ai-native-specmap",
    "core-ai-native-specmark",
    "core-ai-native-specmark-grammar",
)


def files(root):
    return {p.relative_to(root).as_posix(): p for p in root.rglob("*") if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--record", action="store_true", help="verify copied bytes and refresh only the receipt")
    args = parser.parse_args()
    rows = []
    for crate in CRATES:
        source = SOURCE / "crates" / crate
        destination = PACKAGE / "crates" / "vendor" / crate
        if not source.is_dir():
            raise SystemExit(f"missing canonical source crate: {source}")
        if not args.check and not args.record:
            shutil.copytree(source, destination, dirs_exist_ok=True)
        original, copied = files(source), files(destination)
        if original.keys() != copied.keys():
            raise SystemExit(f"file-set drift: {crate}")
        for name, path in sorted(original.items()):
            data = path.read_bytes()
            if copied[name].read_bytes() != data:
                raise SystemExit(f"source drift: {crate}/{name}")
            rows.append({"path": f"crates/vendor/{crate}/{name}", "sha256": hashlib.sha256(data).hexdigest()})
    receipt = {"schema": 1, "source": "flow:org.vibevm.ai-native/core-ai-native", "version": "1.1.0", "files": rows}
    encoded = json.dumps(receipt, indent=2) + "\n"
    receipt_path = PACKAGE / "core-vendor-receipt.json"
    if args.check:
        if not receipt_path.is_file() or receipt_path.read_text() != encoded:
            raise SystemExit("core vendor receipt drift")
    else:
        receipt_path.write_text(encoded)
    print(f"neutral core copies match: {len(rows)} files")


if __name__ == "__main__":
    main()
