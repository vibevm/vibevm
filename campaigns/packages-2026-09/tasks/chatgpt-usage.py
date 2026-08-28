#!/usr/bin/env python3
"""Read the signed-in Codex plan window and enforce the campaign pause reserve.

ChatGPT/Codex plan usage is not the OpenAI API organisation-usage API.  The
desktop/CLI app-server already exposes the same read-only account snapshot its
Usage UI consumes.  This helper speaks that local stdio protocol, prints only
safe rate-limit metadata, and exits 75 when the requested pause reserve is reached.

Harness-only: PROP-055 owns the policy.  No token, auth header, account id,
prompt, response body, repository byte or credit-reset mutation is requested.
"""

from __future__ import annotations

import argparse
import json
import os
import queue
import shutil
import subprocess
import sys
import threading
import time
from typing import Any


STOP_EXIT = 75


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    result.add_argument(
        "--pause-at-remaining",
        type=int,
        default=15,
        metavar="PERCENT",
        help="exit 75 when the minimum remaining Codex window is at or below this value",
    )
    result.add_argument("--timeout", type=float, default=20.0, metavar="SECONDS")
    return result


def codex_command() -> list[str]:
    # Prefer the native executable.  On this Windows host `codex` first resolves
    # to a PowerShell shim, which CreateProcess cannot execute directly.
    candidate = shutil.which("codex.exe") or shutil.which("codex")
    if not candidate:
        raise RuntimeError("`codex` is not available on PATH")
    if os.name == "nt" and candidate.lower().endswith((".ps1", ".cmd", ".bat")):
        raise RuntimeError("PATH exposes only a shell shim; install/resolve native `codex.exe`")
    return [candidate, "app-server", "--stdio"]


def read_snapshot(timeout: float) -> dict[str, Any]:
    creationflags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    process = subprocess.Popen(
        codex_command(),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        encoding="utf-8",
        creationflags=creationflags,
    )
    assert process.stdin is not None
    assert process.stdout is not None
    lines: queue.Queue[str | None] = queue.Queue()

    def read_lines() -> None:
        assert process.stdout is not None
        for line in process.stdout:
            lines.put(line)
        lines.put(None)

    threading.Thread(target=read_lines, daemon=True).start()
    deadline = time.monotonic() + timeout

    def send(message: dict[str, Any]) -> None:
        process.stdin.write(json.dumps(message, separators=(",", ":")) + "\n")
        process.stdin.flush()

    def wait_for(request_id: int) -> dict[str, Any]:
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError(f"Codex usage read exceeded {timeout:g}s")
            try:
                line = lines.get(timeout=remaining)
            except queue.Empty as error:
                raise RuntimeError(f"Codex usage read exceeded {timeout:g}s") from error
            if line is None:
                raise RuntimeError("Codex app-server closed before answering usage")
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                continue
            if message.get("id") == request_id:
                return message

    try:
        send(
            {
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": {"name": "campaign-usage-monitor", "version": "0.1.0"},
                    "capabilities": {"experimentalApi": True},
                },
            }
        )
        initialized = wait_for(1)
        if initialized.get("error") is not None:
            raise RuntimeError(f"Codex initialize refused: {initialized['error']!r}")
        send({"method": "initialized"})
        send({"id": 2, "method": "account/rateLimits/read", "params": None})
        message = wait_for(2)
        if message.get("error") is not None:
            raise RuntimeError(f"Codex usage request refused: {message['error']!r}")
        result = message.get("result")
        if not isinstance(result, dict):
            raise RuntimeError("Codex usage response has no object result")
        return result
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=2)


def safe_summary(snapshot: dict[str, Any], threshold: int) -> dict[str, Any]:
    buckets = snapshot.get("rateLimitsByLimitId")
    bucket = buckets.get("codex") if isinstance(buckets, dict) else None
    if not isinstance(bucket, dict):
        bucket = snapshot.get("rateLimits")
    if not isinstance(bucket, dict):
        raise RuntimeError("Codex rate-limit snapshot has no `codex` bucket")

    windows: list[dict[str, Any]] = []
    for name in ("primary", "secondary"):
        window = bucket.get(name)
        if not isinstance(window, dict) or not isinstance(window.get("usedPercent"), int):
            continue
        used = max(0, min(100, window["usedPercent"]))
        windows.append(
            {
                "name": name,
                "used_percent": used,
                "remaining_percent": 100 - used,
                "window_duration_minutes": window.get("windowDurationMins"),
                "resets_at_unix": window.get("resetsAt"),
            }
        )
    individual = bucket.get("individualLimit")
    if isinstance(individual, dict) and isinstance(individual.get("remainingPercent"), int):
        remaining = max(0, min(100, individual["remainingPercent"]))
        windows.append(
            {
                "name": "individual",
                "used_percent": 100 - remaining,
                "remaining_percent": remaining,
                "window_duration_minutes": None,
                "resets_at_unix": individual.get("resetsAt"),
            }
        )
    if not windows:
        raise RuntimeError("Codex rate-limit snapshot carries no measurable window")
    remaining = min(window["remaining_percent"] for window in windows)
    return {
        "limit_id": bucket.get("limitId", "codex"),
        "plan_type": bucket.get("planType"),
        "remaining_percent": remaining,
        "pause_at_remaining_percent": threshold,
        "decision": "pause" if remaining <= threshold else "continue",
        "windows": windows,
    }


def main() -> int:
    args = parser().parse_args()
    if not 0 <= args.pause_at_remaining <= 100:
        parser().error("--pause-at-remaining must be between 0 and 100")
    try:
        summary = safe_summary(read_snapshot(args.timeout), args.pause_at_remaining)
    except (OSError, RuntimeError, ValueError) as error:
        print(f"CHATGPT-USAGE-ERROR: {error}", file=sys.stderr)
        return 2
    print("CHATGPT-USAGE: " + json.dumps(summary, separators=(",", ":"), sort_keys=True))
    return STOP_EXIT if summary["decision"] == "pause" else 0


if __name__ == "__main__":
    raise SystemExit(main())
