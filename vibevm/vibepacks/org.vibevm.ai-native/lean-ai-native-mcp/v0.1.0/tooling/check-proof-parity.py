#!/usr/bin/env python3
"""Check one real pinned-Lean proof through both compiled front doors.

This is a focused maintainer check, not a product-wide gate. Both commands
run against the supplied project; their engine uses private temporary builds.
"""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ("cli", "mcp", "project", "toolchain", "subject"):
        parser.add_argument(f"--{flag}", required=True)
    parser.add_argument("--carrier", default="lean-proof.toml")
    args = parser.parse_args()
    common = ["--path", str(Path(args.project).resolve()), "--toolchain-dir", str(Path(args.toolchain).resolve())]
    cli = subprocess.run([str(Path(args.cli).resolve()), *common, "check", "--carrier", args.carrier, "--subject", args.subject], capture_output=True, text=True, timeout=300)
    if cli.returncode:
        raise SystemExit(f"CLI check failed ({cli.returncode}): {cli.stdout}{cli.stderr}")
    expected = json.loads(cli.stdout)
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "lean_ai_native_check", "arguments": {"carrier": args.carrier, "subject": args.subject}}}
    mcp = subprocess.run([str(Path(args.mcp).resolve()), *common], input=json.dumps(request) + "\n", capture_output=True, text=True, timeout=300)
    if mcp.returncode:
        raise SystemExit(f"MCP process failed ({mcp.returncode}): {mcp.stderr}")
    message = json.loads(mcp.stdout)
    result = message["result"]
    actual = json.loads(result["content"][0]["text"])
    if expected != actual or result["isError"] or not actual["ok"]:
        raise SystemExit("CLI/MCP proof result mismatch or refused proof")
    print(json.dumps({"ok": True, "subject": args.subject, "grade": actual["result"]["grade"], "outcome": actual["result"]["outcome"], "cli_mcp_exact_reply_equal": True}, indent=2))


if __name__ == "__main__":
    main()
