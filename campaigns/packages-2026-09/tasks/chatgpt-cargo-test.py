#!/usr/bin/env python3
"""REDs for the ChatGPT campaign's two-slot Cargo semaphore."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest


MODULE = Path(__file__).with_name("chatgpt-cargo.py").resolve()


def load_module():
    spec = importlib.util.spec_from_file_location("chatgpt_cargo", MODULE)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


CHILD = textwrap.dedent(
    """
    import importlib.util, json, os, pathlib, sys, time
    spec = importlib.util.spec_from_file_location("chatgpt_cargo", sys.argv[1])
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    number, handle = module.acquire(pathlib.Path(sys.argv[2]), 10.0, ["cargo", "test"])
    started = time.time()
    print(json.dumps({"event": "start", "slot": number, "at": started}), flush=True)
    if sys.argv[3] == "crash":
        os._exit(7)
    time.sleep(float(sys.argv[3]))
    ended = time.time()
    module.unlock(handle)
    handle.close()
    print(json.dumps({"event": "end", "slot": number, "at": ended}), flush=True)
    """
)


def spawn(slot_dir: Path, action: str) -> subprocess.Popen[str]:
    return subprocess.Popen(
        [sys.executable, "-c", CHILD, str(MODULE), str(slot_dir), action],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def events(process: subprocess.Popen[str]) -> list[dict]:
    stdout, stderr = process.communicate(timeout=15)
    if process.returncode != 0:
        raise AssertionError(f"child exit {process.returncode}: {stderr}\n{stdout}")
    return [json.loads(line) for line in stdout.splitlines() if line.startswith("{")]


class CargoSlotsTest(unittest.TestCase):
    def test_three_contenders_never_run_more_than_two_at_once(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            slot_dir = Path(raw)
            children = [spawn(slot_dir, "0.8") for _ in range(3)]
            rows = [row for child in children for row in events(child)]
        timeline = sorted(
            (row["at"], 1 if row["event"] == "start" else -1) for row in rows
        )
        live = 0
        peak = 0
        for _, delta in timeline:
            live += delta
            peak = max(peak, live)
            self.assertGreaterEqual(live, 0)
        self.assertEqual(live, 0)
        self.assertEqual(peak, 2)
        starts = sorted(row["at"] for row in rows if row["event"] == "start")
        ends = sorted(row["at"] for row in rows if row["event"] == "end")
        self.assertGreaterEqual(starts[2], ends[0])

    def test_os_releases_a_slot_when_the_holder_crashes(self) -> None:
        module = load_module()
        with tempfile.TemporaryDirectory() as raw:
            slot_dir = Path(raw)
            crashed = spawn(slot_dir, "crash")
            crashed.communicate(timeout=10)
            self.assertEqual(crashed.returncode, 7)
            started = time.monotonic()
            number, handle = module.acquire(slot_dir, 2.0, ["cargo", "test"])
            elapsed = time.monotonic() - started
            module.unlock(handle)
            handle.close()
        self.assertIn(number, {1, 2})
        self.assertLess(elapsed, 1.0)


if __name__ == "__main__":
    unittest.main()
