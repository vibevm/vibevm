#!/usr/bin/env bash
# Whole-campaign acceptance for the app-versions campaign (PLAN.md §7).
#
# One scripted scenario across P1–P7, in a network-isolated sandbox with a
# fresh HOME. The run re-executes itself inside `bwrap --unshare-net` (only
# loopback exists) and every `vibe` invocation runs under
# `strace -f -e trace=connect`: any connect(2) to a non-loopback address
# fails the run even though it could not have succeeded. A promise is not a
# measurement.
#
# It reaches **no** upstream host. Node's release files, its `index.json` and
# the empty npm registry are served over loopback from the one documented
# mirror this repository already pins, `conformance/vibenvm/mirror`
# (`conformance/vibenvm/fetch-mirror.sh` is the only networked step in the
# tree, and it is a prerequisite here rather than a part of this script).
# The two application packages that live in their own repositories — the real
# `org.vibevm.node/node` template and the `org.vibevm.node/vibenvm` shim —
# are published into a local `localdir` registry by
# `conformance/vibenvm/make-registry.sh`, which is likewise a prerequisite:
# a registry a script could have built out of this repository alone would not
# be the packages the campaign shipped.
#
# The thirteen legs, in order. Nine are §7's own scenario; the last four are
# the legs the owner's decisions of 2026-11-10 added, one per new feature a
# user can see.
#
#    1 install Node from the template at a channel                   (P1–P3)
#    2 `vibe app versions`, with its new version pattern        (P5b, B-263)
#    3 a second version, and `vibe app env` switching this shell      (P5a)
#    4 the prompt hook switching on a `cd`                           (P5a)
#    5 `vibe app exec` at the second version                         (P5a)
#    6 `vibe app migrate` carrying the global npm packages       (P6, P7b)
#    7 mirror the template into a local registry with `mirror.toml`   (P4)
#    8 install from that snapshot with the network denied             (P4)
#    9 the vibenvm differential proof                                (P7f)
#   10 a `keyed` version file written and read back for two slots     (P7a)
#   11 the user's extras list applied to a new version, observed      (P7c)
#   12 an application that recommends another, declined               (P7d)
#   13 an ordinary package's `slot:post-install` steps and a `needs`   (P7e)
#
# usage: accept.sh [path/to/vibe]       (default: <repo>/target/debug/vibe)
# env:   ACCEPT_TMP=<dir>     where the temp directory is created (default /tmp)
#        ACCEPT_KEEP=1        keep the temp directory (logs/<n>.out, <n>.strace)
#        ACCEPT_SKIP_VIBENVM=1  skip leg 9, the minutes-long differential suite
#        VIBENVM_REGISTRY     the local registry holding the two packages
#                             (default: conformance/vibenvm/mirror/registry)
# exit:  0 when every leg passed, 1 otherwise, 2 when a prerequisite is missing.

set -u

SCRIPT="$(readlink -f "${BASH_SOURCE[0]}")"
REPO="$(cd "$(dirname "$SCRIPT")/../.." && pwd)"
MIRROR="$REPO/conformance/vibenvm/mirror"

if [ -z "${ACCEPT_IN_SANDBOX:-}" ]; then
    VIBE_SRC="$(readlink -f "${1:-$REPO/target/debug/vibe}")"
    REGISTRY_SRC="${VIBENVM_REGISTRY:-$MIRROR/registry}"
    missing=""
    for tool in bwrap strace python3 tar sha256sum; do
        command -v "$tool" >/dev/null 2>&1 || missing="$missing $tool"
    done
    [ -x "$VIBE_SRC" ] || missing="$missing $VIBE_SRC (build it: cargo build -p vibe-cli)"
    [ -f "$MIRROR/dist/index.json" ] \
        || missing="$missing $MIRROR/dist/index.json (fetch it once: conformance/vibenvm/fetch-mirror.sh)"
    [ -d "$REGISTRY_SRC/org.vibevm.node/node" ] \
        || missing="$missing $REGISTRY_SRC/org.vibevm.node/node (publish it: conformance/vibenvm/make-registry.sh)"
    [ -d "$REGISTRY_SRC/org.vibevm.node/vibenvm" ] \
        || missing="$missing $REGISTRY_SRC/org.vibevm.node/vibenvm (publish it: conformance/vibenvm/make-registry.sh)"
    if [ -n "$missing" ]; then
        echo "FAIL prerequisites: missing$missing"
        exit 2
    fi
    if ! bwrap --dev-bind / / --unshare-net true 2>/dev/null; then
        echo "FAIL prerequisites: bwrap --unshare-net is not permitted here"
        exit 2
    fi
    bwrap --dev-bind / / --proc /proc --unshare-net --unshare-pid --die-with-parent \
        env ACCEPT_IN_SANDBOX=1 ACCEPT_VIBE="$VIBE_SRC" ACCEPT_REGISTRY="$REGISTRY_SRC" \
        ACCEPT_TMP="${ACCEPT_TMP:-/tmp}" ACCEPT_KEEP="${ACCEPT_KEEP:-}" \
        ACCEPT_SKIP_VIBENVM="${ACCEPT_SKIP_VIBENVM:-}" \
        bash "$SCRIPT"
    exit $?
fi

# ---------------------------------------------------------------- sandbox ---

VIBE="$ACCEPT_VIBE"
REGISTRY="$ACCEPT_REGISTRY"
W="$(mktemp -d "${ACCEPT_TMP:-/tmp}/vibe-appversions-accept.XXXXXX")"
LOGS="$W/logs"
mkdir -p "$LOGS" "$W/tmp"

HOME_DIR="$W/home"
SETTINGS="$W/settings"
PROJECT="$W/project"
CLOSED="$W/closed"           # the "inside a closed network" home of legs 7–8
TARGET="$W/mirror-target"    # the registry `vibe registry mirror` writes
LOCAL="$W/local-registry"    # the fixture packages of legs 10–13
mkdir -p "$HOME_DIR" "$SETTINGS" "$PROJECT" "$CLOSED" "$LOCAL" "$TARGET"

# The pinned version set of the documented mirror. Two versions in one major
# (22.9.0 non-LTS, 22.11.0 the Jod LTS line), one older major (20.18.0, the
# Iron LTS line) and one newer non-LTS (23.1.0), which is exactly what makes
# a channel, a prefix and a pin distinguishable below.
NODE_LTS_VERSION="22.11.0"
NODE_SECOND_VERSION="20.18.0"
NODE_PINNED_VERSION="22.9.0"

PASS=0
FAIL=0
STEP=0

step() {
    STEP=$((STEP + 1))
    printf '%2d. %s ... ' "$STEP" "$1"
}
ok() {
    PASS=$((PASS + 1))
    echo "ok"
}
bad() {
    FAIL=$((FAIL + 1))
    echo "FAIL: $1"
}
skip() { echo "skipped ($1)"; }

# ------------------------------------------------- the loopback mirror ---
#
# Served from the one documented mirror, over loopback, exactly as
# conformance/vibenvm/run.sh serves it. Node's own release host is then
# reached through a `[[remap]]` row — the mechanism a closed network uses
# anyway (PROP-066 `#locator`), so this exercises the real path and not a
# test-only flag.

SERVER_LOG="$W/mirror-server.log"
(
    cd "$MIRROR" || exit 1
    exec python3 -I -u -m http.server 0 --bind 127.0.0.1 >"$SERVER_LOG" 2>&1
) &
SERVER_PID=$!
if [ -z "${ACCEPT_KEEP:-}" ]; then
    trap 'kill "$SERVER_PID" 2>/dev/null; rm -rf "$W"' EXIT
else
    trap 'kill "$SERVER_PID" 2>/dev/null; echo "kept: $W"' EXIT
fi
MIRROR_URL=""
for _ in $(seq 1 100); do
    MIRROR_URL="$(sed -nE 's#.*Serving HTTP on 127\.0\.0\.1 port ([0-9]+).*#http://127.0.0.1:\1#p' "$SERVER_LOG" 2>/dev/null | head -1)"
    [ -n "$MIRROR_URL" ] && break
    sleep 0.1
done
if [ -z "$MIRROR_URL" ]; then
    echo "FAIL prerequisites: the loopback mirror server did not start: $(cat "$SERVER_LOG" 2>/dev/null)"
    exit 2
fi

settings_for() { # settings_for <dir> <extra-toml-or-empty>
    mkdir -p "$1"
    cat >"$1/settings.toml" <<EOF
# Written by campaigns/app-versions-2026-11/accept.sh.
[[remap]]
prefix = "https://nodejs.org/dist"
url = "$MIRROR_URL/dist"
reason = "the campaign acceptance's local Node release mirror"
EOF
    cat >"$1/registry.toml" <<EOF
# Written by campaigns/app-versions-2026-11/accept.sh.
[[registry]]
name = "accept-local"
type = "localdir"
url = "file://$2"
ref = "main"
naming = "fqdn"

# Leg 7 mirrors into this one, which is why it is declared rather than passed
# as a path: --into takes the name of a monorepo or localdir registry.
[[registry]]
name = "accept-target"
type = "localdir"
url = "file://$TARGET"
ref = "main"
naming = "fqdn"
EOF
}

# ------------------------------------------------ the loopback npm registry ---
#
# Leg 6 carries a **real** global npm package between two Node versions, so
# npm has to be able to reinstall it by name. The documented mirror's own
# `npm-registry/` is empty and digest-pinned — a run that wrote into it could
# not verify it again — so this one serves a registry of its own, over
# loopback, holding exactly one package with no dependencies.

NPM_ROOT="$W/npm-root"
mkdir -p "$W/npm-fixture/package" "$NPM_ROOT"
printf '{\n  "name": "accept-npm-fixture",\n  "version": "1.0.0",\n  "license": "UPL-1.0"\n}\n' \
    >"$W/npm-fixture/package/package.json"
tar -czf "$NPM_ROOT/accept-npm-fixture-1.0.0.tgz" -C "$W/npm-fixture" package

NPM_LOG="$W/npm-server.log"
(
    cd "$NPM_ROOT" || exit 1
    exec python3 -I -u -m http.server 0 --bind 127.0.0.1 >"$NPM_LOG" 2>&1
) &
NPM_PID=$!
if [ -z "${ACCEPT_KEEP:-}" ]; then
    trap 'kill "$SERVER_PID" "$NPM_PID" 2>/dev/null; rm -rf "$W"' EXIT
else
    trap 'kill "$SERVER_PID" "$NPM_PID" 2>/dev/null; echo "kept: $W"' EXIT
fi
NPM_URL=""
for _ in $(seq 1 100); do
    NPM_URL="$(sed -nE 's#.*Serving HTTP on 127\.0\.0\.1 port ([0-9]+).*#http://127.0.0.1:\1#p' "$NPM_LOG" 2>/dev/null | head -1)"
    [ -n "$NPM_URL" ] && break
    sleep 0.1
done
if [ -z "$NPM_URL" ]; then
    echo "FAIL prerequisites: the loopback npm registry did not start: $(cat "$NPM_LOG" 2>/dev/null)"
    exit 2
fi
# One packument, written as the file npm asks for by that name, with the
# tarball's own sha1 so npm's own integrity check is the one that passes it.
python3 -I "$REPO/campaigns/app-versions-2026-11/accept-npm-packument.py" \
    "$NPM_ROOT" "$NPM_URL" \
    || {
        echo "FAIL prerequisites: the npm packument could not be written"
        exit 2
    }

settings_for "$SETTINGS" "$REGISTRY"

# Run `vibe` under strace with the isolated home, in `$W` unless a leg says
# otherwise. Any connect(2) outside loopback fails the run.
# `vibe` on PATH for a shell that needs it as a program — the prompt hook
# does, because the managed block calls `command vibe app env`.
mkdir -p "$W/bin"
ln -sf "$VIBE" "$W/bin/vibe"

# Stdout and stderr are kept **apart**: the plan a mutation prints goes to
# stderr so that `--json` keeps one envelope on stdout, and a run that merged
# them could not read its own answers.
vibe_in() { # vibe_in <cwd> <args…>
    local cwd="$1"
    shift
    local out="$LOGS/$STEP.out" err="$LOGS/$STEP.err" trace="$LOGS/$STEP.strace"
    (
        cd "$cwd" || exit 97
        exec env -i \
            HOME="$HOME_DIR" \
            PATH="$W/bin:/usr/bin:/bin" \
            TMPDIR="$W/tmp" \
            LC_ALL=C \
            VIBE_SETTINGS="$SETTINGS" \
            VIBE_REGISTRY_CACHE="$W/cache" \
            VIBE_NO_DEFAULT_REGISTRY=1 \
            npm_config_registry="$NPM_URL/" \
            npm_config_audit=false \
            npm_config_fund=false \
            npm_config_update_notifier=false \
            strace -f -o "$trace" -e trace=connect \
            "$VIBE" "$@"
    ) >"$out" 2>"$err"
    local code=$?
    guard_trace "$trace"
    return $code
}

# `$W` is deliberately not a project: a `vibe app` verb run inside one
# resolves that project's home, and these legs are about the user's.
vibe() { vibe_in "$W" "$@"; }

guard_trace() { # guard_trace <trace-file>
    if grep -E 'connect\(' "$1" 2>/dev/null \
        | grep -E 'AF_INET6?' \
        | grep -v 'inet_addr("127\.' \
        | grep -qv '"::1"'; then
        echo "non-loopback connect(2) attempt:" >&2
        grep -E 'connect\(' "$1" >&2
        FAIL=$((FAIL + 1))
    fi
}

out() { cat "$LOGS/$STEP.out" "$LOGS/$STEP.err" 2>/dev/null; }

# One JSON question of the last step's document, as a python expression over
# `d`. Printed, so a leg can compare it as a string.
ask() { # ask <python-expression>
    python3 -I -c '
import json, sys
d = json.load(open(sys.argv[1]))
print(eval(sys.argv[2]))
' "$LOGS/$STEP.out" "$1" 2>>"$LOGS/$STEP.ask"
}

# ------------------------------------------------------------- fixtures ---
#
# The four legs the owner's decisions added need packages of their own, and
# they are built here rather than mirrored: each is the smallest package that
# exercises one declaration, and none of them touches the network.

fixture_app() { # fixture_app <name> <slot> <extra-above-[application]> <extra-in-application>
    local name="$1" slot="$2" above="$3" inside="$4"
    local dir="$LOCAL/org.accept/$name/v1.0.0"
    mkdir -p "$dir/payload/bin"
    cat >"$dir/vibe.toml" <<EOF
[package]
name = "$name"
group = "org.accept"
kind = "app"
version = "1.0.0"
$above
[application]
id = "$slot"
$inside

[[installer_options]]
id = "main"
name = "Main"
platform = "linux"
platform_name = "Linux"
arch = "$(uname -m)"

[[installer_options.steps]]
id = "place"
title = "Place the payload"
kind = "copy"
from = "payload"

[[installer_options.launchers]]
id = "run"
name = "Run"
executable = "\$app_bin/accept-$name"
kind = "command"

[[installer_options.public_commands]]
name = "accept-$name"
launcher = "run"
EOF
    printf '#!/bin/sh\necho "%s $*"\n' "$name" >"$dir/payload/bin/accept-$name"
    chmod 755 "$dir/payload/bin/accept-$name"
}

# Two slots that share one `.sdkmanrc`, each editing only its own line —
# the shape a Java pass needs (PROP-075 `#VERSION-FILE-KEYED`).
fixture_keyed() { # fixture_keyed <name>
    fixture_app "$1" "$1" "" "
[[application.version_file]]
format = \"plain\"
names = [\".${1}rc\"]

[[application.version_file]]
format = \"keyed\"
names = [\".sdkmanrc\"]
key = \"$1\"
separator = \"equals\""
}

# An ordinary (non-application) package with two judged `slot:post-install`
# contributions: one that runs and one whose `needs` can never be met
# (PROP-065 `#conditions`, `#needs`, `#ordinary-plan`).
fixture_ordinary() { # fixture_ordinary <name> <needs-or-empty>
    local name="$1" needs="$2"
    local dir="$LOCAL/org.accept/$name/v1.0.0"
    mkdir -p "$dir/hooks" "$dir/boot"
    cat >"$dir/vibe.toml" <<EOF
[package]
name = "$name"
group = "org.accept"
kind = "flow"
version = "1.0.0"
format = "simple"

[boot_snippet]
source = "boot/main.md"

[[extension]]
id = "judged"
point = "slot:post-install"
title = "Judged contribution"
handler = { kind = "script", base = "hooks/post" }
$needs
EOF
    printf '# %s {#root}\n\nthe accepted ordinary package\n' "$name" >"$dir/boot/main.md"
    # `base = "hooks/post"` selects a declared script **variant** (PROP-020):
    # the POSIX one is what this host runs, and the PowerShell one ships so
    # the package is not Linux-only by omission.
    printf '#!/bin/sh\necho "judged ran"\n' >"$dir/hooks/post.sh"
    printf 'Write-Output "judged ran"\n' >"$dir/hooks/post.ps1"
    chmod 755 "$dir/hooks/post.sh"
}

# ----------------------------------------------------------------- the run ---

echo "app-versions acceptance — $W"
echo "mirror: $MIRROR_URL  ·  registry: $REGISTRY"
echo

# --- 1 -----------------------------------------------------------------------
step "Node installs from the template at a channel"
if vibe install -g "org.vibevm.node/node@lts" --unattended --json; then
    installed="$(ask 'd["installation"]')"
    if [ "$installed" = "org.vibevm.node/node@$NODE_LTS_VERSION" ]; then
        # The lock-shaped template record is what makes the render verifiable
        # later (PROP-075 `#TEMPLATE-LOCK-RECORD`).
        revision="$(python3 -I -c '
import json, sys
d = json.load(open(sys.argv[1]))
print(d["installations"][sys.argv[2]]["template"]["revision"])
' "$SETTINGS/applications/index.json" "org.vibevm.node/node@$NODE_LTS_VERSION" 2>/dev/null)"
        if [ -n "$revision" ]; then
            ok
        else
            bad "the installation record carries no template revision"
        fi
    else
        bad "@lts resolved to '$installed', not $NODE_LTS_VERSION"
    fi
else
    bad "$(out | tail -5)"
fi

# --- 2 -----------------------------------------------------------------------
step "the version listing carries origins, channels and a pattern filter"
if vibe app versions node --all --json; then
    facts="$(ask '(
    len(d["packages"][0]["versions"]),
    d["packages"][0]["template"]["revision"],
    sorted(d["packages"][0]["channels"]),
    [r["state"] for r in d["packages"][0]["versions"] if r["version"] == "'"$NODE_LTS_VERSION"'"],
)')"
    case "$facts" in
        *"'lts'"*"'lts/jod'"*"installed"*) : ;;
        *)
            bad "the listing is not the template's own domain: $facts"
            facts=""
            ;;
    esac
    if [ -n "$facts" ]; then
        # B-263: the positional pattern narrows by the same prefix rule a
        # version file's value is read with. The same step's log is reused,
        # so the leg's evidence is the narrowed answer.
        if vibe app versions node --all --json "$NODE_SECOND_VERSION" \
            && [ "$(ask 'len(d["packages"][0]["versions"])')" = "1" ] \
            && [ "$(ask 'd["packages"][0]["versions"][0]["version"]')" = "$NODE_SECOND_VERSION" ]; then
            ok
        else
            bad "the pattern did not narrow the listing: $(out | tail -3)"
        fi
    fi
else
    bad "$(out | tail -5)"
fi

# --- 3 -----------------------------------------------------------------------
step "a second version installs and \`vibe app env\` switches this shell"
if vibe install -g "org.vibevm.node/node@=$NODE_SECOND_VERSION" --unattended --json; then
    if vibe app env --use "node==$NODE_SECOND_VERSION" --json; then
        chosen="$(ask 'd["variables"]["VIBE_APP_USE"]')"
        leading="$(ask 'd["path"][0]')"
        reported="$(cd "$W" && env -i HOME="$HOME_DIR" PATH="$leading:/usr/bin:/bin" node -v 2>&1)"
        if [ "$chosen" = "node=org.vibevm.node/node@$NODE_SECOND_VERSION" ] \
            && [ "$reported" = "v$NODE_SECOND_VERSION" ]; then
            ok
        else
            bad "chose '$chosen' and the leading PATH entry runs '$reported'"
        fi
    else
        bad "$(out | tail -5)"
    fi
else
    bad "the second install failed: $(out | tail -5)"
fi

# --- 4 -----------------------------------------------------------------------
step "the prompt hook applies the project's version on a \`cd\`"
printf '[project]\nname = "accept"\nversion = "1.0.0"\n' >"$PROJECT/vibe.toml"
printf '%s\n' "$NODE_PINNED_VERSION" >"$PROJECT/.nvmrc"
vibe install -g "org.vibevm.node/node@=$NODE_PINNED_VERSION" --unattended --json >/dev/null 2>&1
if vibe hook bash --print; then
    cp "$LOGS/$STEP.out" "$W/hook.bash"
    # The block is sourced into a real bash, which then `cd`s into the
    # project: the version the hook applies is the one `node -v` reports,
    # and nothing of the operator's environment is in scope.
    cat >"$W/hook-drive.sh" <<EOF
set -u
. "$W/hook.bash"
cd "$PROJECT" || exit 9
# The hook runs on the prompt, which a non-interactive shell never draws.
if [ -n "\${PROMPT_COMMAND:-}" ]; then eval "\$PROMPT_COMMAND"; fi
command -v node >/dev/null 2>&1 && node -v || echo "no node"
EOF
    applied="$(env -i HOME="$HOME_DIR" PATH="$W/bin:/usr/bin:/bin" TMPDIR="$W/tmp" LC_ALL=C \
        VIBE_SETTINGS="$SETTINGS" VIBE_REGISTRY_CACHE="$W/cache" \
        VIBE_NO_DEFAULT_REGISTRY=1 \
        bash "$W/hook-drive.sh" 2>"$LOGS/$STEP.hook" | tail -1)"
    if [ "$applied" = "v$NODE_PINNED_VERSION" ]; then
        ok
    else
        bad "the hook left \`node -v\` at '$applied', not v$NODE_PINNED_VERSION"
    fi
else
    bad "$(out | tail -5)"
fi

# --- 5 -----------------------------------------------------------------------
step "\`vibe app exec\` runs the second version without changing anything"
before="$(python3 -I -c '
import json, sys
print(json.load(open(sys.argv[1]))["slots"]["node"]["default"])
' "$SETTINGS/applications/index.json" 2>/dev/null)"
if vibe app exec "node@=$NODE_SECOND_VERSION" -- node -v; then
    ran="$(out | tail -1)"
    after="$(python3 -I -c '
import json, sys
print(json.load(open(sys.argv[1]))["slots"]["node"]["default"])
' "$SETTINGS/applications/index.json" 2>/dev/null)"
    if [ "$ran" = "v$NODE_SECOND_VERSION" ] && [ "$before" = "$after" ]; then
        ok
    else
        bad "ran '$ran' and the default moved from '$before' to '$after'"
    fi
else
    bad "$(out | tail -5)"
fi

# --- 6 -----------------------------------------------------------------------
step "\`vibe app migrate\` carries the global npm packages between versions"
# The package is the one the loopback npm registry above holds, named
# rather than given as a path: a carry reinstalls by name, so a path would
# prove only that the first install worked.
if vibe app extras add node --ecosystem npm accept-npm-fixture@1.0.0 --json \
    && vibe app use node "org.vibevm.node/node@$NODE_LTS_VERSION" >/dev/null 2>&1 \
    && vibe app extras apply node --ecosystem npm --json; then
    if vibe app migrate node "$NODE_LTS_VERSION" "$NODE_SECOND_VERSION" \
        --only ecosystem:npm --yes --json; then
        vibe app use node "org.vibevm.node/node@$NODE_SECOND_VERSION" >/dev/null 2>&1
        vibe app extras list node --ecosystem npm --json
        carried="$(ask '[i for e in d.get("ecosystems", []) for i in e.get("observed", [])]')"
        case "$carried" in
            *accept-npm-fixture*) ok ;;
            *) bad "the adapter's own list does not see the carried package: $carried" ;;
        esac
    else
        bad "the migration failed: $(out | tail -5)"
    fi
else
    bad "the extras could not be placed in the first version: $(out | tail -8)"
fi

# --- 7 -----------------------------------------------------------------------
step "the template mirrors into a local registry from \`mirror.toml\`"
cat >"$W/mirror.toml" <<EOF
schema = 1

[select]
all = false

[[template]]
package = "org.vibevm.node/node"
versions = ["=$NODE_PINNED_VERSION"]
platforms = ["linux"]
archs = ["$(uname -m)"]
payloads = true

[payloads]
into = "$W/drop"
EOF
mkdir -p "$W/drop"
mkdir -p "$TARGET"
if vibe registry mirror accept-local --into accept-target --config "$W/mirror.toml" \
    --json; then
    snapshot="$TARGET/_index/templates/org.vibevm.node/node"
    payload="$(find "$W/drop" -type f -name '*.tar.*' 2>/dev/null | head -1)"
    if [ -f "$snapshot/snapshot.toml" ] \
        && [ -f "$snapshot/versions.json" ] \
        && [ -f "$snapshot/digests/$NODE_PINNED_VERSION" ] \
        && grep -q "$NODE_PINNED_VERSION" "$snapshot/snapshot.toml" \
        && [ -n "$payload" ]; then
        ok
    else
        bad "the snapshot is not under _index/templates: $(ls -R "$TARGET/_index" 2>&1 | head -20)"
    fi
else
    bad "$(out | tail -8)"
fi

# --- 8 -----------------------------------------------------------------------
step "a closed-network home installs that version from the snapshot alone"
# Its own home, its own settings, **no remap row at all** and `--offline`:
# the only thing it can read is the mirrored registry and the snapshot inside
# it. The strace guard then proves it opened nothing.
mkdir -p "$CLOSED/settings" "$CLOSED/home"
cat >"$CLOSED/settings/registry.toml" <<EOF
[[registry]]
name = "closed"
type = "localdir"
url = "file://$TARGET"
ref = "main"
naming = "fqdn"
EOF
# The bytes come from the drop the mirror projected into, which is the one
# artifact mechanism of PROP-066 #drops and not a flag of its own.
cat >"$CLOSED/settings/config.toml" <<EOF
[artifacts]
packages = "$W/drop"
EOF
if env -i HOME="$CLOSED/home" PATH="/usr/bin:/bin" TMPDIR="$W/tmp" LC_ALL=C \
    VIBE_SETTINGS="$CLOSED/settings" VIBE_REGISTRY_CACHE="$CLOSED/cache" \
    VIBE_NO_DEFAULT_REGISTRY=1 \
    strace -f -o "$LOGS/$STEP.strace" -e trace=connect \
    "$VIBE" install -g "org.vibevm.node/node@=$NODE_PINNED_VERSION" \
    --offline --unattended --json >"$LOGS/$STEP.out" 2>"$LOGS/$STEP.err"; then
    guard_trace "$LOGS/$STEP.strace"
    if [ "$(ask 'd["installation"]')" = "org.vibevm.node/node@$NODE_PINNED_VERSION" ]; then
        ok
    else
        bad "the closed install placed '$(ask 'd["installation"]')'"
    fi
else
    guard_trace "$LOGS/$STEP.strace"
    bad "$(out | tail -8)"
fi

# --- 9 -----------------------------------------------------------------------
step "the vibenvm differential proof returns verdict 0"
if [ -n "${ACCEPT_SKIP_VIBENVM:-}" ]; then
    skip "ACCEPT_SKIP_VIBENVM"
else
    # Already inside the network namespace, so the suite does not re-exec
    # itself into a second one; it serves its own loopback mirror and builds
    # its own two sandboxes.
    if env VIBENVM_IN_SANDBOX=1 VIBE_BIN="$VIBE" VIBENVM_REGISTRY="$REGISTRY" \
        VIBENVM_TMP="$W/tmp" \
        bash "$REPO/conformance/vibenvm/run.sh" >"$LOGS/$STEP.out" 2>&1; then
        grep -q 'verdict:    0' "$LOGS/$STEP.out" && ok \
            || bad "the suite exited 0 without printing verdict 0: $(out | tail -5)"
    else
        bad "verdict $?: $(out | tail -15)"
    fi
fi

# --- 10 ----------------------------------------------------------------------
step "one \`keyed\` version file is written and read back for two slots"
fixture_keyed alpha
fixture_keyed beta
SHARED="$PROJECT/.sdkmanrc"
printf '# the team'"'"'s toolchain\ngamma=9.9.9\n' >"$SHARED"
if vibe install -g org.accept/alpha --registry "$LOCAL" --offline --unattended --json \
    && vibe install -g org.accept/beta --registry "$LOCAL" --offline --unattended --json; then
    wrote=1
    for slot in alpha beta; do
        vibe_in "$PROJECT" app env --use "$slot==1.0.0" --write-version-file=.sdkmanrc --json \
            || { wrote=0; bad "the $slot write failed: $(out | tail -5)"; break; }
    done
    if [ "$wrote" = 1 ]; then
        # Each slot edited only its own line, and the third key, the heading
        # comment and the file's own order survived byte for byte.
        want="$(printf '# the team'"'"'s toolchain\ngamma=9.9.9\nalpha=1.0.0\nbeta=1.0.0\n')"
        if [ "$(cat "$SHARED")" = "$want" ]; then
            vibe_in "$PROJECT" app env --project --json
            read_back="$(ask 'sorted((s["slot"], s["key"], s["source"]) for s in d["selections"] if s["slot"] in ("alpha", "beta"))')"
            case "$read_back" in
                *"org.accept/alpha@1.0.0"*"version_file"*"org.accept/beta@1.0.0"*"version_file"*) ok ;;
                *) bad "the shared file did not resolve both slots: $read_back" ;;
            esac
        else
            bad "the shared file is not what two single-line edits make: $(cat "$SHARED")"
        fi
    fi
else
    bad "the two keyed fixtures did not install: $(out | tail -8)"
fi

# --- 11 ----------------------------------------------------------------------
step "the user's extras list reaches a newly installed version"
# The list is already in `~/.vibe/config.toml` from leg 6, and leg 6 proved
# the adapter observes what it placed. What this leg adds is the *automatic*
# apply: a version the home did not hold gets the list without being asked
# (PROP-075 `#SLOT-EXTRAS`), and `--no-extras` is the decline.
if vibe install -g "org.vibevm.node/node@=23.1.0" --unattended --json; then
    vibe app use node "org.vibevm.node/node@23.1.0" >/dev/null 2>&1
    vibe app extras list node --ecosystem npm --json
    applied="$(ask '[i for e in d.get("ecosystems", []) for i in e.get("observed", [])]')"
    case "$applied" in
        *accept-npm-fixture*)
            # And the decline: a fourth version taken with `--no-extras`
            # carries nothing.
            if vibe install -g "org.vibevm.node/node@=$NODE_LTS_VERSION" --force \
                --no-extras --unattended --json >/dev/null 2>&1; then
                ok
            else
                bad "the --no-extras install failed: $(out | tail -5)"
            fi
            ;;
        *) bad "a new version did not get the user's list: $applied" ;;
    esac
else
    bad "$(out | tail -8)"
fi

# --- 12 ----------------------------------------------------------------------
step "a recommended application is installed, and \`--no-recommends\` declines"
fixture_app tool-a tool-a "" ""
fixture_app tool-b tool-b "
[recommends]
packages = [\"org.accept/tool-a\"]
" ""
if vibe install -g org.accept/tool-b --registry "$LOCAL" --offline --unattended --json; then
    rows="$(ask '[(r["package"], r["outcome"]) for r in d.get("recommended", [])]')"
    case "$rows" in
        *"org.accept/tool-a"*installed*)
            vibe uninstall -g org.accept/tool-b --assume-yes >/dev/null 2>&1
            vibe uninstall -g org.accept/tool-a --assume-yes >/dev/null 2>&1
            if vibe install -g org.accept/tool-b --registry "$LOCAL" --offline \
                --unattended --no-recommends --json; then
                declined="$(ask 'len(d.get("recommended", []))')"
                present="$(python3 -I -c '
import json, sys
d = json.load(open(sys.argv[1]))
print(any(k.startswith("org.accept/tool-a@") for k in d["installations"]))
' "$SETTINGS/applications/index.json" 2>/dev/null)"
                if [ "$declined" = "0" ] && [ "$present" = "False" ]; then
                    ok
                else
                    bad "--no-recommends reported $declined rows and tool-a present=$present"
                fi
            else
                bad "$(out | tail -5)"
            fi
            ;;
        *) bad "the recommendation was not installed: $rows" ;;
    esac
else
    bad "$(out | tail -8)"
fi

# --- 13 ----------------------------------------------------------------------
step "an ordinary package's judged steps are planned, and an unmet need refuses"
fixture_ordinary ordinary-ok ""
fixture_ordinary ordinary-need 'needs = ["command:no-such-command-anywhere"]'
ORD="$W/ordinary"
mkdir -p "$ORD"
write_ordinary_project() { # write_ordinary_project <package-name>
    cat >"$ORD/vibe.toml" <<EOF
[project]
group = "org.accept"
name = "consumer"
version = "0.1.0"
spec_format = "markdown"

[requires.packages]
"org.accept/$1" = { version = "=1.0.0" }

[[registry]]
name = "accept-local"
url = "file://$LOCAL"
EOF
    mkdir -p "$ORD/vibevm/vibespecs/boot"
}
write_ordinary_project ordinary-ok
# What this build reports is the install's own `contributions` array, one row
# per `slot:` contribution with its point, its handler and its outcome.
# PLAN §7 asks for the rows in `--plan-only`; the ordinary plane's
# `--plan-only` preview does not carry them, because PROP-065
# `#ordinary-plan` is `impl/work` and `AV-FMT-032` was deliberately not
# delivered by P7e. The leg therefore measures the row where it exists, and
# says so rather than asserting a surface the campaign did not build.
if vibe_in "$ORD" install --offline --assume-yes --json --path "$ORD"; then
    reported="$(python3 -I "$REPO/campaigns/app-versions-2026-11/accept-contributions.py" \
        "$LOGS/$STEP.out" 2>/dev/null)"
    case "$reported" in
        *"slot:post-install"*script*ok*)
            write_ordinary_project ordinary-need
            rm -rf "$ORD/vibevm/vibedeps" "$ORD/vibe.lock"
            if vibe_in "$ORD" install --offline --assume-yes --json --path "$ORD"; then
                bad "an unmet need did not refuse the installation"
            else
                message="$(out)"
                case "$message" in
                    *"no-such-command-anywhere"*"PROP-065#needs"*) ok ;;
                    *) bad "the refusal names neither the need nor the law: $(out | tail -8)" ;;
                esac
            fi
            ;;
        *) bad "the install reports no judged slot:post-install row: $reported" ;;
    esac
else
    bad "$(out | tail -10)"
fi

echo
echo "legs: $PASS passed, $FAIL failed"
[ "$FAIL" = 0 ] || exit 1
exit 0
