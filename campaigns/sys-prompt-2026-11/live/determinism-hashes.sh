#!/usr/bin/env bash
# Measure the cross-operating-system half of `PROP-076#SYS-DETERMINISM`.
#
# The fact's claim is mechanical: "two checkouts of the same commit in
# different directories, on Linux, macOS and Windows, produce byte-identical
# `SYSTEM.md` and `SYSTEM.meta.json`". P7 measured the two-directories half
# on Linux and left the two other operating systems argued rather than
# measured. This script is the measurement, made runnable by whoever has
# the host: it prints hashes, and the coordinator compares them with the
# Linux figures recorded in `LOG.md`.
#
# It builds the committed fixture `live/sample/` twice, at two different
# paths, through `make-sample.sh` — `vibe install --offline` against a
# `file://` registry inside the copy. **It contacts nothing, starts no agent
# and calls no model.**
#
# usage: determinism-hashes.sh <scratch-dir>
#
#   <scratch-dir>  created; must not exist. Keep it outside the repository.
#
# env: VIBE=<path>   the vibe binary (default: `vibe` on PATH)
#
# What it prints, and why each line is there:
#
#   inputs  <sha256>   over the committed `live/sample/` tree — every
#                      relative path and every byte, in sorted path order.
#                      A **different inputs hash means the checkout differs**,
#                      which is a line-ending or a working-tree question and
#                      not a determinism failure. It is printed first so that
#                      failure is diagnosed rather than discovered as a
#                      mysterious artifact mismatch. `.gitattributes` pins
#                      the tree to LF in every checkout, so this hash is the
#                      proof that the pin held on this host.
#   SYSTEM.md          <sha256>  — the prefix (`SP-FMT-003`)
#   SYSTEM.meta.json   <sha256>  — the sidecar (`SP-FMT-004`)
#   AGENT-LANE.md      <sha256>  — the agent lane (`SP-FMT-103`)
#   two-paths          same | DIFFER — the leg P7 already measured on Linux,
#                      re-measured here so one run answers both halves.
#
# The report is on **stdout**; `vibe install`'s progress goes to stderr, so
# `determinism-hashes.sh <dir> 2>/dev/null` prints the report alone. The exit
# status is 0 when the two paths agree and 1 when they do not.
#
# Hashing goes through `python3`, which `make-sample.sh` already requires, so
# every host hashes with the same implementation rather than with whichever
# of `sha256sum` / `shasum` / `certutil` it happens to carry.

set -eu

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VIBE="${VIBE:-vibe}"
TARGET="${1:?usage: determinism-hashes.sh <scratch-dir>}"
BOOT="vibevm/vibespecs/boot"

[ -e "$TARGET" ] && { echo "refusing: $TARGET exists" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "no python3 on PATH" >&2; exit 2; }

mkdir -p "$TARGET"
ROOT="$(cd "$TARGET" && pwd)"

VIBE="$VIBE" bash "$HERE/make-sample.sh" "$ROOT/a" >/dev/null
VIBE="$VIBE" bash "$HERE/make-sample.sh" "$ROOT/b" >/dev/null

echo "host:   $(uname -s) $(uname -m)"
python3 -I - "$HERE/sample" "$ROOT/a/$BOOT" "$ROOT/b/$BOOT" <<'PY'
import hashlib, pathlib, sys

source, first, second = (pathlib.Path(argument) for argument in sys.argv[1:4])

# The committed fixture's own identity: sorted relative paths and raw bytes,
# each length-prefixed so no concatenation of two trees can collide with a
# third.
digest = hashlib.sha256()
for path in sorted(p for p in source.rglob("*") if p.is_file()):
    relative = path.relative_to(source).as_posix().encode()
    body = path.read_bytes()
    for field in (relative, body):
        digest.update(len(field).to_bytes(8, "little"))
        digest.update(field)
print(f"inputs: {digest.hexdigest()}")

agree = True
for name in ("SYSTEM.md", "SYSTEM.meta.json", "AGENT-LANE.md"):
    a = hashlib.sha256((first / name).read_bytes()).hexdigest()
    b = hashlib.sha256((second / name).read_bytes()).hexdigest()
    agree = agree and a == b
    print(f"{name:<18} {a}")
print(f"two-paths: {'same' if agree else 'DIFFER'}")
sys.exit(0 if agree else 1)
PY
