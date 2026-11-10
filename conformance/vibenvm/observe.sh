#!/usr/bin/env bash
# observe.sh — write one observation record for one step of one case, in one
# sandbox. This is the owner's observation list, mechanised (PROP-075
# `#VIBENVM-SUITE`): the record, not the output text, is what the suite
# compares, field by field.
#
# usage: observe.sh <a|b> <record-path>
# env:   OBS_EXIT      the step's exit status (required)
#        OBS_STEP      the step number, recorded but not compared
#        OBS_COMMAND   the command line, recorded but not compared
#        OBS_RULES     path to normalise.rules
#        OBS_HOME      the sandbox HOME
#        OBS_TOOLROOT  the root the tool under test owns
#        OBS_MIRROR    the loopback mirror's base URL
#        OBS_FILES     newline-separated files whose content this case
#                      observes (the `observe_files` key of a case)
#        VIBE_BIN      sandbox B's vibe binary
#
# The tool's own root comes from OBS_TOOLROOT, which the harness sets, not
# from `$NVM_DIR` in the shell: `nvm unload` unsets its own variables, and an
# observation that went blind because the tool forgot where it lives would
# compare nothing at exactly the step that matters.
#
# The record is a sorted `field<TAB>value` file. Fields whose name starts
# with `_` are recorded for the reader and never compared.
#
# Fields: exit, node_v, npm_v, which_node, owned_path, installed, default,
# aliases, globals, and one `file:<path>` per observed file.
#
# It is deliberately read-only about the tool: sandbox A's state comes from
# the tool root on disk and sandbox B's from vibe's own query commands, so the
# observation never asks the tool under test to grade itself.

set -uo pipefail

KIND="${1-}"
OUT="${2-}"
[ -n "$KIND" ] && [ -n "$OUT" ] || {
    echo "observe.sh: usage: observe.sh <a|b> <record-path>" >&2
    exit 2
}

ABSENT="absent"
RULES="${OBS_RULES:?observe.sh: OBS_RULES is required}"

# ---------------------------------------------------- the normaliser ------

# The `value` rows of normalise.rules, with the sandbox's own paths
# substituted, become one sed script. Building it once per step is cheap and
# keeps the rules in the file rather than in this script.
SEDSCRIPT="$(mktemp "${TMPDIR:-/tmp}/vibenvm-sed.XXXXXX")" || exit 3
trap 'rm -f "$SEDSCRIPT"' EXIT
awk -F'\t' -v home="${OBS_HOME:-}" -v toolroot="${OBS_TOOLROOT:-}" \
    -v mirror="${OBS_MIRROR:-}" -v tmp="${TMPDIR:-}" '
$1=="value" {
  rule=$2
  if (home!="")     gsub(/@HOME@/, home, rule)
  if (toolroot!="") gsub(/@TOOLROOT@/, toolroot, rule)
  if (mirror!="")   gsub(/@MIRROR@/, mirror, rule)
  if (tmp!="")      gsub(/@TMPDIR@/, tmp, rule)
  print rule
}' "$RULES" >"$SEDSCRIPT"

normalise() { sed -E -f "$SEDSCRIPT"; }

# A record value is one line: newlines become spaces, tabs become spaces.
field() { # field <name> <value...>
    local name="$1"
    shift
    printf '%s\t%s\n' "$name" \
        "$(printf '%s' "$*" | tr '\n\t' '  ' | normalise)"
}

# ---------------------------------------------------- shared probes ------

probe_node_v() { command node -v 2>/dev/null || printf '%s' "$ABSENT"; }
probe_npm_v() { command npm -v 2>/dev/null || printf '%s' "$ABSENT"; }
probe_which_node() { command -v node 2>/dev/null || printf '%s' "$ABSENT"; }

# The ordered PATH entries under the tool's own root — the tool's footprint
# on this shell, which is the whole point of a version manager.
probe_owned_path() {
    local root="${OBS_TOOLROOT:-}" entry out=""
    [ -n "$root" ] || {
        printf '%s' "$ABSENT"
        return
    }
    local IFS=:
    for entry in $PATH; do
        case "$entry" in
            "$root"/*) out="${out:+$out,}$entry" ;;
        esac
    done
    printf '%s' "${out:-none}"
}

# `name@version` for every global package of one version's npm prefix.
globals_of_npm() { # globals_of_npm <npm-binary> [node-bin-dir]
    local npm="$1" bindir="${2-}" line pkg out=""
    [ -x "$npm" ] || {
        printf '%s' "$ABSENT"
        return
    }
    while IFS= read -r line; do
        pkg="${line##*:}"
        case "$pkg" in
            *@*[!@]*) out="${out:+$out|}$pkg" ;;
        esac
    done < <(
        PATH="${bindir:+$bindir:}$PATH" "$npm" ls -g --depth 0 --parseable --long \
            2>/dev/null | sort
    )
    printf '%s' "${out:-none}"
}

# ---------------------------------------------------- sandbox A ----------

a_installed() {
    local dir="${OBS_TOOLROOT:-}/versions/node" entry out=""
    [ -d "$dir" ] || {
        printf '%s' "none"
        return
    }
    while IFS= read -r entry; do
        [ -n "$entry" ] && out="${out:+$out,}$entry"
    done < <(ls -1 "$dir" 2>/dev/null | sort)
    printf '%s' "${out:-none}"
}

a_default() {
    if [ -f "${OBS_TOOLROOT:-}/alias/default" ]; then
        tr -d '\n' <"${OBS_TOOLROOT:-}/alias/default"
    else
        printf '%s' "$ABSENT"
    fi
}

a_aliases() {
    local dir="${OBS_TOOLROOT:-}/alias" rel out=""
    [ -d "$dir" ] || {
        printf '%s' "none"
        return
    }
    while IFS= read -r rel; do
        out="${out:+$out,}${rel#"$dir"/}=$(tr -d '\n' <"$rel")"
    done < <(find "$dir" -type f 2>/dev/null | sort)
    printf '%s' "${out:-none}"
}

a_globals() {
    local dir="${OBS_TOOLROOT:-}/versions/node" version out=""
    [ -d "$dir" ] || {
        printf '%s' "none"
        return
    }
    while IFS= read -r version; do
        [ -n "$version" ] || continue
        out="${out:+$out;}$version=$(globals_of_npm \
            "$dir/$version/bin/npm" "$dir/$version/bin")"
    done < <(ls -1 "$dir" 2>/dev/null | sort)
    printf '%s' "${out:-none}"
}

# ---------------------------------------------------- sandbox B ----------
#
# vibe's own query surface, from the mapping of PROP-075 `#VIBENVM-MAPPING`:
# `vibe app versions node --installed --json` is nvm's installed list,
# `vibe app env --json` resolves the active and default version, and
# `vibe app exec node@<v> -- npm …` reaches one version's own npm. The slot
# has no named user aliases (they are an owner-approved exclusion), so the
# `aliases` field reports the channels the slot resolves.

# Each reader is its own small program over vibe's JSON, so nothing here
# interprets a field it was not told to read.
B_VERSIONS_PY='
import json, sys
data = json.load(sys.stdin)
print(sys.argv[1].join(sorted(v["version"] for v in data.get("versions", []))) or "none")
'
B_DEFAULT_PY='
import json, sys
data = json.load(sys.stdin)
print(data.get("default") or "absent")
'
B_CHANNELS_PY='
import json, sys
data = json.load(sys.stdin)
pairs = sorted((data.get("channels") or {}).items())
print(",".join(f"{name}={target}" for name, target in pairs) or "none")
'

vibe_read() { # vibe_read <python-program> <separator-arg> <vibe args...>
    local program="$1" separator="$2"
    shift 2
    "${VIBE_BIN:?observe.sh: VIBE_BIN is required for sandbox B}" "$@" \
        --json 2>/dev/null | python3 -I -c "$program" "$separator" 2>/dev/null \
        || printf '%s' "$ABSENT"
}

b_installed() {
    vibe_read "$B_VERSIONS_PY" "," app versions node --installed
}

b_default() {
    vibe_read "$B_DEFAULT_PY" "" app env
}

b_aliases() {
    vibe_read "$B_CHANNELS_PY" "" app versions node
}

b_globals() {
    local versions version out=""
    versions="$(vibe_read "$B_VERSIONS_PY" " " app versions node --installed)"
    if [ "$versions" = "$ABSENT" ] || [ "$versions" = "none" ]; then
        printf '%s' "$versions"
        return
    fi
    for version in $versions; do
        out="${out:+$out;}$version=$(
            "$VIBE_BIN" app exec "node@$version" -- npm ls -g --depth 0 --parseable --long \
                2>/dev/null | sort | sed -E 's/^.*://' | grep -E '.@.' | paste -sd'|' -
        )"
    done
    printf '%s' "${out:-none}"
}

# ---------------------------------------------------- the record ---------

{
    printf '_step\t%s\n' "${OBS_STEP:-0}"
    printf '_command\t%s\n' "${OBS_COMMAND:-}"
    field exit "${OBS_EXIT:?observe.sh: OBS_EXIT is required}"
    field node_v "$(probe_node_v)"
    field npm_v "$(probe_npm_v)"
    field which_node "$(probe_which_node)"
    field owned_path "$(probe_owned_path)"
    case "$KIND" in
        a)
            field installed "$(a_installed)"
            field default "$(a_default)"
            field aliases "$(a_aliases)"
            field globals "$(a_globals)"
            ;;
        b)
            field installed "$(b_installed)"
            field default "$(b_default)"
            field aliases "$(b_aliases)"
            field globals "$(b_globals)"
            ;;
        *)
            echo "observe.sh: unknown sandbox kind: $KIND" >&2
            exit 2
            ;;
    esac
    while IFS= read -r observed; do
        [ -n "$observed" ] || continue
        if [ -f "$observed" ]; then
            field "file:$observed" "$(tr -d '\r' <"$observed")"
        else
            field "file:$observed" "$ABSENT"
        fi
    done <<<"${OBS_FILES:-}"
} | sort -t$'\t' -k1,1 >"$OUT"

# The meta-test hook, and the only place in the suite where anything is
# deliberately wrong: `run.sh --self-test` sets it on the second sandbox to
# prove a mismatch is detected and reported with its field and both values.
# It is honoured nowhere else — a real run never sets it.
if [ -n "${VIBENVM_PERTURB_FIELD:-}" ]; then
    awk -F'\t' -v OFS='\t' -v f="$VIBENVM_PERTURB_FIELD" \
        '$1==f {$2=$2 "-PERTURBED"} {print}' "$OUT" >"$OUT.perturbed" \
        && mv "$OUT.perturbed" "$OUT"
fi
exit 0
