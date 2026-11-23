#!/usr/bin/env bash
# vibevm user-home tripwire — proves a test run did not touch the
# developer's real per-user settings home (`~/.vibe`, or `$VIBE_SETTINGS`)
# nor their real shell startup files.
#
# The distrust this answers is concrete. The settings home carries the
# operator's publish tokens and API keys next to `config.toml`,
# `settings.toml` and `registry.toml`. A test that writes there is a bug
# in the test — and three findings in a row (F-055/F-056/F-057) were
# exactly that, each caught by accident rather than by a gate. This makes
# it a gate.
#
# The shell startup files are the second, later-found surface. `vibe self
# install`, `self import --use --yes` and `self doctor --fix --yes` all
# write a managed block into the rc file under `$HOME` (PROP-019 `#path`),
# and `vibe app` env steps write their own block into the same files
# (PROP-065 `#env-step`). On 2026-11-23 a test that pinned
# `VIBEVM_INSTALL_ROOT` but not `HOME` rewrote the developer's real
# `~/.bashrc` to point `PATH` and `VIBEVM_HOME` at a temp directory that
# then vanished. Those paths live beside the settings home, not inside it,
# so the original snapshot could not see it happen.
#
#   user-home-tripwire.sh snapshot <out-file>
#       Record every path under the settings home plus a content hash,
#       and the shell startup files beside it.
#   user-home-tripwire.sh compare <baseline-file>
#       Re-snapshot and diff against the baseline. Exit 1 if anything
#       moved, naming the paths and what to do about it.
#
# Nothing invokes this: `tools/self-check.sh` runs the suite, the operator
# brackets a full run with `snapshot` and `compare`. Both verbs read only
# this script's own arguments, so widening what is snapshotted needs no
# change at the call site.
#
# SECRET HYGIENE — load-bearing, do not relax:
#   this script never reads a file's bytes into its output. It emits a
#   SHA-256 and a path, nothing else — and that holds for the startup
#   files just as it does for the settings home. A tripwire that leaks
#   what it guards is worse than no tripwire.
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

# The user's own home — `$HOME` then `$USERPROFILE`, and deliberately NOT
# `$VIBE_SETTINGS`, which relocates the settings home only. This is the
# directory `dirs::home_dir()` hands the CLI, and so the one whose startup
# files a durable-environment write actually lands in.
resolve_user_home() {
  if [ -n "${HOME:-}" ]; then
    printf '%s\n' "$HOME"
    return 0
  fi
  if [ -n "${USERPROFILE:-}" ]; then
    printf '%s\n' "$USERPROFILE"
    return 0
  fi
  printf '\n'
}

# The startup files a vibe run can write, relative to the user's home:
# every rc `Shell::rc_path` selects (`crates/vibe-cli/src/shell_syntax.rs`)
# and every file `vibe-app`'s `startup_files` touches
# (`crates/vibe-app/src/environment/blocks.rs`), plus the two login-shell
# profiles those get sourced from.
PROFILE_FILES="
.bashrc
.profile
.bash_profile
.zshrc
.zprofile
.config/fish/config.fish
"

# Emit one `profile <relpath> <sha256>` record per startup file that
# exists. An absent file emits nothing, so it appearing during the run
# reads as `+` and it vanishing as `-`.
#
# `sha256sum` follows a symbolic link, which is what we want: a dotfiles
# repo symlinked into place is still guarded by its content.
snapshot_profiles() {
  local out="$1" home="$2" path hash
  printf '# profiles: %s\n' "$home" >>"$out"
  [ -n "$home" ] || return 0
  for relative in $PROFILE_FILES; do
    path="$home/$relative"
    [ -f "$path" ] || continue
    hash="$(sha256sum -- "$path" 2>/dev/null | cut -d' ' -f1)"
    [ -n "$hash" ] || continue
    printf 'profile %s %s\n' "$relative" "$hash" >>"$out"
  done
}

# Write a snapshot of the settings home and the startup files to $1.
#
# Format (line-oriented, sorted, stable):
#   # home: <abs path>
#   # state: unresolved | absent | unreadable | present
#   # profiles: <abs path of the user's home>
#   dir  <relpath>
#   file <relpath> <sha256>
#   link <relpath>
#   profile <relpath> <sha256>
#
# Directories are recorded too: a test that mints an empty registry-cache
# bucket under the real home moved it, even though no file changed.
snapshot() {
  local out="$1"
  local home
  home="$(resolve_home)"

  printf '# vibevm user-home tripwire snapshot v2\n' >"$out"
  printf '# home: %s\n' "$home" >>"$out"

  # Before the settings-home branches below: the startup files sit beside
  # that home, so they are recorded even when it is absent or unreadable.
  snapshot_profiles "$out" "$(resolve_user_home)"

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

# The settings-home records and the startup-file records are compared
# separately: the first is gated on the settings home being resolvable and
# readable, the second is not.
settings_records() {
  records "$1" | grep -v '^profile ' || true
}

profile_records() {
  records "$1" | grep '^profile ' || true
}

# The `+ / - / ~` delta between two record streams, sorted by path.
delta() {
  awk '
    NR == FNR { before[$1 " " $2] = $0; next }
    {
      key = $1 " " $2
      if (!(key in before)) { printf "    + %s %s\n", $1, $2; next }
      if (before[key] != $0) { printf "    ~ %s %s (contents changed)\n", $1, $2 }
      delete before[key]
    }
    END { for (k in before) { split(k, p, " "); printf "    - %s %s\n", p[1], p[2] } }
  ' "$1" "$2" | sort -k2
}

state_of() {
  sed -n 's/^# state: //p' "$1" | head -1
}

home_of() {
  sed -n 's/^# home: //p' "$1" | head -1
}

profiles_home_of() {
  sed -n 's/^# profiles: //p' "$1" | head -1
}

version_of() {
  sed -n 's/^# vibevm user-home tripwire snapshot //p' "$1" | head -1
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

  # The startup files first, and unconditionally: they are observable
  # whenever a home resolves, whatever state the settings home is in.
  if [ "$(version_of "$before")" != "v2" ]; then
    echo "user-home tripwire: WARNING — the baseline predates startup-file" \
      "coverage; shell profiles not compared this run" >&2
  else
    local profile_report
    profile_report="$(delta <(profile_records "$before") <(profile_records "$after"))"
    if [ -n "$profile_report" ]; then
      {
        echo "user-home tripwire: FIRED — a real shell startup file changed during this run."
        echo "  home: $(profiles_home_of "$after")"
        echo "  paths that moved (+ appeared, - vanished, ~ contents changed):"
        printf '%s\n' "$profile_report"
        cat <<'EOF'
  What this means: something in this run wrote a managed block into the
  operator's own login environment instead of an isolated one. `vibe self
  install`, `self import --use --yes` and `self doctor --fix --yes` edit the
  rc file under `$HOME` (PROP-019 `#path`), and `vibe app` env steps edit the
  same files (PROP-065 `#env-step`); every one of them resolves that file
  from `$HOME`.
  What to do:
    1. Find the test. It is one that pins `VIBEVM_INSTALL_ROOT`, a settings
       home or an app root but leaves `HOME` ambient — pinning the install
       root is NOT enough, and is exactly how the 2026-11-23 leak happened.
    2. Give its command builder an owned `HOME` (and `SHELL`, so the rc
       target is deterministic), the way `crates/vibe-cli/tests/vvm.rs` and
       `cli_self_profile.rs` do.
    3. Restore the startup file by hand — a leaked block points `PATH` and
       `VIBEVM_HOME` at a temp directory that no longer exists — then re-run.
  This gate is not advisory. These files are the operator's login shell.
EOF
      } >&2
      return 1
    fi
  fi

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
  report="$(delta <(settings_records "$before") <(settings_records "$after"))"

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
