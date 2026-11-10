#!/usr/bin/env bash
# fetch-mirror.sh — populate the local Node release mirror and the pinned
# nvm-sh checkout. THIS IS THE ONLY NETWORKED STEP OF THE SUITE
# (PROP-075 `#VIBENVM-SUITE`); `run.sh` itself makes no internet access at
# all and fails any run that attempts one.
#
# It reaches exactly two hosts: `nodejs.org` (the release index and the
# pinned versions' checksums and Linux x64 archives) and `github.com` (the
# pinned nvm-sh tag). It sends no credentials and writes nothing outside
# `conformance/vibenvm/mirror/`, which is git-ignored.
#
# usage: fetch-mirror.sh [--lock] [--force]
#          --lock   rewrite the `sha256` rows of mirror.lock from what was
#                   just fetched, instead of verifying against them. The
#                   pin rows (nvm tag, node versions) are never rewritten:
#                   they are the authored pin, from PROP-075 `#VIBENVM-PIN`.
#          --force  re-download files that are already present.
# exit:  0 fetched and verified; 2 a prerequisite is missing;
#        3 a digest or the pinned tag does not match the lock.
#
# What lands in `mirror/`:
#   nvm/                  the pinned nvm-sh tree (no .git), sandbox A's $NVM_DIR
#   nvm-git/              the clone itself, kept only as the pin's evidence
#   dist/index.json       the release index, FILTERED to the pinned versions
#   dist/index.tab        the same, in nvm's own tab format (header kept)
#   dist/v<V>/SHASUMS256.txt            upstream, verbatim
#   dist/v<V>/node-v<V>-linux-x64.tar.{xz,gz}
#   npm-registry/         an empty directory, deliberately: both sandboxes
#                         point `npm_config_registry` at it over loopback, so
#                         an npm command that wants the registry gets a
#                         deterministic 404 instead of a forbidden connect.
#   .upstream/            the unfiltered index files the filter reads
#
# The indexes are filtered, and that is a decision, not a shortcut: a
# mirror describes what the mirror has. It also keeps `mirror.lock`
# stable — an unfiltered `index.json` changes with every Node release and
# could never be pinned by digest, while the filtered one is a function of
# the pinned version set alone.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIRROR="$HERE/mirror"
LOCK="$HERE/mirror.lock"
NODE_DIST="https://nodejs.org/dist"
NVM_REPO="https://github.com/nvm-sh/nvm.git"

DO_LOCK=0
FORCE=0
for arg in "$@"; do
    case "$arg" in
        --lock) DO_LOCK=1 ;;
        --force) FORCE=1 ;;
        -h | --help)
            sed -n '2,40p' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            echo "fetch-mirror.sh: unknown argument: $arg" >&2
            exit 2
            ;;
    esac
done

die() {
    echo "fetch-mirror.sh: $1" >&2
    exit "${2:-3}"
}

for tool in curl git sha256sum python3 tar; do
    command -v "$tool" >/dev/null 2>&1 || die "missing prerequisite: $tool" 2
done
[ -f "$LOCK" ] || die "missing $LOCK" 2

# ------------------------------------------------------------- the pin ---

lock_value() { awk -F'\t' -v k="$1" '$1==k {print $2; exit}' "$LOCK"; }
lock_values() { awk -F'\t' -v k="$1" '$1==k {print $2}' "$LOCK"; }

NVM_TAG="$(lock_value nvm_tag)"
NVM_TAG_OBJECT="$(lock_value nvm_tag_object)"
NVM_COMMIT="$(lock_value nvm_commit)"
mapfile -t NODE_VERSIONS < <(lock_values node_version)

[ -n "$NVM_TAG" ] || die "mirror.lock names no nvm_tag"
[ -n "$NVM_TAG_OBJECT" ] || die "mirror.lock names no nvm_tag_object"
[ "${#NODE_VERSIONS[@]}" -gt 0 ] || die "mirror.lock names no node_version"

echo "pin: nvm-sh $NVM_TAG ($NVM_TAG_OBJECT)"
echo "pin: node ${NODE_VERSIONS[*]}"

# `npm-registry/` and `iojs-dist/` are deliberately empty: both sandboxes
# point npm's registry and nvm's io.js mirror at them over loopback, so a
# command that wants either gets a deterministic 404 rather than a connection
# the suite forbids.
mkdir -p "$MIRROR/dist" "$MIRROR/.upstream" "$MIRROR/npm-registry" \
    "$MIRROR/iojs-dist" || die "cannot create $MIRROR"

# ------------------------------------------------------- the nvm-sh pin ---

if [ "$FORCE" = 1 ]; then
    rm -rf "$MIRROR/nvm-git" "$MIRROR/nvm"
fi
if [ ! -d "$MIRROR/nvm-git/.git" ]; then
    echo "fetching nvm-sh $NVM_TAG ..."
    rm -rf "$MIRROR/nvm-git"
    git clone --quiet --depth 1 --branch "$NVM_TAG" "$NVM_REPO" "$MIRROR/nvm-git" \
        || die "cannot clone $NVM_REPO at $NVM_TAG"
fi

got_tag="$(git -C "$MIRROR/nvm-git" rev-parse "$NVM_TAG" 2>/dev/null)"
got_commit="$(git -C "$MIRROR/nvm-git" rev-parse "$NVM_TAG^{commit}" 2>/dev/null)"
[ "$got_tag" = "$NVM_TAG_OBJECT" ] \
    || die "pinned tag object mismatch: want $NVM_TAG_OBJECT, got ${got_tag:-<none>}"
if [ -n "$NVM_COMMIT" ] && [ "$got_commit" != "$NVM_COMMIT" ]; then
    die "pinned commit mismatch: want $NVM_COMMIT, got ${got_commit:-<none>}"
fi
echo "ok: tag object $got_tag, commit $got_commit"

# The tree sandbox A sources, without the clone's own .git: it is copied
# per case, so it stays small on purpose.
rm -rf "$MIRROR/nvm"
mkdir -p "$MIRROR/nvm"
git -C "$MIRROR/nvm-git" archive --format=tar "$NVM_TAG" \
    | tar -C "$MIRROR/nvm" -xf - || die "cannot export the pinned tree"

# ------------------------------------------------------ the node mirror ---

fetch() { # fetch <url> <dest>
    local url="$1" dest="$2"
    if [ -s "$dest" ] && [ "$FORCE" != 1 ]; then
        echo "have: ${dest#"$MIRROR"/}"
        return 0
    fi
    mkdir -p "$(dirname "$dest")"
    echo "get:  ${dest#"$MIRROR"/}"
    curl --fail --silent --show-error --location --retry 3 \
        --output "$dest.part" "$url" || die "cannot download $url"
    mv "$dest.part" "$dest"
}

fetch "$NODE_DIST/index.json" "$MIRROR/.upstream/index.json"
fetch "$NODE_DIST/index.tab" "$MIRROR/.upstream/index.tab"

# Filter both indexes down to the pinned set. The downloaded JSON is
# untrusted data, so the filter runs under `python3 -I` and is handed its
# input as arguments, never as code.
python3 -I - "$MIRROR/.upstream/index.json" "$MIRROR/dist/index.json" \
    "${NODE_VERSIONS[@]}" <<'PY' || die "cannot filter index.json"
import json, sys

src, dest, *wanted = sys.argv[1:]
want = set(wanted)
with open(src, encoding="utf-8") as fh:
    records = json.load(fh)
kept = [r for r in records if r.get("version") in want]
missing = want - {r["version"] for r in kept}
if missing:
    sys.exit("index.json lists none of: " + ", ".join(sorted(missing)))
with open(dest, "w", encoding="utf-8", newline="\n") as fh:
    json.dump(kept, fh, indent=1, sort_keys=True)
    fh.write("\n")
PY

python3 -I - "$MIRROR/.upstream/index.tab" "$MIRROR/dist/index.tab" \
    "${NODE_VERSIONS[@]}" <<'PY' || die "cannot filter index.tab"
import sys

src, dest, *wanted = sys.argv[1:]
want = set(wanted)
out = []
with open(src, encoding="utf-8") as fh:
    for i, line in enumerate(fh):
        if i == 0 or line.split("\t", 1)[0] in want:
            out.append(line.rstrip("\n"))
seen = {line.split("\t", 1)[0] for line in out[1:]}
missing = want - seen
if missing:
    sys.exit("index.tab lists none of: " + ", ".join(sorted(missing)))
with open(dest, "w", encoding="utf-8", newline="\n") as fh:
    fh.write("\n".join(out) + "\n")
PY

for v in "${NODE_VERSIONS[@]}"; do
    fetch "$NODE_DIST/$v/SHASUMS256.txt" "$MIRROR/dist/$v/SHASUMS256.txt"
    for ext in tar.xz tar.gz; do
        fetch "$NODE_DIST/$v/node-$v-linux-x64.$ext" \
            "$MIRROR/dist/$v/node-$v-linux-x64.$ext"
    done
done

# ---------------------------------------------------------- the digests ---

# Every mirrored file, in one sorted list, relative to mirror/. The
# clone and the unfiltered inputs are evidence and input, not mirror
# content: `nvm/` is pinned by the tag object above plus the two digests
# the lock carries for the files sandbox A actually sources.
mirror_files() {
    (
        cd "$MIRROR" || exit 1
        printf '%s\n' nvm/nvm.sh nvm/nvm-exec nvm/bash_completion
        find dist -type f ! -name '*.part' | sort
    )
}

digest_of() { sha256sum "$MIRROR/$1" | awk '{print $1}'; }

if [ "$DO_LOCK" = 1 ]; then
    tmp="$LOCK.new"
    grep -v $'^sha256\t' "$LOCK" >"$tmp" || die "cannot rewrite $LOCK"
    while IFS= read -r rel; do
        [ -f "$MIRROR/$rel" ] || die "missing mirrored file: $rel"
        printf 'sha256\t%s\t%s\n' "$rel" "$(digest_of "$rel")" >>"$tmp"
    done < <(mirror_files)
    mv "$tmp" "$LOCK"
    echo "locked: $(grep -c $'^sha256\t' "$LOCK") files in ${LOCK##*/}"
    exit 0
fi

bad=0
locked=0
while IFS=$'\t' read -r _ rel want; do
    locked=$((locked + 1))
    if [ ! -f "$MIRROR/$rel" ]; then
        echo "missing: $rel" >&2
        bad=$((bad + 1))
        continue
    fi
    got="$(digest_of "$rel")"
    if [ "$got" != "$want" ]; then
        echo "digest mismatch: $rel" >&2
        echo "  want $want" >&2
        echo "  got  $got" >&2
        bad=$((bad + 1))
    fi
done < <(grep $'^sha256\t' "$LOCK")

if [ "$locked" = 0 ]; then
    die "mirror.lock carries no sha256 rows — run fetch-mirror.sh --lock once"
fi
if [ "$bad" != 0 ]; then
    die "$bad of $locked mirrored files do not match mirror.lock"
fi
echo "ok: $locked mirrored files match ${LOCK##*/}"
exit 0
