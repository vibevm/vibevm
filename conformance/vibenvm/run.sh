#!/usr/bin/env bash
# run.sh — the vibenvm differential conformance suite. Its verdict is its
# exit code, and no part of it reads or writes a model's opinion
# (PROP-075 `#VIBENVM-ALGORITHMIC-PROOF`, `#VIBENVM-SUITE`).
#
# It runs the same command sequences, request for request, against the pinned
# upstream nvm-sh (sandbox A) and against vibe's `vibenvm` shim (sandbox B),
# each in its own isolated home and shell, from the same starting state and
# the same local mirror of Node release files, and compares the observation
# record after every step, field by field.
#
# usage: run.sh [options]
#   --cases GLOB          run only the cases whose file name matches GLOB
#   --self-test           the suite's own meta-tests: sandbox A against
#                         itself over every case, which must pass, plus the
#                         fixtures that prove the verdict can fail (1, 2, 3)
#   --self-compare        both sandboxes are A (the --self-test comparison,
#                         available on its own for one case at a time)
#   --perturb FIELD       deliberately corrupt FIELD in the second sandbox's
#                         records; only the meta-tests use this
#   --vibe PATH           the vibe binary for sandbox B
#                         (default: $VIBE_BIN, else <repo>/target/debug/vibe)
#   --nvm-registry P      a local registry path publishing
#                         org.vibevm.nvm/nvm (default: $VIBENVM_REGISTRY)
#   --lock PATH           read the mirror pin from PATH instead of mirror.lock
#   --work DIR            where the per-case sandboxes are built
#                         (default: $VIBENVM_TMP, else $TMPDIR, else /tmp)
#   --keep                keep the work directory and say where it is
#   --list                list the cases that would run, and exit
#
# exit:  0  every case passed and coverage is complete
#        1  a differential mismatch — the case, the step, the field, both values
#        2  coverage is incomplete, or the exclusion list is not the owner's
#        3  a harness error: a mirror digest mismatch, a missing pinned tag, a
#           missing prerequisite, a non-loopback connection attempt, or a
#           sandbox that could not be built (sandbox B needs the
#           `org.vibevm.nvm/nvm` package in a local registry)
#
# Linux only, and it says so: it needs `bwrap --unshare-net` for the network
# isolation and `strace` for the proof that nothing reached past loopback.
# It is not a cross-platform claim.

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
MIRROR="$HERE/mirror"
OBSERVE="$HERE/observe.sh"
RULES="$HERE/normalise.rules"
COVERAGE="$HERE/coverage.py"

EXIT_OK=0
EXIT_MISMATCH=1
EXIT_COVERAGE=2
EXIT_HARNESS=3

# A fixed minimal PATH, the same for both sandboxes: enough for curl, tar,
# xz, sed, awk and node itself, and nothing of the operator's.
SANDBOX_PATH="/usr/bin:/bin"

LOCK="${VIBENVM_LOCK:-$HERE/mirror.lock}"
CASE_GLOB="*.toml"
OBSERVE_FILES=""
META_N=0
SELF_TEST=0
SELF_COMPARE=0
PERTURB=""
KEEP=0
LIST=0
WORK_BASE="${VIBENVM_TMP:-${TMPDIR:-/tmp}}"
VIBE_BIN_ARG="${VIBE_BIN:-}"
VIBENVM_REGISTRY_ARG="${VIBENVM_REGISTRY:-}"

while [ $# -gt 0 ]; do
    case "$1" in
        --cases)
            CASE_GLOB="$2"
            shift 2
            ;;
        --self-test)
            SELF_TEST=1
            SELF_COMPARE=1
            shift
            ;;
        --self-compare)
            SELF_COMPARE=1
            shift
            ;;
        --perturb)
            PERTURB="$2"
            shift 2
            ;;
        --vibe)
            VIBE_BIN_ARG="$2"
            shift 2
            ;;
        --nvm-registry)
            VIBENVM_REGISTRY_ARG="$2"
            shift 2
            ;;
        --lock)
            LOCK="$2"
            shift 2
            ;;
        --work)
            WORK_BASE="$2"
            shift 2
            ;;
        --keep)
            KEEP=1
            shift
            ;;
        --list)
            LIST=1
            shift
            ;;
        -h | --help)
            sed -n '2,45p' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            echo "run.sh: unknown argument: $1" >&2
            exit "$EXIT_HARNESS"
            ;;
    esac
done

harness() {
    echo "run.sh: HARNESS ERROR: $1" >&2
    exit "$EXIT_HARNESS"
}

# ------------------------------------------------------- prerequisites ---

if [ -z "${VIBENVM_IN_SANDBOX:-}" ]; then
    missing=""
    for tool in bwrap strace python3 curl git tar sha256sum timeout sed awk; do
        command -v "$tool" >/dev/null 2>&1 || missing="$missing $tool"
    done
    [ -n "$missing" ] && harness "missing prerequisites:$missing"
    bwrap --dev-bind / / --unshare-net true 2>/dev/null \
        || harness "bwrap --unshare-net is not permitted here, so the suite cannot prove it stayed off the network"
fi

[ -f "$LOCK" ] || harness "missing mirror pin: $LOCK"
[ -f "$RULES" ] || harness "missing $RULES"
[ -x "$OBSERVE" ] || harness "missing or non-executable $OBSERVE"

lock_value() { awk -F'\t' -v k="$1" '$1==k {print $2; exit}' "$LOCK"; }

NVM_TAG="$(lock_value nvm_tag)"
NVM_TAG_OBJECT="$(lock_value nvm_tag_object)"
[ -n "$NVM_TAG" ] && [ -n "$NVM_TAG_OBJECT" ] \
    || harness "$LOCK names no nvm_tag / nvm_tag_object"

# --------------------------------------------------- the mirror's pin ----
#
# Verified before anything runs, and verified again from nothing but the
# lock: a suite that ran against a mirror nobody pinned would prove nothing.

verify_mirror() {
    [ -d "$MIRROR" ] \
        || harness "no local mirror at $MIRROR — run conformance/vibenvm/fetch-mirror.sh once (the only networked step)"
    [ -f "$MIRROR/nvm/nvm.sh" ] \
        || harness "no pinned nvm-sh tree at $MIRROR/nvm — run fetch-mirror.sh once"
    [ -d "$MIRROR/nvm-git/.git" ] \
        || harness "missing pinned tag: no clone at $MIRROR/nvm-git — run fetch-mirror.sh once"
    local got
    got="$(git -C "$MIRROR/nvm-git" rev-parse "$NVM_TAG" 2>/dev/null)"
    [ "$got" = "$NVM_TAG_OBJECT" ] \
        || harness "missing pinned tag: $NVM_TAG is ${got:-absent}, not $NVM_TAG_OBJECT"

    local rows=0 bad=0 rel want have
    while IFS=$'\t' read -r _ rel want; do
        rows=$((rows + 1))
        if [ ! -f "$MIRROR/$rel" ]; then
            echo "run.sh: mirror file missing: $rel" >&2
            bad=$((bad + 1))
            continue
        fi
        have="$(sha256sum "$MIRROR/$rel" | awk '{print $1}')"
        if [ "$have" != "$want" ]; then
            echo "run.sh: mirror digest mismatch: $rel" >&2
            echo "  pinned $want" >&2
            echo "  found  $have" >&2
            bad=$((bad + 1))
        fi
    done < <(grep $'^sha256\t' "$LOCK")
    [ "$rows" -gt 0 ] || harness "$LOCK carries no sha256 rows"
    [ "$bad" -eq 0 ] || harness "$bad of $rows mirrored files do not match ${LOCK##*/}"
    echo "mirror:     $rows files match ${LOCK##*/}; nvm-sh $NVM_TAG pinned at $NVM_TAG_OBJECT"
}

# ------------------------------------------------------------- cases -----

mapfile -t CASES < <(find "$HERE/cases" -maxdepth 1 -name "$CASE_GLOB" -type f 2>/dev/null | sort)
if [ "${#CASES[@]}" -eq 0 ]; then
    harness "no cases matched $CASE_GLOB under $HERE/cases"
fi
if [ "$LIST" = 1 ]; then
    printf '%s\n' "${CASES[@]##*/}"
    exit "$EXIT_OK"
fi

# ----------------------------------------------- the network isolation ---
#
# One re-exec into a network namespace that has only loopback, exactly as
# campaigns/sys-prompt-2026-11/accept.sh does it. Inside, every sandbox
# command also runs under `strace -e trace=connect`, so a connect(2) to
# anything but loopback fails the run even though it could not have
# succeeded. A promise is not a measurement.

if [ -z "${VIBENVM_IN_SANDBOX:-}" ]; then
    verify_mirror
    args=(--cases "$CASE_GLOB" --work "$WORK_BASE" --lock "$LOCK")
    [ "$SELF_TEST" = 1 ] && args+=(--self-test)
    [ "$SELF_TEST" = 0 ] && [ "$SELF_COMPARE" = 1 ] && args+=(--self-compare)
    [ -n "$PERTURB" ] && args+=(--perturb "$PERTURB")
    [ "$KEEP" = 1 ] && args+=(--keep)
    [ -n "$VIBE_BIN_ARG" ] && args+=(--vibe "$VIBE_BIN_ARG")
    [ -n "$VIBENVM_REGISTRY_ARG" ] && args+=(--nvm-registry "$VIBENVM_REGISTRY_ARG")
    bwrap --dev-bind / / --proc /proc --unshare-net --unshare-pid --die-with-parent \
        env VIBENVM_IN_SANDBOX=1 bash "${BASH_SOURCE[0]}" "${args[@]}"
    exit $?
fi

verify_mirror

WORK="$(mktemp -d "$WORK_BASE/vibenvm-conform.XXXXXX")" || harness "cannot create a work directory under $WORK_BASE"
if [ "$KEEP" = 1 ]; then
    trap 'echo "kept: $WORK"' EXIT
else
    trap 'rm -rf "$WORK"' EXIT
fi
mkdir -p "$WORK/results"

# ------------------------------------------------- the loopback mirror ---

SERVER_LOG="$WORK/mirror-server.log"
(
    cd "$MIRROR" || exit 1
    exec python3 -I -u -m http.server 0 --bind 127.0.0.1 >"$SERVER_LOG" 2>&1
) &
SERVER_PID=$!
MIRROR_URL=""
for _ in $(seq 1 100); do
    MIRROR_URL="$(sed -nE 's#.*Serving HTTP on 127\.0\.0\.1 port ([0-9]+).*#http://127.0.0.1:\1#p' "$SERVER_LOG" 2>/dev/null | head -1)"
    [ -n "$MIRROR_URL" ] && break
    sleep 0.1
done
[ -n "$MIRROR_URL" ] || harness "the loopback mirror server did not start: $(cat "$SERVER_LOG" 2>/dev/null)"
if [ "$KEEP" = 1 ]; then
    trap 'kill "$SERVER_PID" 2>/dev/null; echo "kept: $WORK"' EXIT
else
    trap 'kill "$SERVER_PID" 2>/dev/null; rm -rf "$WORK"' EXIT
fi
curl --silent --fail --max-time 5 --output /dev/null "$MIRROR_URL/dist/index.json" \
    || harness "the loopback mirror does not serve dist/index.json"
echo "mirror:     served over loopback at $MIRROR_URL"

# --------------------------------------------- the non-loopback guard ---

# Every traced connect(2) that is neither a unix socket, nor netlink, nor
# 127.0.0.0/8, nor ::1. Printed, not summarised: a reader has to be able to
# see what reached out.
suspect_connects() { # suspect_connects <trace-file>
    grep -E 'connect\(' "$1" 2>/dev/null \
        | grep -E 'AF_INET6?' \
        | grep -v 'inet_addr("127\.' \
        | grep -v '"::1"'
}

RUNNER_RC=0

# ------------------------------------------------------ case compiling --

# A case is TOML; the runner wants lines. One small reader turns the one
# into the other so nothing else in the suite has to parse TOML.
compile_case() { # compile_case <case-file> <out>
    python3 -I - "$1" >"$2" <<'PY'
import sys, tomllib
from pathlib import Path

path = Path(sys.argv[1])
with path.open("rb") as handle:
    case = tomllib.load(handle)
if case.get("schema") != 1:
    sys.exit(f"{path.name}: unsupported schema {case.get('schema')!r}")
rows = [
    ("name", str(case.get("name") or path.stem)),
    ("wall_secs", str(int(case.get("wall_secs", 900)))),
    ("compare_stdout", "1" if case.get("compare_stdout") else "0"),
]
for key in ("setup", "setup_a", "setup_b", "commands"):
    for line in case.get(key, []):
        rows.append((key, str(line)))
for observed in case.get("observe_files", []):
    rows.append(("observe_file", str(observed)))
if not case.get("commands"):
    sys.exit(f"{path.name}: a case with no commands proves nothing")
for key, value in rows:
    if "\n" in value:
        sys.exit(f"{path.name}: {key} must be one line: {value!r}")
    print(f"{key}\t{value}")
PY
}

case_field() { awk -F'\t' -v k="$2" '$1==k {print $2; exit}' "$1"; }
case_rows() { awk -F'\t' -v k="$2" '$1==k {print $2}' "$1"; }

# -------------------------------------------------- sandbox preparation --

# Sandbox A: the pinned nvm-sh, its own HOME and $NVM_DIR, pointed at the
# loopback mirror with `NVM_NODEJS_ORG_MIRROR` — the real path a closed
# network uses, not a test-only flag.
prepare_a() { # prepare_a <dir>
    local dir="$1"
    mkdir -p "$dir/project" "$dir/tmp" "$dir/.nvm" || return 1
    cp -a "$MIRROR/nvm/." "$dir/.nvm/" || return 1
}

# Sandbox B: its own HOME and `VIBE_SETTINGS`, a remap row that sends
# nodejs.org to the same loopback mirror (PROP-066 `#locator`), the local
# registry declared as this home's own, and the `org.vibevm.nvm/nvm` package installed
# from it.
prepare_b() { # prepare_b <dir>
    local dir="$1" vibe="$VIBE_BIN_ARG" registry="$VIBENVM_REGISTRY_ARG"
    mkdir -p "$dir/project" "$dir/tmp" "$dir/.vibe" "$dir/tool" || return 1

    cat >"$dir/.vibe/settings.toml" <<EOF
# Written by conformance/vibenvm/run.sh for sandbox B. The remap row is the
# mechanism a closed network uses anyway (PROP-066 #locator), so the suite
# exercises the real path rather than a test-only flag.
[[remap]]
prefix = "https://nodejs.org/dist"
url = "$MIRROR_URL/dist"
reason = "the vibenvm conformance suite's local Node release mirror"
EOF

    # The package first, and the binary after it: the package is what phase
    # P7f's shim half owes this suite, and a reader who sees "no vibe binary"
    # when the real blocker is a package that does not exist yet has been
    # sent to the wrong place.
    if [ -z "$registry" ]; then
        harness_b "sandbox B needs the package org.vibevm.nvm/nvm, and no registry was given: pass --nvm-registry PATH (or set VIBENVM_REGISTRY) pointing at a local registry that publishes it"
        return 1
    fi
    if [ ! -d "$registry/org.vibevm.nvm/nvm" ]; then
        harness_b "missing package org.vibevm.nvm/nvm in the registry at $registry — run conformance/vibenvm/make-registry.sh, which publishes it from its own repository"
        return 1
    fi
    [ -z "$vibe" ] && vibe="$REPO/target/debug/vibe"
    if [ ! -x "$vibe" ]; then
        harness_b "no vibe binary at $vibe (build it with \`cargo build -p vibe-cli\`, or pass --vibe PATH)"
        return 1
    fi

    # The registry as this home's own `[[registry]]`, not as a flag on one
    # command: the shim calls `vibe install -g org.vibevm.node/node@…`
    # exactly as a user's shell would, so the registry has to be configured
    # rather than passed. A `localdir` registry is the plain directory
    # layout `<group>/<name>/v<version>/` this suite publishes
    # (PROP-002 `#registry-type`), and it is a `file://` url, so resolution
    # reaches no host.
    cat >"$dir/.vibe/registry.toml" <<EOF
# Written by conformance/vibenvm/run.sh for sandbox B.
[[registry]]
name = "vibenvm-conformance"
type = "localdir"
url = "file://$registry"
ref = "main"
naming = "fqdn"
EOF

    # `vibe` on PATH, which is how the shim finds it and how a user has it.
    # The directory is deliberately **outside** the tool root, so it never
    # appears in the `owned_path` observation.
    ln -sf "$vibe" "$dir/tool/vibe" || return 1

    env -i HOME="$dir" PATH="$dir/tool:$SANDBOX_PATH" TMPDIR="$dir/tmp" \
        VIBE_SETTINGS="$dir/.vibe" \
        "$vibe" install -g org.vibevm.nvm/nvm \
        --registry "$registry" >"$dir/install.log" 2>&1 \
        || {
            harness_b "\`vibe install -g org.vibevm.nvm/nvm\` failed: $(tail -3 "$dir/install.log" | tr '\n' ' ')"
            return 1
        }
    # The *installed* copy, never the package cache's: both carry the same
    # bytes, and sourcing the cache would let a failed installation pass.
    local shim
    shim="$(find "$dir/.vibe/opt" -name 'nvm.sh' -type f 2>/dev/null | sort | head -1)"
    if [ -z "$shim" ]; then
        harness_b "the installed org.vibevm.nvm/nvm publishes no nvm.sh to source under $dir/.vibe/opt"
        return 1
    fi
    printf '%s\n' "$shim" >"$dir/tool-init"
}

# A sandbox that cannot be built is a harness error, never a mismatch and
# never a pass: the suite must not be able to report conformance it did not
# measure.
harness_b() {
    echo "run.sh: HARNESS ERROR (sandbox B): $1" >&2
    RUNNER_RC="$EXIT_HARNESS"
}

# ------------------------------------------------------- the driver -----

# One shell per sandbox per case, so the shell state a version manager
# exists to change — PATH above all — carries from step to step the way it
# does for a user. The observation is taken after every step inside that
# same shell.
write_driver() { # write_driver <compiled-case> <kind> <sandbox-dir> <out-dir> <driver-path>
    local compiled="$1" kind="$2" dir="$3" out="$4" driver="$5"
    local init
    if [ "$kind" = a ]; then
        init="$dir/.nvm/nvm.sh"
    else
        init="$(cat "$dir/tool-init")"
    fi
    {
        echo '#!/usr/bin/env bash'
        echo '# Generated by conformance/vibenvm/run.sh — one case, one shell.'
        echo "cd \"$dir/project\" || exit 90"
        echo "OBS=\"$OBSERVE\""
        echo "KIND=\"$kind\""
        echo "STEPS=\"$out\""
        echo ". \"$init\" --no-use >\"$out/source.out\" 2>\"$out/source.err\""
        echo 'echo "$?" >"$STEPS/source.status"'
        echo '__vibenvm_step() {'
        echo '  local __n="$1" __cmd="$2" __code'
        echo '  eval "$__cmd" >"$STEPS/step-$__n.out" 2>"$STEPS/step-$__n.err"'
        echo '  __code=$?'
        echo '  OBS_EXIT="$__code" OBS_STEP="$__n" OBS_COMMAND="$__cmd" \'
        echo '    "$OBS" "$KIND" "$STEPS/step-$__n.rec" || echo "observe failed" >&2'
        echo '}'
        local line
        while IFS= read -r line; do
            printf '%s\n' "$line"
            printf '%s\n' 'echo "$?" >>"$STEPS/setup.status"'
        done < <(case_rows "$compiled" setup; case_rows "$compiled" "setup_$kind")
        local n=0
        while IFS= read -r line; do
            n=$((n + 1))
            printf '__vibenvm_step %d %q\n' "$n" "$line"
        done < <(case_rows "$compiled" commands)
    } >"$driver"
}

# ------------------------------------------------------ running a step ---

run_driver() { # run_driver <kind> <sandbox-dir> <out-dir> <driver> <wall_secs> <perturb>
    local kind="$1" dir="$2" out="$3" driver="$4" wall="$5" perturb="$6"
    local trace="$out/connect.trace"
    # Each sandbox's PATH carries its own tool and nothing else: sandbox A
    # has nvm as a sourced function, sandbox B has `vibe` as a program, and
    # B's directory for it sits outside the tool root so it never shows up
    # in the `owned_path` observation.
    local sandbox_path="$SANDBOX_PATH"
    [ "$kind" = b ] && sandbox_path="$dir/tool:$SANDBOX_PATH"
    local -a env_common=(
        "HOME=$dir"
        "PATH=$sandbox_path"
        "TERM=dumb"
        "LANG=C"
        "LC_ALL=C"
        "TMPDIR=$dir/tmp"
        "OBS_RULES=$RULES"
        "OBS_HOME=$dir"
        "OBS_MIRROR=$MIRROR_URL"
        "OBS_FILES=$OBSERVE_FILES"
        # npm must never leave loopback either: its registry is a directory
        # of the local mirror, so a command that wants the registry gets a
        # deterministic 404 instead of a forbidden connection.
        "npm_config_registry=$MIRROR_URL/npm-registry/"
        "npm_config_audit=false"
        "npm_config_fund=false"
        "npm_config_update_notifier=false"
        "NO_UPDATE_NOTIFIER=1"
    )
    [ -n "$perturb" ] && env_common+=("VIBENVM_PERTURB_FIELD=$perturb")
    if [ "$kind" = a ]; then
        env_common+=(
            "NVM_DIR=$dir/.nvm"
            "OBS_TOOLROOT=$dir/.nvm"
            "NVM_NODEJS_ORG_MIRROR=$MIRROR_URL/dist"
            # nvm's default category set includes iojs, so `ls-remote` would
            # ask nodejs.org's io.js sibling if this were unset. It points at
            # the same loopback server, which holds no io.js index — the
            # listing is then deterministically empty rather than networked.
            "NVM_IOJS_ORG_MIRROR=$MIRROR_URL/iojs-dist"
        )
    else
        env_common+=(
            "VIBE_SETTINGS=$dir/.vibe"
            "OBS_TOOLROOT=$dir/.vibe"
            "VIBE_BIN=$VIBE_BIN_ARG"
        )
    fi
    timeout --signal=KILL "$wall" \
        env -i "${env_common[@]}" \
        strace -f -o "$trace" -e trace=connect \
        bash "$driver" >"$out/driver.out" 2>"$out/driver.err"
    local code=$?
    if [ "$code" = 137 ]; then
        echo "run.sh: sandbox $kind exceeded its ${wall}s budget" >&2
        return 2
    fi
    local suspect
    suspect="$(suspect_connects "$trace")"
    if [ -n "$suspect" ]; then
        echo "run.sh: HARNESS ERROR: a non-loopback connect(2) was attempted in sandbox $kind:" >&2
        printf '%s\n' "$suspect" >&2
        return 3
    fi
    return 0
}

# ------------------------------------------------------- comparing -------

# Field by field, step by step. Fields whose name starts with `_` are the
# reader's, not the comparison's.
compare_records() { # compare_records <rec-left> <rec-right>
    awk -F'\t' -v OFS='\t' -v rules="$RULES" '
        function sort_members(value) {
            sep = (index(value, ";") > 0) ? ";" : ","
            n = split(value, parts, sep)
            for (i = 2; i <= n; i++) {
                hold = parts[i]
                for (j = i - 1; j >= 1 && parts[j] > hold; j--) parts[j+1] = parts[j]
                parts[j+1] = hold
            }
            out = parts[1]
            for (i = 2; i <= n; i++) out = out sep parts[i]
            return out
        }
        BEGIN {
            while ((getline line < rules) > 0) {
                split(line, row, "\t")
                if (row[1] == "sort") setfield[row[2]] = 1
            }
        }
        NR == FNR {
            if ($1 ~ /^_/) next
            left[$1] = ($1 in setfield) ? sort_members($2) : $2
            order[++count] = $1
            next
        }
        {
            if ($1 ~ /^_/) next
            right[$1] = ($1 in setfield) ? sort_members($2) : $2
        }
        END {
            for (i = 1; i <= count; i++) {
                key = order[i]
                have = (key in right) ? right[key] : "<field absent>"
                if (left[key] != have) print key, left[key], have
            }
            for (key in right) if (!(key in left)) print key, "<field absent>", right[key]
        }
    ' "$1" "$2"
}

# The stdout/stderr of a step, normalised — only for a case that asked for
# its output to be compared (`compare_stdout = true`).
normalise_stream() { # normalise_stream <sed-script> <drop-script> <file>
    sed -E -f "$1" "$3" | sed -E -f "$2"
}

build_rules_scripts() { # build_rules_scripts <sandbox-dir> <kind> <out-prefix>
    local dir="$1" kind="$2" prefix="$3" toolroot
    if [ "$kind" = a ]; then toolroot="$dir/.nvm"; else toolroot="$dir/.vibe"; fi
    awk -F'\t' -v home="$dir" -v toolroot="$toolroot" -v mirror="$MIRROR_URL" \
        -v tmp="$dir/tmp" '
        $1=="value" {
          rule=$2
          gsub(/@HOME@/, home, rule); gsub(/@TOOLROOT@/, toolroot, rule)
          gsub(/@MIRROR@/, mirror, rule); gsub(/@TMPDIR@/, tmp, rule)
          print rule
        }' "$RULES" >"$prefix.sed"
    awk -F'\t' '$1=="drop" {printf "\\%%%s%%d\n", $2}' "$RULES" >"$prefix.drop"
}

# --------------------------------------------------------- one case ------

PASSED=0
FAILED=0
MISMATCHES=0

run_case() { # run_case <case-file> <kind-left> <kind-right>
    local file="$1" left_kind="$2" right_kind="$3"
    local base compiled name wall stdout_compare
    base="$(basename "$file" .toml)"
    compiled="$WORK/$base.compiled"
    if ! compile_case "$file" "$compiled"; then
        harness "cannot read case $file"
    fi
    name="$(case_field "$compiled" name)"
    wall="$(case_field "$compiled" wall_secs)"
    stdout_compare="$(case_field "$compiled" compare_stdout)"
    OBSERVE_FILES="$(case_rows "$compiled" observe_file)"

    local root="$WORK/cases/$base"
    local left="$root/left" right="$root/right"
    mkdir -p "$left" "$right" "$root/out-left" "$root/out-right"

    local prepared=1
    for side in left right; do
        local dir kind
        if [ "$side" = left ]; then
            dir="$left"
            kind="$left_kind"
        else
            dir="$right"
            kind="$right_kind"
        fi
        if [ "$kind" = a ]; then
            prepare_a "$dir" || {
                harness "cannot prepare sandbox A at $dir"
            }
        else
            prepare_b "$dir" || prepared=0
        fi
    done
    if [ "$prepared" = 0 ]; then
        printf 'status\t%s\n' "harness" >"$WORK/results/$base.tsv"
        case_rows "$compiled" commands \
            | while IFS= read -r line; do printf 'command\t%s\n' "$line"; done \
                >>"$WORK/results/$base.tsv"
        return "$EXIT_HARNESS"
    fi

    write_driver "$compiled" "$left_kind" "$left" "$root/out-left" "$root/driver-left.sh"
    write_driver "$compiled" "$right_kind" "$right" "$root/out-right" "$root/driver-right.sh"

    local rc=0
    run_driver "$left_kind" "$left" "$root/out-left" "$root/driver-left.sh" "$wall" "" || rc=$?
    if [ "$rc" = 3 ]; then return "$EXIT_HARNESS"; fi
    run_driver "$right_kind" "$right" "$root/out-right" "$root/driver-right.sh" "$wall" "$PERTURB" || {
        local rc2=$?
        [ "$rc2" = 3 ] && return "$EXIT_HARNESS"
        rc=$rc2
    }

    build_rules_scripts "$left" "$left_kind" "$root/left-rules"
    build_rules_scripts "$right" "$right_kind" "$root/right-rules"

    # Every step of the case, compared.
    local steps diffs=0 step
    steps="$(case_rows "$compiled" commands | wc -l)"
    for step in $(seq 1 "$steps"); do
        local recl="$root/out-left/step-$step.rec" recr="$root/out-right/step-$step.rec"
        local command
        command="$(case_rows "$compiled" commands | sed -n "${step}p")"
        if [ ! -f "$recl" ] || [ ! -f "$recr" ]; then
            echo "MISMATCH $name step $step [$command]" >&2
            echo "  record missing: left=$([ -f "$recl" ] && echo present || echo absent) right=$([ -f "$recr" ] && echo present || echo absent)" >&2
            diffs=$((diffs + 1))
            continue
        fi
        local out
        out="$(compare_records "$recl" "$recr")"
        if [ -n "$out" ]; then
            echo "MISMATCH $name step $step [$command]" >&2
            while IFS=$'\t' read -r field lv rv; do
                echo "  field $field" >&2
                echo "    $left_kind: $lv" >&2
                echo "    $right_kind: $rv" >&2
            done <<<"$out"
            diffs=$((diffs + 1))
        fi
        if [ "$stdout_compare" = 1 ]; then
            local stream
            for stream in out err; do
                local a_text b_text
                a_text="$(normalise_stream "$root/left-rules.sed" "$root/left-rules.drop" "$root/out-left/step-$step.$stream" 2>/dev/null)"
                b_text="$(normalise_stream "$root/right-rules.sed" "$root/right-rules.drop" "$root/out-right/step-$step.$stream" 2>/dev/null)"
                if [ "$a_text" != "$b_text" ]; then
                    echo "MISMATCH $name step $step [$command]" >&2
                    echo "  field std$stream (compare_stdout = true)" >&2
                    diff <(printf '%s\n' "$a_text") <(printf '%s\n' "$b_text") \
                        | sed 's/^/    /' >&2
                    diffs=$((diffs + 1))
                fi
            done
        fi
    done

    {
        if [ "$diffs" = 0 ] && [ "$rc" = 0 ]; then
            printf 'status\tpass\n'
        else
            printf 'status\tfail\n'
        fi
        case_rows "$compiled" commands \
            | while IFS= read -r line; do printf 'command\t%s\n' "$line"; done
    } >"$WORK/results/$base.tsv"

    if [ "$diffs" = 0 ] && [ "$rc" = 0 ]; then
        echo "pass: $name ($steps steps)"
        PASSED=$((PASSED + 1))
        return 0
    fi
    echo "FAIL: $name ($diffs of $steps steps differ)" >&2
    FAILED=$((FAILED + 1))
    MISMATCHES=$((MISMATCHES + diffs))
    return "$EXIT_MISMATCH"
}

# ----------------------------------------------------- the whole run -----

run_all() {
    local left_kind=a right_kind=b
    [ "$SELF_COMPARE" = 1 ] && right_kind=a
    echo "sandboxes:  $left_kind vs $right_kind"
    echo "cases:      ${#CASES[@]} matching $CASE_GLOB"
    local rc=0 one
    for one in "${CASES[@]}"; do
        local case_rc=0
        run_case "$one" "$left_kind" "$right_kind" || case_rc=$?
        if [ "$case_rc" = "$EXIT_HARNESS" ]; then
            return "$EXIT_HARNESS"
        fi
        [ "$case_rc" != 0 ] && rc="$EXIT_MISMATCH"
    done
    return "$rc"
}

# --------------------------------------------------- the meta-tests ------

# The suite's verdict is only worth its exit code if the exit code can fail.
# Each fixture below is a deliberate defect with an expected verdict, and
# every one of them runs inside the same network namespace as the real run.
expect() { # expect <wanted-exit> <label> <command...>
    local want="$1" label="$2"
    shift 2
    local got=0
    META_N=$((META_N + 1))
    local log="$WORK/meta-$META_N.log"
    "$@" >"$log" 2>&1 || got=$?
    if [ "$got" = "$want" ]; then
        echo "meta: ok   $label (exit $got)"
        return 0
    fi
    echo "meta: FAIL $label: expected exit $want, got $got" >&2
    sed 's/^/      /' "$WORK/meta-$want-$$.log" >&2
    return 1
}

self_test_meta() {
    local rc=0 cheap
    cheap="$(basename "${CASES[0]}")"
    for one in "${CASES[@]}"; do
        case "$one" in *help*) cheap="$(basename "$one")" ;; esac
    done
    local me="${BASH_SOURCE[0]}"

    # 1 — a deliberately wrong observation fails with exit 1, naming the field.
    expect "$EXIT_MISMATCH" "a perturbed field is a mismatch" \
        bash "$me" --self-compare --cases "$cheap" --work "$WORK_BASE" \
        --lock "$LOCK" --perturb node_v || rc=1

    # 2 — a pair no case covers fails the coverage gate with exit 2.
    python3 -I - "$HERE/inventory.json" "$WORK/inventory.extra.json" <<'PY' || rc=1
import json, sys
src, dest = sys.argv[1:]
with open(src, encoding="utf-8") as handle:
    inventory = json.load(handle)
inventory["pairs"].append({"subcommand": "install", "flag": "--no-such-flag"})
inventory["pair_count"] = len(inventory["pairs"])
with open(dest, "w", encoding="utf-8") as handle:
    json.dump(inventory, handle)
PY
    expect "$EXIT_COVERAGE" "an uncovered pair fails coverage" \
        python3 -I "$COVERAGE" --inventory "$WORK/inventory.extra.json" \
        --exclusions "$HERE/exclusions.toml" --results "$WORK/results" || rc=1

    # 3 — an exclusion row that nobody approved fails the coverage gate.
    sed 's/^approved_by = "owner"$/approved_by = "a worker"/' \
        "$HERE/exclusions.toml" >"$WORK/exclusions.unapproved.toml"
    expect "$EXIT_COVERAGE" "an unapproved exclusion row fails coverage" \
        python3 -I "$COVERAGE" --inventory "$HERE/inventory.json" \
        --exclusions "$WORK/exclusions.unapproved.toml" --results "$WORK/results" || rc=1

    # 4 — a row beyond the owner's closed list fails the coverage gate.
    {
        cat "$HERE/exclusions.toml"
        printf '\n[[exclusion]]\nitem = "a worker widened the list"\n'
        printf 'subcommand = "use"\nflag = "--silent"\n'
        printf 'reason = "a row the owner never approved"\n'
        printf 'approved_by = "owner"\napproved = "2026-11-10"\n'
    } >"$WORK/exclusions.thirteenth.toml"
    expect "$EXIT_COVERAGE" "an extra exclusion row fails coverage" \
        python3 -I "$COVERAGE" --inventory "$HERE/inventory.json" \
        --exclusions "$WORK/exclusions.thirteenth.toml" --results "$WORK/results" || rc=1

    # 5 — a tampered mirror.lock fails the run with exit 3.
    awk -F'\t' -v OFS='\t' '
        $1=="sha256" && !done {$3="0000000000000000000000000000000000000000000000000000000000000000"; done=1}
        {print}' "$LOCK" >"$WORK/mirror.lock.tampered"
    expect "$EXIT_HARNESS" "a tampered mirror.lock is a harness error" \
        bash "$me" --self-compare --cases "$cheap" --work "$WORK_BASE" \
        --lock "$WORK/mirror.lock.tampered" || rc=1

    # 6 — a non-loopback connect(2) attempt is a harness error. The attempt
    # is made for real, inside this namespace, where it cannot succeed —
    # what is proven is that the guard sees it.
    mkdir -p "$WORK/guard"
    env -i PATH="$SANDBOX_PATH" HOME="$WORK/guard" \
        strace -f -o "$WORK/guard/connect.trace" -e trace=connect \
        curl --silent --max-time 3 --output /dev/null http://93.184.216.34/ \
        >/dev/null 2>&1
    if [ -n "$(suspect_connects "$WORK/guard/connect.trace")" ]; then
        echo "meta: ok   a non-loopback connect(2) is seen by the guard"
    else
        echo "meta: FAIL the guard did not see a non-loopback connect(2)" >&2
        rc=1
    fi
    return "$rc"
}

# ------------------------------------------------------- the verdict -----

echo "work:       $WORK"
run_all
RUN_RC=$?
if [ "$RUN_RC" = "$EXIT_HARNESS" ] || [ "$RUNNER_RC" = "$EXIT_HARNESS" ]; then
    echo "verdict:    3 (harness error — nothing was proven)" >&2
    exit "$EXIT_HARNESS"
fi

COVERAGE_RC=0
python3 -I "$COVERAGE" --inventory "$HERE/inventory.json" \
    --exclusions "$HERE/exclusions.toml" --results "$WORK/results" || COVERAGE_RC=$?
if [ "$COVERAGE_RC" = 3 ]; then
    echo "verdict:    3 (harness error in the coverage gate)" >&2
    exit "$EXIT_HARNESS"
fi

if [ "$SELF_TEST" = 1 ]; then
    META_RC=0
    self_test_meta || META_RC=$?
    if [ "$META_RC" != 0 ]; then
        echo "verdict:    3 (a meta-test did not behave as it must)" >&2
        exit "$EXIT_HARNESS"
    fi
fi

if [ "$RUN_RC" != 0 ]; then
    echo "verdict:    1 ($FAILED case(s) differ, $MISMATCHES step mismatch(es))" >&2
    exit "$EXIT_MISMATCH"
fi
if [ "$COVERAGE_RC" != 0 ]; then
    echo "verdict:    2 (coverage incomplete)" >&2
    exit "$EXIT_COVERAGE"
fi
echo "verdict:    0 ($PASSED case(s) passed, coverage complete)"
exit "$EXIT_OK"
