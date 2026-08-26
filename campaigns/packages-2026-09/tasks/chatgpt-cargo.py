#!/usr/bin/env python3
"""Two-slot Cargo wrapper for the ChatGPT/Codex campaign harness only.

The lifecycle/extensions campaign deliberately runs several Claude/Opus/Codex
workers at once. A process census is racy: several workers can all observe zero
Cargo processes and start together. This wrapper acquires one of two OS locks
before spawning Cargo, holds it for the whole child lifetime, and releases it
automatically when the wrapper exits or crashes.

It is not a VibeVM product command and is not standing guidance for Claude.
The ChatGPT-only PROP-055 packet template tells selected workers to invoke it.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys
import time
from typing import BinaryIO


SLOTS = 2
WAIT_HEARTBEAT_SECS = 30.0


def repository_root(start: Path) -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
        cwd=start,
        check=True,
        capture_output=True,
        text=True,
    )
    common = Path(result.stdout.strip()).resolve()
    if common.name == ".git":
        return common.parent
    raise RuntimeError(f"expected a non-bare worktree, got git common dir {common}")


def try_lock(handle: BinaryIO) -> bool:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        try:
            msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
            return True
        except OSError:
            return False
    import fcntl

    try:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        return True
    except BlockingIOError:
        return False


def unlock(handle: BinaryIO) -> None:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
        return
    import fcntl

    fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def acquire(slot_dir: Path, timeout_secs: float, command: list[str]) -> tuple[int, BinaryIO]:
    slot_dir.mkdir(parents=True, exist_ok=True)
    deadline = time.monotonic() + timeout_secs
    next_heartbeat = time.monotonic()
    while True:
        for number in range(1, SLOTS + 1):
            path = slot_dir / f"slot-{number}.lock"
            handle = path.open("a+b")
            if try_lock(handle):
                print(f"CHATGPT-CARGO-SLOT: acquired {number}/{SLOTS}", flush=True)
                return number, handle
            handle.close()
        now = time.monotonic()
        if now >= deadline:
            raise TimeoutError(
                f"timed out after {timeout_secs:g}s waiting for one of {SLOTS} Cargo slots"
            )
        if now >= next_heartbeat:
            remaining = max(0, int(deadline - now))
            print(
                f"PROGRESS: waiting for one of {SLOTS} ChatGPT Cargo slots "
                f"({remaining}s timeout remaining)",
                flush=True,
            )
            next_heartbeat = now + WAIT_HEARTBEAT_SECS
        time.sleep(1.0)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run one Cargo command under the campaign's two-slot OS semaphore."
    )
    parser.add_argument("--cwd", type=Path, default=Path.cwd())
    parser.add_argument("--timeout-secs", type=float, default=3600.0)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if not args.command:
        parser.error("a Cargo command is required after `--`")
    executable = Path(args.command[0]).name.lower()
    if executable not in {"cargo", "cargo.exe"}:
        parser.error("the wrapped command must be cargo/cargo.exe")
    if args.timeout_secs <= 0:
        parser.error("--timeout-secs must be positive")
    return args


def main() -> int:
    args = parse_args()
    cwd = args.cwd.resolve()
    root = repository_root(cwd)
    slot_dir = root / "cache" / "chatgpt-cargo-slots"
    number, handle = acquire(slot_dir, args.timeout_secs, args.command)
    try:
        environment = os.environ.copy()
        environment.setdefault("CARGO_BUILD_JOBS", "4")
        return subprocess.run(args.command, cwd=cwd, env=environment, check=False).returncode
    finally:
        unlock(handle)
        handle.close()
        print(f"CHATGPT-CARGO-SLOT: released {number}/{SLOTS}", flush=True)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, TimeoutError, subprocess.CalledProcessError) as error:
        print(f"chatgpt-cargo: {error}", file=sys.stderr)
        raise SystemExit(2) from error
