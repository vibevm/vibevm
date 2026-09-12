"""R-25 gate for the campaign zone: nothing session-specific or private may be committed.

Run from the repository root before every commit that touches campaigns/docs-2026-09:

    python campaigns/docs-2026-09/tasks/zone-gate.py

Exit 1 with the offending lines when a file under the zone carries a local
temporary path, an IP address, a token-shaped string or a private-infrastructure
name. JOURNAL.md is exempt for the one entry (J-046) that quotes the patterns.
"""
import re
import sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8")
ZONE = Path("campaigns/docs-2026-09")
PATTERNS = [
    ("scratch path", re.compile(r"Temp[\\/]claude[\\/]|/tmp/claude|C--Users-")),
    ("ip address", re.compile(r"(?<![\w.])(?:\d{1,3}\.){3}\d{1,3}(?![\w.])")),
    ("token-like", re.compile(r"(?:ghp|gho|github_pat)_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9]{20,}")),
    ("private infra", re.compile(r"anarchic\.pro|x-ui\.service|\bREALITY\b(?!-)|\bxray\b")),
]
ALLOW = {"0.0.0.0", "127.0.0.1"}
bad = 0
for p in sorted(ZONE.rglob("*")):
    if not p.is_file() or p.suffix not in {".md", ".py", ".toml", ".txt", ".out", ".err"}:
        continue
    for i, line in enumerate(p.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        for name, rx in PATTERNS:
            m = rx.search(line)
            if not m:
                continue
            if name == "ip address" and m.group(0) in ALLOW:
                continue
            if p.name == "JOURNAL.md" and "J-046" in line:
                continue
            if "grep" in line:  # a line quoting the gate's own patterns
                continue
            if p.name == "zone-gate.py":
                continue
            bad += 1
            print(f"{p.as_posix()}:{i}: {name}: {line.strip()[:160]}")
print("zone gate:", "clean" if bad == 0 else f"{bad} finding(s)")
sys.exit(1 if bad else 0)
