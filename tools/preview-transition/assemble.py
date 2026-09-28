#!/usr/bin/env python3
"""Assemble deterministic preview drafts through generated Rust readers."""

from __future__ import annotations

import argparse
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--transition", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()
    return subprocess.run(
        ["cargo", "run", "--quiet", "-p", "vibe-wire", "--bin",
         "preview-transition-assemble", "--", str(Path(args.transition).absolute()),
         str(Path(args.output_dir).absolute())],
        cwd=ROOT, check=False,
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
