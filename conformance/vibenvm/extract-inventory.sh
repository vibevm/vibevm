#!/usr/bin/env bash
# extract-inventory.sh — re-extract nvm's own command and flag inventory from
# the pinned tree, mechanically, into `inventory.json`.
#
# The inventory is the coverage gate's unit of account (PROP-075
# `#VIBENVM-SUITE`): every `(subcommand, flag)` pair it yields must be covered
# by a passing differential case or appear in `exclusions.toml` as an
# owner-approved row. It is extracted, never authored, so a later nvm release
# cannot pass unnoticed — a pair the re-extraction finds and no case covers
# fails the gate.
#
# Two sources, both from the pinned tree and nothing else:
#   1. the dispatch of the `nvm()` function in `nvm.sh` — the `case $COMMAND`
#      branch patterns, and the flag-shaped `case` patterns inside each branch
#      (that is where nvm parses its own flags), plus the pre-dispatch
#      argument loop that handles `-h` / `help` / `--help`;
#   2. the text `nvm --help` prints, captured by sourcing the pinned `nvm.sh`
#      in an isolated environment and running it. Flags nvm documents but
#      parses by string comparison rather than by a `case` pattern reach the
#      inventory this way.
#
# usage: extract-inventory.sh [--out PATH] [--print]
# exit:  0 wrote the inventory; 2 a prerequisite is missing;
#        3 the pinned tree is missing or does not match `mirror.lock`.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIRROR="$HERE/mirror"
LOCK="$HERE/mirror.lock"
OUT="$HERE/inventory.json"
PRINT=0

while [ $# -gt 0 ]; do
    case "$1" in
        --out)
            OUT="$2"
            shift 2
            ;;
        --print)
            PRINT=1
            shift
            ;;
        -h | --help)
            sed -n '2,25p' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            echo "extract-inventory.sh: unknown argument: $1" >&2
            exit 2
            ;;
    esac
done

die() {
    echo "extract-inventory.sh: $1" >&2
    exit "${2:-3}"
}

for tool in python3 git sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || die "missing prerequisite: $tool" 2
done
[ -f "$LOCK" ] || die "missing $LOCK" 2
[ -f "$MIRROR/nvm/nvm.sh" ] \
    || die "no pinned nvm tree at $MIRROR/nvm — run fetch-mirror.sh once"

lock_value() { awk -F'\t' -v k="$1" '$1==k {print $2; exit}' "$LOCK"; }
NVM_TAG="$(lock_value nvm_tag)"
NVM_TAG_OBJECT="$(lock_value nvm_tag_object)"
NVM_COMMIT="$(lock_value nvm_commit)"

# The pin, verified here too: an inventory extracted from an unpinned tree
# would be a gate over nothing.
if [ -d "$MIRROR/nvm-git/.git" ]; then
    got="$(git -C "$MIRROR/nvm-git" rev-parse "$NVM_TAG" 2>/dev/null)"
    [ "$got" = "$NVM_TAG_OBJECT" ] \
        || die "pinned tag object mismatch: want $NVM_TAG_OBJECT, got ${got:-<none>}"
else
    die "no pinned clone at $MIRROR/nvm-git — run fetch-mirror.sh once"
fi
want_sha="$(awk -F'\t' '$1=="sha256" && $2=="nvm/nvm.sh" {print $3; exit}' "$LOCK")"
got_sha="$(sha256sum "$MIRROR/nvm/nvm.sh" | awk '{print $1}')"
[ -n "$want_sha" ] || die "mirror.lock carries no digest for nvm/nvm.sh"
[ "$got_sha" = "$want_sha" ] || die "nvm/nvm.sh does not match mirror.lock"

# `nvm --help`, from the pinned tree itself. Isolated: its own NVM_DIR, no
# NVM_* or VIBE_* inherited, a fixed minimal PATH, and no network needed.
HELPTMP="$(mktemp -d "${TMPDIR:-/tmp}/vibenvm-help.XXXXXX")" || die "mktemp failed" 2
trap 'rm -rf "$HELPTMP"' EXIT
cp -a "$MIRROR/nvm/." "$HELPTMP/nvm/" 2>/dev/null || {
    mkdir -p "$HELPTMP/nvm"
    cp -a "$MIRROR/nvm/." "$HELPTMP/nvm/"
}
env -i HOME="$HELPTMP" PATH=/usr/bin:/bin TERM=dumb NVM_DIR="$HELPTMP/nvm" \
    bash -c '. "$NVM_DIR/nvm.sh" --no-use >/dev/null 2>&1; nvm --help --no-colors' \
    >"$HELPTMP/help.txt" 2>"$HELPTMP/help.err"
[ -s "$HELPTMP/help.txt" ] \
    || die "sourcing the pinned nvm.sh produced no --help text: $(tail -1 "$HELPTMP/help.err")"

python3 -I - "$MIRROR/nvm/nvm.sh" "$HELPTMP/help.txt" "$OUT" \
    "$NVM_TAG" "$NVM_TAG_OBJECT" "$NVM_COMMIT" "$got_sha" <<'PY' || die "extraction failed"
"""Scan the pinned nvm.sh and its --help text into the committed inventory.

Three scans, each one rule, all of them mechanical:

* the pre-dispatch argument loop — the first `case $i in` inside `nvm()`,
  whose alternatives are nvm's help spellings;
* the dispatch — `case $COMMAND in` at indent 2 inside `nvm()`. A branch
  header is a line of patterns at indent 4 ending in `)`; its first
  alternative is the canonical spelling and the rest are aliases of the
  same code path. Inside the branch, a line whose text before the first
  `)` splits on `|` into nothing but flag-shaped alternatives (and the
  globs `*`, `--`, `---*`) is a flag `case` pattern, and those
  alternatives are the branch's flags;
* the help text — a `  nvm <sub> …` line names a subcommand (with any
  flags spelled on that same line), and the `    --flag …` lines under it
  are its flags. Columns are separated by three or more spaces, which is
  how nvm's help is laid out.

A flag found for an alias spelling is recorded against the canonical one,
because the alias enters the same branch and the flags are parsed once.
Alias spellings therefore yield one bare pair each — the gate still asks
for a case per alias, which is what proves the alias exists at all.
"""

import json
import re
import sys

nvm_sh, help_txt, out_path, tag, tag_object, commit, sha = sys.argv[1:]

FLAG = re.compile(r"^--?[A-Za-z][A-Za-z0-9-]*(=\*)?$")
GLOB = re.compile(r"^(-{1,3}\*|\*|--)$")
WORD = re.compile(r"^--?[A-Za-z][A-Za-z0-9-]*$")

with open(nvm_sh, encoding="utf-8") as fh:
    lines = fh.read().split("\n")


def dequote(text):
    text = text.strip()
    if len(text) >= 2 and text[0] == text[-1] and text[0] in "'\"":
        return text[1:-1]
    return text.replace('"', "").replace("'", "")


def alternatives(candidate):
    """Split a `case` pattern into its dequoted alternatives, or None."""
    if any(ch in candidate for ch in "($;&`"):
        return None
    parts = [dequote(p) for p in candidate.split("|")]
    return parts if all(parts) else None


# ---- where the nvm() function is -------------------------------------
start = next(i for i, line in enumerate(lines) if line == "nvm() {")
end = next(i for i in range(start + 1, len(lines)) if lines[i] == "}")
body = range(start, end)

subcommands = {}  # canonical -> {"aliases": [...], "flags": set(), "sources": set()}
alias_of = {}  # alias -> canonical


def add_group(group, source):
    canonical = group[0]
    entry = subcommands.setdefault(
        canonical, {"aliases": [], "flags": set(), "sources": set()}
    )
    entry["sources"].add(source)
    for name in group[1:]:
        if name not in entry["aliases"]:
            entry["aliases"].append(name)
        alias_of[name] = canonical
    return canonical


def canonical_of(name):
    return alias_of.get(name, name)


def add_flag(name, flag, source):
    canonical = canonical_of(name)
    entry = subcommands.setdefault(
        canonical, {"aliases": [], "flags": set(), "sources": set()}
    )
    entry["flags"].add(flag)
    entry["sources"].add(source)


# ---- scan 1: the pre-dispatch argument loop --------------------------
loop = next(i for i in body if lines[i].strip() == 'for i in "$@"')
case_i = next(i for i in range(loop, end) if lines[i].strip() == "case $i in")
depth_done = False
for i in range(case_i + 1, end):
    stripped = lines[i].strip()
    if stripped == "esac":
        depth_done = True
    if depth_done:
        break
    head = stripped.split(")")[0]
    alts = alternatives(head) if ")" in stripped else None
    if not alts:
        continue
    group = [a for a in alts if WORD.match(a) or re.match(r"^[a-z][a-z-]*$", a)]
    if group:
        add_group(group, "dispatch")

# ---- scan 2: the dispatch and its per-branch flag patterns ----------
dispatch = next(i for i in body if lines[i] == "  case $COMMAND in")
dispatch_end = next(i for i in range(dispatch + 1, end) if lines[i] == "  esac")
branch = None
branch_header = re.compile(r"^    (?P<pats>[^ ].*)\)$")
for i in range(dispatch + 1, dispatch_end):
    line = lines[i]
    header = branch_header.match(line)
    if header:
        alts = alternatives(header.group("pats"))
        if alts is None:
            continue
        if any(GLOB.match(a) for a in alts):
            branch = None  # the `*)` fallback is not a subcommand
            continue
        branch = add_group(alts, "dispatch")
        continue
    if branch is None or ")" not in line:
        continue
    alts = alternatives(line.lstrip().split(")")[0])
    if not alts:
        continue
    if not all(FLAG.match(a) or GLOB.match(a) for a in alts):
        continue
    for alt in alts:
        if FLAG.match(alt):
            add_flag(branch, alt.removesuffix("=*"), "dispatch")

# ---- scan 3: the help text -------------------------------------------
with open(help_txt, encoding="utf-8") as fh:
    help_lines = fh.read().split("\n")

in_usage = False
current = None
for raw in help_lines:
    line = raw.rstrip()
    if line.strip() == "Usage:":
        in_usage = True
        continue
    if not in_usage:
        continue
    if line.strip() in ("Example:", "Note:"):
        break
    column = re.split(r" {3,}", line.strip())[0]
    if line.startswith("  nvm "):
        tokens = column.split()
        if len(tokens) < 2:
            continue
        current = tokens[1]
        if canonical_of(current) not in subcommands:
            add_group([current], "help")
        else:
            subcommands[canonical_of(current)]["sources"].add("help")
        for token in tokens[2:]:
            if token.startswith("-"):
                flag = token.split("=")[0]
                if WORD.match(flag):
                    add_flag(current, flag, "help")
    elif line.startswith("    -") and current:
        flag = column.split()[0].split("=")[0]
        if WORD.match(flag):
            add_flag(current, flag, "help")

# ---- the inventory ---------------------------------------------------
entries = []
pairs = []
for name in sorted(subcommands):
    entry = subcommands[name]
    flags = sorted(entry["flags"])
    entries.append(
        {
            "name": name,
            "aliases": sorted(entry["aliases"]),
            "flags": flags,
            "sources": sorted(entry["sources"]),
        }
    )
    pairs.append({"subcommand": name, "flag": ""})
    pairs.extend({"subcommand": name, "flag": flag} for flag in flags)
    for alias in sorted(entry["aliases"]):
        pairs.append({"subcommand": alias, "flag": ""})

pairs.sort(key=lambda p: (p["subcommand"], p["flag"]))

inventory = {
    "schema": 1,
    "generated_by": "conformance/vibenvm/extract-inventory.sh",
    "authority": "PROP-075 #VIBENVM-SUITE, #VIBENVM-PIN",
    "nvm_tag": tag,
    "nvm_tag_object": tag_object,
    "nvm_commit": commit,
    "nvm_sh_sha256": sha,
    "flag_spelling": (
        "flag names carry no `=` and no value: nvm.sh's `--lts` and `--lts=*` "
        "are one flag, `--lts`, and so is the help's `--lts=<LTS name>`"
    ),
    "pair_count": len(pairs),
    "subcommands": entries,
    "pairs": pairs,
}

with open(out_path, "w", encoding="utf-8", newline="\n") as fh:
    json.dump(inventory, fh, indent=2)
    fh.write("\n")
print(
    f"inventory: {len(entries)} subcommands, {len(pairs)} pairs -> {out_path}",
    file=sys.stderr,
)
PY

if [ "$PRINT" = 1 ]; then
    python3 -I -c '
import json, sys
inv = json.load(open(sys.argv[1], encoding="utf-8"))
for pair in inv["pairs"]:
    print(pair["subcommand"], pair["flag"])
' "$OUT"
fi
exit 0
