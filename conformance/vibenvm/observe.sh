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

# One alias, resolved the way nvm's own `nvm_resolve_alias` resolves it:
# an alias file may name another alias (`default` -> `lts/*` -> `lts/jod`),
# and an alias store's internal chaining is nvm's storage, not an
# observable. What a reader can see is what each name *resolves to*, which
# is also the only thing the other tool can be asked for.
# One alias resolver for both sandboxes, over whichever directory the
# tool keeps its store in. The walk is deliberately naive — follow the
# file, bounded — because it must be the **same** naive walk on both
# sides: a difference in what the two stores hold has to show up as a
# difference in this field, never as a difference in how it was read.
resolve_alias_in() { # resolve_alias_in <alias dir> <name>
    local dir="$1" value="$2" hops=0
    while [ -n "$value" ] && [ -f "$dir/$value" ] && [ "$hops" -lt 10 ]; do
        value="$(tr -d '\n' <"$dir/$value")"
        hops=$((hops + 1))
    done
    printf '%s' "$value"
}

a_resolve_alias() { # a_resolve_alias <name>
    resolve_alias_in "${OBS_TOOLROOT:-}/alias" "$1"
}

# Whether this sandbox holds that version installed. nvm's `default` alias
# survives the uninstall of the version it names, and a default naming a
# version that is gone provides nothing to a new shell — so the record
# reports the default this home can actually give, not the string the tool
# happens to still hold.
a_is_installed() { # a_is_installed <version>
    local dir="${OBS_TOOLROOT:-}/versions/node" bare="${1#v}"
    [ -n "$bare" ] || return 1
    [ -d "$dir/v$bare" ] || [ -d "$dir/$bare" ]
}

a_default() {
    local value
    value="$(a_resolve_alias default)"
    if [ -n "$value" ] && a_is_installed "$value"; then
        printf '%s' "$value"
    else
        printf '%s' "$ABSENT"
    fi
}

a_aliases() {
    local dir="${OBS_TOOLROOT:-}/alias" rel name out="" value
    [ -d "$dir" ] || {
        printf '%s' "none"
        return
    }
    while IFS= read -r rel; do
        name="${rel#"$dir"/}"
        value="$(a_resolve_alias "$name")"
        if [ "$name" = default ] && ! a_is_installed "$value"; then
            continue
        fi
        out="${out:+$out,}$name=$value"
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
# vibe's own query surface, read as the documents vibe actually emits:
# `vibe app list -g --json` (`vibe-app-list/1`) is the home's inventory —
# which versions of the slot are installed and which one is its default,
# answered without a registry, a template or a request; and `vibe app exec
# node@=<v> -- npm …` reaches one version's own npm, the same command
# sandbox A runs. Named user aliases are no longer an accepted gap
# (PROP-075 `#VIBENVM-NO-EXCLUSIONS`): the shim owns a real alias store in
# nvm's own layout, so the `aliases` field is read the same way on both
# sides — the files, walked and resolved by one function — and only its
# directory differs, because each tool keeps its own under its own root.
#
# Every version a target names is spelled `=<version>` — vibe's exact-version
# requirement. A bare `node@22.9.0` is a *requirement*, and would select the
# newest installed version satisfying it, so an observation taken with it
# would silently read a different version than the one it asked about.

# Each reader is its own small program over vibe's JSON, so nothing here
# interprets a field it was not told to read.
B_INSTALLED_PY='
import json, sys
data = json.load(sys.stdin)
slot = sys.argv[2]
versions = sorted(
    row["package"]["version"]
    for row in data.get("installations", [])
    if row.get("slot") == slot
)
print(sys.argv[1].join(versions) or "none")
'
B_DEFAULT_PY='
import json, sys
data = json.load(sys.stdin)
slot = sys.argv[2]
for row in data.get("installations", []):
    if row.get("slot") == slot and row.get("default"):
        print(row["package"]["version"])
        break
else:
    print("absent")
'
# nvm keeps one alias file per LTS line plus `lts/*` for the newest line;

# nvm writes its LTS lines into the alias store as real files the moment
# it reads a remote index (`nvm.sh` 2816-2822), so sandbox A's store holds
# `lts/*`, `lts/iron`, `lts/jod` beside whatever the user named. vibe keeps
# the same facts as **channels** and the shim deliberately does not copy
# them into a second store, so sandbox B reads them from vibe and spells
# them nvm's way: `lts` is vibe's name for "the newest LTS of any line",
# which is nvm's `lts/*`, and the two are one name under two spellings.
B_CHANNELS_PY='
import json, sys
data = json.load(sys.stdin)
rows = []
for package in data.get("packages", []):
    for name, targets in (package.get("channels") or {}).items():
        if not targets:
            continue
        rows.append(("lts/*" if name == "lts" else name, targets[0]))
print(",".join(f"{name}={target}" for name, target in sorted(rows)) or "")
'

vibe_bin() { printf '%s' "${VIBE_BIN:?observe.sh: VIBE_BIN is required for sandbox B}"; }

# The home's inventory, read once per field that needs it.
b_list_read() { # b_list_read <python-program> <separator-arg>
    "$(vibe_bin)" app list -g --json 2>/dev/null |
        python3 -I -c "$1" "$2" node 2>/dev/null ||
        printf '%s' "$ABSENT"
}

b_installed() { b_list_read "$B_INSTALLED_PY" ","; }

b_default() { b_list_read "$B_DEFAULT_PY" ""; }

# The channels are asked for **offline**, so the observation never makes a
# request of its own: it reports the names the tool knows from the version
# list it has already fetched, which is exactly what nvm's alias store is.
# Before any remote call both tools therefore answer `none`.
# The shim's alias store, in nvm's own layout, under the root the shim
# resolves for itself: `$NVM_DIR` when the user set one, else vibe's
# settings root — which in this sandbox is `OBS_TOOLROOT`. So the store is
# `<toolroot>/apps/nvm/alias`, and `resolve_alias_in` walks it exactly as
# it walks nvm's.
b_alias_dir() { printf '%s' "${OBS_TOOLROOT:-}/apps/nvm/alias"; }

b_aliases() {
    local dir rel name out="" value default channels
    dir="$(b_alias_dir)"
    # `default` is never a file here: it is the slot's default, which is
    # vibe's own answer and the one source of truth for it. nvm keeps it
    # as a file, and both sandboxes report the same `default=<version>`.
    default="$(b_default)"
    [ "$default" = "$ABSENT" ] || out="default=$default"
    # The named user aliases, from the store the shim owns.
    if [ -d "$dir" ]; then
        while IFS= read -r rel; do
            name="${rel#"$dir"/}"
            [ "$name" = default ] && continue
            value="$(resolve_alias_in "$dir" "$name")"
            out="${out:+$out,}$name=$value"
        done < <(find "$dir" -type f 2>/dev/null | sort)
    fi
    # The LTS lines, which nvm keeps as files in the same store and vibe
    # keeps as channels. Asked for **offline**, so the observation never
    # makes a request of its own: it reports the names the tool already
    # knows, which is exactly what nvm's own `lts/` files are.
    channels="$("$(vibe_bin)" app versions org.vibevm.node/node --json --offline 2>/dev/null |
        python3 -I -c "$B_CHANNELS_PY" 2>/dev/null)"
    [ -z "$channels" ] || out="${out:+$out,}$channels"
    # The two halves overlap on purpose. The shim writes nvm's `lts/` alias
    # files the moment it learns which lines are LTS, exactly as nvm writes
    # them on reading a release index — so once it has, a line is in the
    # store **and** in vibe's channel answer. Whichever source knew it
    # first, it is one fact and the field carries it once; the store wins,
    # because the store is what nvm would read. A name the two disagree
    # about is the one case worth seeing twice, and it would be a real
    # divergence rather than this bookkeeping.
    [ -z "$out" ] || out="$(printf '%s' "$out" | command tr ',' '\n' |
        command awk -F= '!seen[$1]++' | command paste -sd, -)"
    printf '%s' "${out:-none}"
}

b_globals() {
    local versions version out=""
    versions="$(b_list_read "$B_INSTALLED_PY" " ")"
    if [ "$versions" = "$ABSENT" ] || [ "$versions" = "none" ]; then
        printf '%s' "$versions"
        return
    fi
    local one
    for version in $versions; do
        one="$(
            "$(vibe_bin)" app exec "node@=$version" -- npm ls -g --depth 0 --parseable --long \
                2>/dev/null | sort | sed -E 's/^.*://' | grep -E '.@.' | paste -sd'|' -
        )"
        # A version carrying nothing is `none`, as sandbox A spells it: an
        # empty value and "no global packages" must not read differently.
        out="${out:+$out;}$version=${one:-none}"
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
