#!/usr/bin/env bash
# Materialise the P9 live-check sample project into a scratch directory.
#
# It copies `live/sample/` out of the repository, points the fixture's
# registry at the copy, optionally pads the lane to a stated size, and runs
# `vibe install` so the three generated artifacts exist. It starts no agent
# and contacts nothing: `vibe install --offline` against a `file://` registry
# inside the copy.
#
# usage: make-sample.sh <scratch-dir> [pad-bytes]
#
#   <scratch-dir>  created; must not exist. Keep it outside the repository.
#   [pad-bytes]    when given, the carrier's boot source is padded with
#                  neutral filler until the composed lane is at least this
#                  many bytes, with the tail canary still the last line of
#                  that segment. Use 0 or omit for the short variant.
#
# After it finishes it prints the three artifacts' paths and sizes, the
# delivered lane's first and last marker lines, and the exact
# `vibe agent <launcher>` command to run from the project directory.
#
# env: VIBE=<path>   the vibe binary (default: `vibe` on PATH)

set -eu

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VIBE="${VIBE:-vibe}"
TARGET="${1:?usage: make-sample.sh <scratch-dir> [pad-bytes]}"
PAD="${2:-0}"
BOOT="vibevm/vibespecs/boot"

[ -e "$TARGET" ] && { echo "refusing: $TARGET exists" >&2; exit 2; }
command -v "$VIBE" >/dev/null 2>&1 || { echo "no vibe binary: $VIBE" >&2; exit 2; }

mkdir -p "$TARGET"
cp -r "$HERE/sample/." "$TARGET/"
PROJECT="$(cd "$TARGET" && pwd)"

# The fixture's registry URL is a placeholder, because an absolute path
# cannot be committed. Point it at the copy. The URL is built by `as_uri`,
# so a Windows path becomes `file:///C:/...` rather than the invalid
# `file://C:/...`; on POSIX it is the same `file:///...` as before.
python3 -I - "$PROJECT/vibe.toml" "$PROJECT/registry" <<'PY'
import pathlib, sys
manifest = pathlib.Path(sys.argv[1])
url = pathlib.Path(sys.argv[2]).resolve().as_uri()
text = manifest.read_text()
assert text.count("file://REGISTRY_PATH") == 1, "fixture registry placeholder"
manifest.write_text(text.replace("file://REGISTRY_PATH", url))
PY

# Padding, when asked for. It inflates the **first** lane segment, which is
# what makes the lane's own tail — the last line of `90-user.md`'s body, and
# the last `vibe:lane-end` marker after it — arrive behind that many bytes.
# Padding the last segment instead would move the tail canary and prove
# nothing about a slot that truncates.
if [ "$PAD" -gt 0 ]; then
    python3 -I - "$PROJECT/registry/org.live/carrier/v1.0.0/boot/main.md" "$PAD" <<'PY'
import pathlib, sys
source = pathlib.Path(sys.argv[1])
want = int(sys.argv[2])
filler = (
    "Neutral filler paragraph {n}. It carries no instruction and no personal\n"
    "data; it exists only to bring the composed lane to the size a real\n"
    "project's lane has, so the delivery question is asked at that size.\n\n"
)
parts = [source.read_text()]
n = 0
while sum(len(part) for part in parts) < want:
    n += 1
    parts.append(filler.format(n=n))
source.write_text("".join(parts))
print(f"padded carrier source to {source.stat().st_size} bytes")
PY
fi

(cd "$PROJECT" && "$VIBE" install --offline --assume-yes >/dev/null)

# The nested sub-project is a project of its own — not a workspace member —
# so it gets its own regeneration and therefore its own router block, with
# its own node id. That is exactly what row N1 tests: a harness's downward
# discovery concatenates this file, and its block must say it does not
# apply.
(cd "$PROJECT/nested" && "$VIBE" install --offline --assume-yes >/dev/null)

echo
echo "project: $PROJECT"
for name in SYSTEM.md SYSTEM.meta.json AGENT-LANE.md; do
    printf '  %-18s %s bytes\n' "$name" "$(wc -c <"$PROJECT/$BOOT/$name")"
done
echo
echo "first marker line:"
head -1 "$PROJECT/$BOOT/AGENT-LANE.md"
echo "last marker line (the one a session is asked to quote):"
tail -1 "$PROJECT/$BOOT/AGENT-LANE.md"
echo
echo "now run, from $PROJECT:"
echo "  vibe agent claude2"
