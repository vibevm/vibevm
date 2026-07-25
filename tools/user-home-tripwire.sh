#!/usr/bin/env bash
# vibevm user-home tripwire — proves a test run did not touch the
# developer's real per-user settings home (`~/.vibe`, or `$VIBE_SETTINGS`).
#
# The distrust this answers is concrete: that home carries the operator's
# publish tokens and API keys next to `config.toml`, `settings.toml` and
# `registry.toml`. A test that writes there is a bug in the test — and
# three findings in a row (F-055/F-056/F-057) were exactly that, each
# caught by accident rather than by a gate. This makes it a gate.
#
#   user-home-tripwire.sh snapshot <out-file>
#       Record every path under the settings home plus a content hash.
#   user-home-tripwire.sh compare <baseline-file>
#       Re-snapshot and diff against the baseline. Exit 1 if anything
#       moved, naming the paths and what to do about it.
#
# SECRET HYGIENE — load-bearing, do not relax:
#   this script never reads a file's bytes into its output. It emits a
#   SHA-256 and a path, nothing else. A tripwire that leaks what it
#   guards is worse than no tripwire.
#
# Error posture: an unresolvable or unreadable home is a WARNING and a
# pass, never a red. The tripwire must not become the thing that blocks
# work. Only an observed change is a failure.

set -u

usage() {
  sed -n '2,/^$/p' "$0" | sed 's/^#\s\?//'
}

# Mirror `vibe_core::settings::settings_dir()` exactly: `$VIBE_SETTINGS`
# verbatim when non-empty, else `<home>/.vibe`, where home is `HOME` then
# `USERPROFILE`. Any divergence here would snapshot a directory the CLI
# does not actually use, which is a tripwire that guards nothing.
resolve_home() {
  if [ -n "${VIBE_SETTINGS:-}" ]; then
    printf '%s\n' "$VIBE_SETTINGS"
    return 0
  fi
  if [ -n "${HOME:-}" ]; then
    printf '%s/.vibe\n' "$HOME"
    return 0
  fi
  if [ -n "${USERPROFILE:-}" ]; then
    printf '%s/.vibe\n' "$USERPROFILE"
    return 0
  fi
  printf '\n'
}

# Write a snapshot of the settings home to $1.
#
# Format (line-oriented, sorted, stable):
#   # home: <abs path>
#   # state: unresolved | absent | unreadable | present
#   dir  <relpath>
#   file <relpath> <sha256>
#   link <relpath>
#
# Directories are recorded too: a test that mints an empty registry-cache
# bucket under the real home moved it, even though no file changed.
snapshot() {
  local out="$1"
  local home
  home="$(resolve_home)"

  printf '# vibevm user-home tripwire snapshot v1\n' >"$out"
  printf '# home: %s\n' "$home" >>"$out"

  if [ -z "$home" ]; then
    printf '# state: unresolved\n' >>"$out"
    return 0
  fi
  if [ ! -d "$home" ]; then
    printf '# state: absent\n' >>"$out"
    return 0
  fi
  if ! find "$home" -maxdepth 0 >/dev/null 2>&1; then
    printf '# state: unreadable\n' >>"$out"
    return 0
  fi
  printf '# state: present\n' >>"$out"

  (
    cd "$home" || exit 0
    find . -mindepth 1 -type d 2>/dev/null | sed -e 's|^\./||' -e 's|^|dir  |'
    find . -mindepth 1 -type l 2>/dev/null | sed -e 's|^\./||' -e 's|^|link |'
    # Hash-and-path only. `sha256sum` prints "<hash>  <name>" (text) or
    # "<hash> *<name>" (binary) — two separator chars either way, so the
    # path starts at length(hash)+3 in both.
    find . -mindepth 1 -type f -print0 2>/dev/null |
      xargs -0 -r sha256sum -- 2>/dev/null |
      awk '{
        hash = $1
        path = substr($0, length(hash) + 3)
        sub(/^\.\//, "", path)
        printf "file %s %s\n", path, hash
      }'
  ) | sort >>"$out"
}

# Extract "kind<TAB>path" keys and "kind<TAB>path<TAB>hash" records so the
# comparison can tell "appeared" from "changed".
records() {
  grep -v '^#' "$1" 2>/dev/null || true
}

state_of() {
  sed -n 's/^# state: //p' "$1" | head -1
}

home_of() {
  sed -n 's/^# home: //p' "$1" | head -1
}

compare() {
  local before="$1"
  local after
  after="$(mktemp)" || {
    echo "user-home tripwire: WARNING — no temp file; skipping (pass)" >&2
    return 0
  }
  # shellcheck disable=SC2064
  trap "rm -f '$after'" RETURN

  snapshot "$after"

  local home before_state after_state
  home="$(home_of "$after")"
  before_state="$(state_of "$before")"
  after_state="$(state_of "$after")"

  if [ "$before_state" = "unresolved" ] || [ "$after_state" = "unresolved" ]; then
    echo "user-home tripwire: WARNING — no settings home resolvable" \
      "(no \$VIBE_SETTINGS, \$HOME or \$USERPROFILE); nothing to guard, passing" >&2
    return 0
  fi
  if [ "$before_state" = "unreadable" ] || [ "$after_state" = "unreadable" ]; then
    echo "user-home tripwire: WARNING — $home is not readable; passing" >&2
    return 0
  fi
  if [ "$before_state" = "absent" ] && [ "$after_state" = "absent" ]; then
    echo "user-home tripwire: no per-user settings home at $home — trivially green" >&2
    return 0
  fi

  local report
  report="$(
    awk '
      NR == FNR { before[$1 " " $2] = $0; next }
      {
        key = $1 " " $2
        if (!(key in before)) { printf "    + %s %s\n", $1, $2; next }
        if (before[key] != $0) { printf "    ~ %s %s (contents changed)\n", $1, $2 }
        delete before[key]
      }
      END { for (k in before) { split(k, p, " "); printf "    - %s %s\n", p[1], p[2] } }
    ' <(records "$before") <(records "$after") | sort -k2
  )"

  if [ -z "$report" ] && [ "$before_state" = "$after_state" ]; then
    return 0
  fi

  {
    echo "user-home tripwire: FIRED — the real per-user settings home changed during this run."
    echo "  home: $home"
    if [ "$before_state" != "$after_state" ]; then
      echo "  state: $before_state -> $after_state"
    fi
    echo "  paths that moved (+ appeared, - vanished, ~ contents changed):"
    if [ -n "$report" ]; then
      printf '%s\n' "$report"
    else
      echo "    (no per-path delta — the home itself appeared or vanished)"
    fi
    cat <<'EOF'
  What this means: something in this run read-modified-wrote the operator's
  real settings home instead of an isolated one. That home carries publish
  tokens and API keys; a test must never touch it.
  What to do:
    1. Find the test. `crates/vibe-test-support` isolates a test process at
       load time — a test binary that links it cannot reach the real home.
       A binary that does NOT link it is the likely culprit.
    2. Route it through `vibe_test_support::UserScratch` (or add
       `use vibe_test_support as _;` so the load-time isolator links in).
    3. Restore whatever moved by hand if it mattered, then re-run.
  This gate is not advisory. Do not add an exception list without recording
  why the path legitimately moves (§4 of DRIFT-020: none are known).
EOF
  } >&2
  return 1
}

case "${1:-}" in
  snapshot)
    [ $# -eq 2 ] || {
      usage >&2
      exit 2
    }
    snapshot "$2"
    ;;
  compare)
    [ $# -eq 2 ] || {
      usage >&2
      exit 2
    }
    compare "$2"
    ;;
  -h | --help)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
