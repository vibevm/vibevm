#!/usr/bin/env bash
# discipline/golden/capture.sh — Phase −1 characterization capture
# (PLAYBOOK-TERRAFORM-VIBEVM v0.2 Phase −1; BROWNFIELD-PROTOCOL §6).
#
# Regenerates every golden transcript deterministically from the current
# tree. Run it twice; `git diff discipline/golden` must be empty — that is
# the inventory's determinism check.
#
# Scope: hermetic fixture-driven flows only (no network, no live
# registries). The `manual-tests/` live recipes are deliberately NOT
# captured: they hit github.com / gitverse.ru and are non-deterministic;
# their health is tracked by debt DBT-0002 / DBT-0005 instead.
#
# Normalization of volatile fields (documented contract):
#   1. every backslash becomes a forward slash (Windows runner);
#   2. the per-flow sandbox dir   -> <SANDBOX>
#   3. the repository root        -> <REPO>
#   4. vibe.lock `generated_at = "…"` -> "<TIMESTAMP>" (the lockfile
#      records install wall-clock time);
#   5. the sandbox project dir is always named `golden-proj`, because
#      `vibe init` derives the project name from the directory basename.
# Nothing else is rewritten. Transcripts were captured on Windows; the
# byte-reproducibility claim is per-machine-class, not cross-OS.
set -u

REPO_SH="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$REPO_SH/discipline/golden"
FIX_SH="$REPO_SH/fixtures/registry"

if command -v cygpath >/dev/null 2>&1; then
  REPO_M="$(cygpath -m "$REPO_SH")"
  FIX_M="$(cygpath -m "$FIX_SH")"
else
  REPO_M="$REPO_SH"
  FIX_M="$FIX_SH"
fi

# Build once; call the binary directly so cargo emits no noise into
# transcripts.
cargo build -q -p vibe-cli --manifest-path "$REPO_SH/Cargo.toml"
VIBE="$REPO_SH/target/debug/vibe"

SB_SH=""; SB_M=""

norm() {
  tr '\\' '/' | sed -e "s|$SB_M|<SANDBOX>|g" -e "s|$SB_SH|<SANDBOX>|g" \
                    -e "s|$FIX_M|<REPO>/fixtures/registry|g" \
                    -e "s|$FIX_SH|<REPO>/fixtures/registry|g" \
                    -e "s|$REPO_M|<REPO>|g" -e "s|$REPO_SH|<REPO>|g" \
                    -e 's|generated_at = "[^"]*"|generated_at = "<TIMESTAMP>"|'
}

new_sandbox() {
  SB_SH="$(mktemp -d)/golden-proj"
  mkdir -p "$SB_SH"
  if command -v cygpath >/dev/null 2>&1; then
    SB_M="$(cygpath -m "$SB_SH")"
  else
    SB_M="$SB_SH"
  fi
}

# step <transcript> <argv...> — run one CLI step inside the sandbox,
# append command line, exit code, stdout, stderr to the transcript.
step() {
  local t="$1"; shift
  local so se rc
  so="$(mktemp)"; se="$(mktemp)"
  ( cd "$SB_SH" && "$VIBE" "$@" >"$so" 2>"$se" ); rc=$?
  { printf '## $ vibe'; printf ' %s' "$@"; printf '\n'; } | norm >>"$t"
  {
    printf 'exit: %s\n' "$rc"
    printf '### stdout\n```\n'; norm <"$so"; printf '```\n'
    printf '### stderr\n```\n'; norm <"$se"; printf '```\n'
  } >>"$t"
  rm -f "$so" "$se"
}

tree_of() {  # final file tree of the sandbox
  local t="$1"
  {
    printf '## final file tree\n```\n'
    ( cd "$SB_SH" && find . -type f | sort )
    printf '```\n'
  } >>"$t"
}

file_of() {  # embed one key file (normalized)
  local t="$1" f="$2"
  {
    printf '## file: %s\n```\n' "$f"
    if [ -f "$SB_SH/$f" ]; then norm <"$SB_SH/$f"; else printf '<absent>\n'; fi
    printf '```\n'
  } >>"$t"
}

begin() {  # begin <name> <description>
  local t="$OUT/$1.transcript.md"
  printf '# golden flow: %s\n%s\n\n' "$1" "$2" >"$t"
  echo "$t"
}

# ---- flow: init ------------------------------------------------------
new_sandbox
T="$(begin init 'vibe init in an empty directory — the scaffold a fresh project gets.')"
step "$T" init --path .
tree_of "$T"
file_of "$T" vibe.toml
file_of "$T" spec/boot/INDEX.md
file_of "$T" CLAUDE.md

# ---- flow: install-qualified ----------------------------------------
new_sandbox
T="$(begin install-qualified 'vibe init, then install a fully-qualified pkgref from the hermetic fixture registry (LocalRegistry path).')"
step "$T" init --path .
step "$T" install org.vibevm/wal --registry "$FIX_M" --assume-yes
tree_of "$T"
file_of "$T" vibe.toml
file_of "$T" vibe.lock
file_of "$T" spec/boot/INDEX.md

# ---- flow: install-short-name ---------------------------------------
new_sandbox
T="$(begin install-short-name 'vibe init, then install by bare short name — exercises the PROP-008 Phase 5 short-name resolution boundary.')"
step "$T" init --path .
step "$T" install wal --registry "$FIX_M" --assume-yes
file_of "$T" vibe.toml
file_of "$T" vibe.lock

# ---- flow: check-installed ------------------------------------------
new_sandbox
T="$(begin check-installed 'vibe check over a freshly initialised + installed project — what a clean checkup looks like.')"
step "$T" init --path .
step "$T" install org.vibevm/wal --registry "$FIX_M" --assume-yes
step "$T" check --path .
step "$T" check --path . --quiet

# ---- flow: uninstall -------------------------------------------------
new_sandbox
T="$(begin uninstall 'install then uninstall — the slot, lockfile and manifest must come back clean.')"
step "$T" init --path .
step "$T" install org.vibevm/wal --registry "$FIX_M" --assume-yes
step "$T" uninstall org.vibevm/wal --assume-yes
tree_of "$T"
file_of "$T" vibe.toml
file_of "$T" vibe.lock

echo "golden capture complete: $OUT"
