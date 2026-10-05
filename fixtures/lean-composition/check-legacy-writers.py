#!/usr/bin/env python3
"""Run the real legacy writers against isolated composition fixtures.

Both writers must refuse the unsupported composition field before replacing
an existing specmap.json. No checker, Node installation, or floor is run.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


FIXTURE = Path(__file__).resolve().parent
SENTINEL = b'{"legacy_writer_sentinel":"retain exact bytes"}\n'


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def exercise(label, binary):
    binary = binary.resolve(strict=True)
    if not binary.is_file():
        raise ValueError(f"{label}: supplied binary is not a regular file")
    with tempfile.TemporaryDirectory(prefix="lean-legacy-refusal-") as temporary:
        root = Path(temporary) / "consumer"
        shutil.copytree(
            FIXTURE,
            root,
            ignore=shutil.ignore_patterns(".targets", "target", ".vibe", "__pycache__", "specmap.json", "proof-artifacts"),
        )
        sentinel = root / "specmap.json"
        sentinel.write_bytes(SENTINEL)
        # An unexpected extraction path cannot invoke an ambient Node tool.
        # The expected failure is the configuration decoder, not tool absence.
        empty_path = Path(temporary) / "empty-path"
        empty_path.mkdir()
        environment = os.environ.copy()
        environment["PATH"] = str(empty_path)
        output = subprocess.run(
            [str(binary), "--path", str(root)],
            cwd=root,
            env=environment,
            capture_output=True,
            timeout=30,
            check=False,
        )
        stderr = output.stderr.decode("utf-8", errors="replace")
        stdout = output.stdout.decode("utf-8", errors="replace")
        unchanged = sentinel.is_file() and sentinel.read_bytes() == SENTINEL
        unknown_composition = "unknown field" in stderr and "composition" in stderr
        passed = output.returncode != 0 and unchanged and unknown_composition
        return {
            "writer": label,
            "binary": str(binary),
            "binary_sha256": sha256(binary.read_bytes()),
            "argv": [str(binary), "--path", "<fixture-copy>"],
            "exit_code": output.returncode,
            "refused_unknown_composition": unknown_composition,
            "sentinel_unchanged": unchanged,
            "sentinel_sha256": sha256(SENTINEL),
            "stdout": stdout.replace(str(root), "<fixture-copy>")[:1000],
            "stderr": stderr.replace(str(root), "<fixture-copy>")[:3000],
            "passed": passed,
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-binary", type=Path, required=True)
    parser.add_argument("--typescript-binary", type=Path, required=True)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    results = [exercise("rust", args.rust_binary), exercise("typescript", args.typescript_binary)]
    receipt = {
        "schema": "lean-composition/legacy-writer-refusal/1",
        "fixture": str(FIXTURE),
        "script_sha256": sha256(Path(__file__).read_bytes()),
        "specmap_config_sha256": sha256((FIXTURE / "specmap.toml").read_bytes()),
        "results": results,
        "passed": all(result["passed"] for result in results),
    }
    if args.receipt:
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    for result in results:
        print(json.dumps({key: result[key] for key in (
            "writer", "exit_code", "refused_unknown_composition", "sentinel_unchanged", "passed",
        )}))
    if not receipt["passed"]:
        raise SystemExit("a legacy writer failed the refusal/unchanged-sentinel contract")


if __name__ == "__main__":
    main()
