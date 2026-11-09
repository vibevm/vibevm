#!/usr/bin/env bash
# Closed-network acceptance for the corporate-env campaign (PLAN.md §4 Phase 9, §7).
#
# One scripted scenario on a fresh machine with no network: the whole run is
# re-executed inside `bwrap --unshare-net` (only loopback exists), and every
# `vibe` invocation runs under `strace -f -e trace=connect`; any connect(2) to
# a non-loopback address is a failure even though it could not succeed.
#
# What it sets up (all under one temp directory, removed at the end):
#   * an isolated HOME and VIBE_SETTINGS (nothing of the operator's is read);
#   * a "downloaded" vibe binary placed by hand in <tmp>/download/;
#   * a loopback sshd (crates/vibe-registry/.../ssh_fixture.py) serving one
#     writable Git repository: the SSH monorepo `corp`;
#   * a shared folder registry `shared` (type localdir);
#   * a release drop with fake vibe releases in the GitHub asset layout;
#   * a package drop holding an application's payload;
#   * a per-package Git registry `upstream` to mirror from;
#   * one site profile corp.toml naming all of it, with
#     exclude_hosting = ["github", "gitverse"], [self] install_root, [path] fix.
#
# usage: accept.sh [path/to/vibe]       (default: <repo>/target/debug/vibe)
# env:   ACCEPT_TMP=<dir>  where the temp directory is created (default /tmp); it and
#                          its parents must not be group/world writable (sticky /tmp is fine)
#        ACCEPT_KEEP=1     keep the temp directory (logs/<n>.out, <n>.strace, commands.txt)
#        ACCEPT_VIBE_ENV="K=V …"  extra environment for every vibe call (e.g. VIBE_LOG=debug)
#        ACCEPT_HOOK=<script>     debugging: run this script against the live fixtures
#                                 instead of the steps (W, HOME_DIR, SETTINGS, SITE, BARE,
#                                 MONOREPO_URL exported)
# exit:  0 when every step passed, 1 otherwise, 2 when a prerequisite is missing.

set -u

SCRIPT="$(readlink -f "${BASH_SOURCE[0]}")"
REPO="$(cd "$(dirname "$SCRIPT")/../.." && pwd)"
SSH_FIXTURE="$REPO/crates/vibe-registry/src/git_backend/shell/native_admission/ssh_fixture.py"

if [ -z "${ACCEPT_IN_SANDBOX:-}" ]; then
    VIBE_SRC="$(readlink -f "${1:-$REPO/target/debug/vibe}")"
    missing=""
    for tool in bwrap strace python3 git ssh ssh-keygen ssh-agent; do
        command -v "$tool" >/dev/null 2>&1 || missing="$missing $tool"
    done
    [ -x /usr/sbin/sshd ] || missing="$missing /usr/sbin/sshd"
    [ -x "$VIBE_SRC" ] || missing="$missing $VIBE_SRC (build it: cargo build -p vibe-cli)"
    [ -f "$SSH_FIXTURE" ] || missing="$missing $SSH_FIXTURE"
    if [ -n "$missing" ]; then
        echo "FAIL prerequisites: missing$missing"
        exit 2
    fi
    if ! bwrap --dev-bind / / --unshare-net true 2>/dev/null; then
        echo "FAIL prerequisites: bwrap --unshare-net is not permitted here"
        exit 2
    fi
    # The real network is gone from here on; only loopback exists.
    # Inside an unprivileged user namespace every file of another account
    # (root's /etc/ssh included) appears owned by `nobody`, which OpenSSH-
    # style ownership checks rightly call unsafe. A private copy of the
    # system SSH client configuration, owned by this user, takes its place.
    ETC_SSH="$(mktemp -d "${TMPDIR:-/tmp}/vibe-accept-etc-ssh.XXXXXX")"
    cp -rL /etc/ssh/ssh_config /etc/ssh/ssh_config.d "$ETC_SSH/" 2>/dev/null
    trap 'rm -rf "$ETC_SSH"' EXIT
    # --unshare-pid: every process the run starts (sshd included) ends with it.
    bwrap --dev-bind / / --bind "$ETC_SSH" /etc/ssh --proc /proc \
        --unshare-net --unshare-pid --die-with-parent \
        env ACCEPT_IN_SANDBOX=1 ACCEPT_VIBE="$VIBE_SRC" bash "$SCRIPT"
    exit $?
fi

# ---------------------------------------------------------------- sandbox ---

VIBE_SRC="$ACCEPT_VIBE"
umask 022
BASE="${ACCEPT_TMP:-/tmp}"
# OpenSSH and vibe's controlled transport refuse an SSH config, key or
# known_hosts file under a directory another user may write (group/world
# writable without the sticky bit), exactly as on a real machine.
d="$(readlink -f "$BASE")"
while [ -n "$d" ]; do
    mode="$(stat -c '%a' "$d")"
    if [ $(( 0$mode & 022 )) -ne 0 ] && [ $(( 0$mode & 01000 )) -eq 0 ]; then
        echo "FAIL prerequisites: $d is group/world writable (mode $mode); choose ACCEPT_TMP under a private directory"
        exit 2
    fi
    [ "$d" = / ] && break
    d="$(dirname "$d")"
done
W="$(mktemp -d "$BASE/vibe-accept.XXXXXX")"
LOGS="$W/logs"
mkdir -p "$LOGS"
FIXTURE_PID=""
HTTP_PID=""

cleanup() {
    [ -n "$HTTP_PID" ] && kill "$HTTP_PID" 2>/dev/null
    if [ -n "$FIXTURE_PID" ]; then
        # The fixture stops its sshd and agent when its stdin closes.
        exec 9>&- 2>/dev/null
        for _ in $(seq 1 100); do kill -0 "$FIXTURE_PID" 2>/dev/null || break; sleep 0.05; done
        kill "$FIXTURE_PID" 2>/dev/null
        wait "$FIXTURE_PID" 2>/dev/null
    fi
    if [ -n "${ACCEPT_KEEP:-}" ]; then
        echo "kept: $W"
    else
        chmod -R u+w "$W" 2>/dev/null
        rm -rf "$W"
    fi
}
trap cleanup EXIT

PASSED=0
FAILED=0
FAILED_STEPS=""
N=0
LAST=""           # stdout+stderr of the last vibe call
CONNECTS=""       # non-loopback connect attempts seen by the current step

# Every non-loopback connect(2) in an strace log.
outside_connects() {
    grep -E 'connect\(.*AF_INET6?' "$1" 2>/dev/null | while IFS= read -r line; do
        addr="$(printf '%s' "$line" | sed -nE 's/.*inet_addr\("([^"]*)"\).*/\1/p')"
        [ -z "$addr" ] && addr="$(printf '%s' "$line" | sed -nE 's/.*inet_pton\(AF_INET6, "([^"]*)".*/\1/p')"
        case "$addr" in
            127.*|::1|::ffff:127.*) ;;
            *) printf '%s\n' "$line" ;;
        esac
    done
}

# The machine's environment: nothing inherited but PATH.
HOME_DIR="$W/home"
SETTINGS="$HOME_DIR/.vibe"
mkdir -p "$HOME_DIR" && chmod 700 "$HOME_DIR"
: > "$W/empty"
MACHINE_PATH=/usr/bin:/bin
machine_env() {
    env -i PATH="$MACHINE_PATH" HOME="$HOME_DIR" SHELL=/bin/bash TERM=dumb \
        LC_ALL=C.UTF-8 NO_COLOR=1 VIBE_SETTINGS="$SETTINGS" VIBE_NO_PROGRESS=1 \
        GIT_CONFIG_NOSYSTEM=1 ${GIT_SSH_COMMAND:+GIT_SSH_COMMAND="$GIT_SSH_COMMAND"} "$@"
}

# v <exe> <args...>: run one vibe command observed by strace; output in $LAST.
v() {
    N=$((N + 1))
    local exe="$1"; shift
    local trace="$LOGS/$N.strace" out="$LOGS/$N.out"
    printf '$ vibe %s\n' "$*" >> "$LOGS/commands.txt"
    machine_env ${ACCEPT_VIBE_ENV:-} strace -f -qq -e trace=connect -o "$trace" "$exe" "$@" > "$out" 2>&1
    local status=$?
    LAST="$(cat "$out")"
    local found
    found="$(outside_connects "$trace")"
    [ -n "$found" ] && CONNECTS="$CONNECTS$found"$'\n'
    return $status
}

# v_step: `v` inside a step; connect attempts are collected in a file so a
# step that runs part of its work in a subshell still reports them.
v_step() { v "$@"; local s=$?; [ -n "$CONNECTS" ] && printf '%s' "$CONNECTS" >> "$W/connects"; return $s; }
step() {
    local name="$1"; shift
    CONNECTS=""
    : > "$W/connects"
    local detail status
    "$@" > "$W/step.out" 2>&1
    status=$?
    detail="$(cat "$W/step.out")"
    local connects
    connects="$(cat "$W/connects")"
    if [ $status -eq 0 ] && [ -z "$connects" ]; then
        PASSED=$((PASSED + 1))
        echo "PASS $name"
    else
        FAILED=$((FAILED + 1))
        FAILED_STEPS="$FAILED_STEPS $name"
        echo "FAIL $name"
        [ -n "$connects" ] && { echo "  non-loopback connect attempts:"; printf '%s\n' "$connects" | sed 's/^/  ! /'; }
        printf '%s\n' "$detail" | tail -n 40 | sed 's/^/  | /'
    fi
}
in_dir() { local d="$1"; shift; pushd "$d" > /dev/null || return 1; "$@"; local s=$?; popd > /dev/null; return $s; }
fail() { echo "$*"; echo "--- vibe output ---"; echo "$LAST"; return 1; }
jget() { python3 -I -c 'import json,sys
d=json.load(sys.stdin)
for k in sys.argv[1].split("."):
    d=d[int(k)] if k.isdigit() else d[k]
print(d if not isinstance(d,(dict,list)) else json.dumps(d))' "$1"; }

g() { git -c user.name=Admin -c user.email=admin@example.corp "$@" > /dev/null 2>&1; }

SITE="$W/site"
DOWNLOAD="$W/download"
INSTALL_ROOT="$W/work"
SHIM="$INSTALL_ROOT/opt/bin/vibe"
mkdir -p "$SITE/releases" "$SITE/payloads" "$SITE/shared" "$DOWNLOAD"

# --------------------------------------------------------------- fixtures ---

# A package directory: pkg <dir> <group> <name> <version> <kind> <keywords> <payload>
pkg() {
    mkdir -p "$1"
    cat > "$1/vibe.toml" <<EOF
[package]
group = "$2"
name = "$3"
version = "$4"
kind = "$5"
description = "Acceptance fixture $2/$3"
keywords = [$6]
EOF
    printf '%s\n' "$7" > "$1/payload"
}

APP_PAYLOAD="corp tool payload"
APP_SHA="$(printf '%s\n' "$APP_PAYLOAD" | sha256sum | cut -d' ' -f1)"

# An application whose one input lives at a corporate download URL; the bytes
# are only in the package drop.
app_pkg() {
    mkdir -p "$1"
    cat > "$1/vibe.toml" <<EOF
[package]
name = "tool"
group = "org.corp"
kind = "app"
version = "1.0.0"
description = "Acceptance application with a dropped payload"

[application]
id = "corp-tool"

[[installer_options]]
id = "src"
name = "Build from source"
platform = "linux"
platform_name = "Linux"
arch = "$(uname -m)"
[installer_options.installation]
type = "source_build"
[[installer_options.installation.sources]]
name = "payload"
url = "https://dist.example.corp/tool/corp-tool.txt"
sha256 = "$APP_SHA"
[installer_options.installation.recipe]
interpreter = "posix_sh"
script = "build.sh"

[[installer_options.launchers]]
id = "main"
name = "Corp tool"
executable = "\$app_bin/corp-tool"
kind = "command"
default = true
EOF
    cat > "$1/build.sh" <<'EOF'
set -e
mkdir -p "$VIBE_OUT/bin"
cp "$VIBE_SOURCES"/payload/* "$VIBE_OUT/bin/corp-tool"
chmod 755 "$VIBE_OUT/bin/corp-tool"
EOF
}

# Fake vibe releases in the GitHub asset layout, one directory per version
# (<drop>/v<version>/), derived from spike-corp/scripts/make_releases.py.
make_releases() {
    python3 -I - "$@" <<'PY'
import hashlib, io, json, os, sys, zipfile
TARGETS = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-musl",
           "x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin"]
COMMIT = "0" * 39 + "1"
TREE = "0" * 39 + "2"
def sha(b): return "sha256:" + hashlib.sha256(b).hexdigest()
drop, *versions = sys.argv[1:]
buf = io.BytesIO()
with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("Cargo.toml", "[workspace]\nmembers = []\n")
    z.writestr("crates/vibe-cli/Cargo.toml", "[package]\nname='vibe-cli'\n")
src = buf.getvalue()
src_meta = {"path": "vibevm-source.zip", "size": len(src), "digest": sha(src), "tree_oid": TREE}
for ver in versions:
    out = os.path.join(drop, "v" + ver)
    os.makedirs(out, exist_ok=True)
    ident = {"schema_version": 1, "product": "vibevm", "repository": "vibevm/vibevm",
             "version": ver, "tag": "v" + ver, "source_commit": COMMIT}
    platforms = []
    for t in TARGETS:
        ext = ".exe" if t.endswith("msvc") else ""
        data = {
            "vibe": f"#!/bin/sh\necho 'vibe {ver} fake ({t})'\n".encode(),
            "vibe-index": f"#!/bin/sh\necho vibe-index {ver}\n".encode(),
            "vibe-app-proxy": f"#!/bin/sh\necho vibe-app-proxy {ver}\n".encode(),
        }
        components = [{"name": n, "path": n + ext, "size": len(data[n]), "digest": sha(data[n])}
                      for n in ["vibe", "vibe-app-proxy", "vibe-index"]]
        bundle = dict(ident, target=t, components=components, source_archive=src_meta)
        zname = f"vibevm-{ver}-{t}.zip"
        zbuf = io.BytesIO()
        with zipfile.ZipFile(zbuf, "w", zipfile.ZIP_STORED) as z:
            z.writestr("DISTRIBUTION.json", json.dumps(bundle, indent=2))
            z.writestr("vibevm-source.zip", src)
            z.writestr("LICENSE.md", "UPL-1.0 (fake)\n")
            z.writestr("README.md", "fake release\n")
            for c in components:
                info = zipfile.ZipInfo(c["path"]); info.external_attr = 0o755 << 16
                z.writestr(info, data[c["name"]])
        zbytes = zbuf.getvalue()
        open(os.path.join(out, zname), "wb").write(zbytes)
        boot = f"vibe-bootstrap-{t}{ext}"
        open(os.path.join(out, boot), "wb").write(data["vibe"])
        frag = dict(ident, target=t, asset={"name": zname, "size": len(zbytes), "digest": sha(zbytes)},
                    bootstrap={"name": boot, "size": components[0]["size"], "digest": components[0]["digest"]},
                    bundle=bundle)
        open(os.path.join(out, f"vibevm-{ver}-{t}.fragment.json"), "w").write(json.dumps(frag, indent=2))
        platforms.append(frag)
    open(os.path.join(out, "DISTRIBUTIONS.json"), "w").write(json.dumps(dict(ident, platforms=platforms), indent=2))
PY
}

setup_fixtures() {
    # The sandbox has loopback only, and the connect observer works.
    local ifaces
    ifaces="$(awk -F: 'NR > 2 { gsub(/ /, "", $1); print $1 }' /proc/self/net/dev)"
    [ "$ifaces" = lo ] || { echo "network interfaces: $ifaces"; return 1; }
    strace -f -qq -e trace=connect -o "$W/selftest.strace" \
        python3 -I -c 'import socket
try: socket.create_connection(("192.0.2.1", 443), 1)
except OSError: pass' 2>/dev/null
    [ -n "$(outside_connects "$W/selftest.strace")" ] || { echo "strace did not observe a test connect"; return 1; }

    # The "downloaded" release, placed by hand: vibe and the companion
    # binaries a release archive carries next to it.
    cp "$VIBE_SRC" "$DOWNLOAD/vibe" && chmod 755 "$DOWNLOAD/vibe" || return 1
    for companion in vibe-app-proxy vibe-index; do
        cp "$(dirname "$VIBE_SRC")/$companion" "$DOWNLOAD/$companion" \
            && chmod 755 "$DOWNLOAD/$companion" || return 1
    done

    # Loopback sshd with one writable repository; stdin held open by fd 9.
    mkdir -p "$W/sshd"
    mkfifo "$W/sshd/stdin"
    TMPDIR="$W/sshd" VIBE_SSH_FIXTURE_WRITABLE=1 python3 -u "$SSH_FIXTURE" \
        < "$W/sshd/stdin" > "$W/sshd/facts.json" 2> "$W/sshd/stderr.log" &
    FIXTURE_PID=$!
    exec 9> "$W/sshd/stdin"
    for _ in $(seq 1 200); do
        [ -s "$W/sshd/facts.json" ] && break
        sleep 0.05
    done
    [ -s "$W/sshd/facts.json" ] || { cat "$W/sshd/stderr.log"; return 1; }
    local facts="$W/sshd/facts.json"
    SSH_USER="$(jget user < "$facts")"
    SSH_PORT="$(jget port < "$facts")"
    BARE="$(jget home < "$facts")/repo.git"
    MONOREPO_URL="ssh://$SSH_USER@corp-git:$SSH_PORT$BARE"
    # The developer's ordinary ~/.ssh/config: `corp-git` is the SSH server.
    mkdir -p "$HOME_DIR/.ssh" && chmod 700 "$HOME_DIR/.ssh"
    cat > "$HOME_DIR/.ssh/config" <<EOF
Host corp-git
 HostName 127.0.0.1
 User $SSH_USER
 Port $SSH_PORT
 IdentityFile $(jget key < "$facts")
 IdentityAgent none
 IdentitiesOnly yes
 UserKnownHostsFile $(jget known_hosts < "$facts")
 GlobalKnownHostsFile /dev/null
 StrictHostKeyChecking yes
EOF
    chmod 600 "$HOME_DIR/.ssh/config"
    # OpenSSH reads ~/.ssh/config from the passwd entry's home, not $HOME, so
    # the isolated HOME's file is named explicitly for every Git child (vibe's
    # controlled transport already captures $HOME/.ssh/config itself). On a
    # real machine HOME is the passwd home and nothing needs setting.
    GIT_SSH_COMMAND="ssh -F $HOME_DIR/.ssh/config"

    # The monorepo's first content: one feat package and one application,
    # pushed by the administrator onto `main` of the bare repository.
    local work="$W/monorepo-work"
    g init -q -b main "$work" || return 1
    pkg "$work/org.corp/hello/v1.0.0" org.corp hello 1.0.0 feat '"corpwidget", "greeting"' "hello 1.0.0"
    app_pkg "$work/org.corp/tool/v1.0.0"
    (cd "$work" && g add -A && g commit -q -m seed && g push -q --force "$BARE" HEAD:refs/heads/main) || return 1

    # A per-package registry to mirror from: <org>/org.corp.mirrored.git, one tag per version.
    local up="$W/upstream-work"
    g init -q -b main "$up" || return 1
    mkdir -p "$W/upstream"
    g init -q --bare "$W/upstream/org.corp.mirrored.git" || return 1
    for ver in 1.0.0 1.1.0; do
        rm -rf "$up/payload" "$up/vibe.toml"
        pkg "$up" org.corp mirrored "$ver" feat '"mirrored"' "mirrored $ver"
        (cd "$up" && g add -A && g commit -q -m "release $ver" && g tag "v$ver") || return 1
    done
    (cd "$up" && g push -q "$W/upstream/org.corp.mirrored.git" main --tags) || return 1

    # The release drop and the package drop.
    make_releases "$SITE/releases" 1.0.12 1.0.13 || return 1
    mkdir -p "$SITE/payloads/org.corp/tool/1.0.0"
    printf '%s\n' "$APP_PAYLOAD" > "$SITE/payloads/org.corp/tool/1.0.0/corp-tool.txt"

    # Packages an internal team publishes.
    pkg "$W/internal" org.corp internal 1.0.0 feat '"internal"' "internal 1.0.0"
    pkg "$W/team" org.corp team 1.0.0 feat '"teamtool"' "team 1.0.0"

    # A local file to download over a loopback HTTP server.
    mkdir -p "$W/files"
    head -c 4096 /dev/urandom > "$W/files/blob.bin"
    HTTP_PORT="$(python3 -I -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')"
    python3 -I -m http.server "$HTTP_PORT" --bind 127.0.0.1 --directory "$W/files" \
        > "$W/http.log" 2>&1 &
    HTTP_PID=$!
    for _ in $(seq 1 100); do
        python3 -I -c "import socket;socket.create_connection(('127.0.0.1',$HTTP_PORT),.1)" 2>/dev/null && break
        sleep 0.05
    done

    # The site profile: the one file the administrator hands out.
    cat > "$SITE/corp.toml" <<EOF
# Site profile of the acceptance network.
schema = 1
name = "corp-accept"

[[registry]]
name = "corp"
type = "monorepo"
url = "$MONOREPO_URL"
ref = "main"

[[registry]]
name = "shared"
type = "localdir"
url = "file://$SITE/shared"

[source_policy]
exclude_hosting = ["github", "gitverse"]

[artifacts]
vibe = "$SITE/releases"
packages = "$SITE/payloads"

[self]
source = "localdir"
install_root = "$INSTALL_ROOT"

[path]
fix = "always"
EOF
    {
        echo "monorepo $MONOREPO_URL"
        echo "http     http://127.0.0.1:$HTTP_PORT/"
    } > "$W/fixture.txt"
}

lock_row() { # lock_row <project> <name> <field>
    python3 -I - "$1/vibe.lock" "$2" "$3" <<'PY'
import sys, tomllib
lock = tomllib.load(open(sys.argv[1], "rb"))
for row in lock.get("package", []):
    if row.get("name") == sys.argv[2]:
        print(row.get(sys.argv[3], "")); break
PY
}

project() { # project <dir> <requires lines>
    mkdir -p "$1"
    printf "[project]\nname = 'consumer'\nversion = '1.0.0'\n[requires.packages]\n%b" "$2" > "$1/vibe.toml"
}

# ------------------------------------------------------------------ steps ---

s_apply_dry_run() {
    v_step "$DOWNLOAD/vibe" --unattended self apply "$SITE/corp.toml" --dry-run || fail "dry-run failed" || return 1
    [ ! -e "$SETTINGS/site-profile.toml" ] || fail "dry-run wrote the profile"
}

s_apply() {
    v_step "$DOWNLOAD/vibe" --json --unattended self apply "$SITE/corp.toml" || fail "apply failed" || return 1
    local report="$LAST"
    [ "$(jget changed <<< "$report")" = "True" ] || fail "apply changed nothing" || return 1
    [ -x "$SHIM" ] || fail "no shim at $SHIM (install_root ignored?)" || return 1
    [ -f "$SETTINGS/site-profile.toml" ] || fail "profile not stored" || return 1
    grep -q "$INSTALL_ROOT/opt/bin" "$HOME_DIR/.bashrc" 2>/dev/null || fail "[path] fix = always did not put the shim dir on PATH" || return 1
    grep -q VIBEVM_INSTALL_ROOT "$HOME_DIR/.bashrc" || fail "install_root not made durable" || return 1
    # From here on the developer works in a new terminal: PATH and
    # VIBEVM_INSTALL_ROOT are what ~/.bashrc now says.
    MACHINE_PATH="$(env -i HOME="$HOME_DIR" PATH=/usr/bin:/bin bash -c '. "$HOME/.bashrc" >/dev/null 2>&1; printf %s "$PATH"')"
    case ":$MACHINE_PATH:" in *":$INSTALL_ROOT/opt/bin:"*) ;; *) fail "a new shell's PATH lacks the shim dir: $MACHINE_PATH"; return 1 ;; esac
}

s_apply_again() {
    v_step "$DOWNLOAD/vibe" --json --unattended self apply "$SITE/corp.toml" || fail "re-apply failed" || return 1
    [ "$(jget changed <<< "$LAST")" = "False" ] || fail "re-apply was not idempotent"
}

s_show() {
    v_step "$SHIM" --json self apply --show || fail "show failed" || return 1
    local names
    names="$(jget effective.registries <<< "$LAST")"
    grep -q '"corp"' <<< "$names" && grep -q '"shared"' <<< "$names" || fail "site registries not effective: $names" || return 1
    if grep -q 'vibespecs' <<< "$names"; then fail "built-in GitHub/GitVerse registries still effective: $names"; fi
}

s_doctor() {
    v_step "$SHIM" --unattended self doctor || fail "doctor failed"
}

s_releases() {
    v_step "$SHIM" --json self releases || fail "releases failed" || return 1
    local versions
    versions="$(python3 -I -c 'import json,sys;print(" ".join(r["version"] for r in json.load(sys.stdin)["releases"]))' <<< "$LAST")"
    [ "$versions" = "1.0.13 1.0.12" ] || fail "drop listing: '$versions'"
}

s_install_project() {
    project "$W/p1" "'org.corp/hello' = '=1.0.0'\n"
    in_dir "$W/p1" v_step "$SHIM" --unattended install || fail "install failed" || return 1
    [ "$(lock_row "$W/p1" hello registry)" = corp ] || fail "hello not locked from corp" || return 1
    case "$(lock_row "$W/p1" hello source_url)" in
        "$MONOREPO_URL#org.corp/hello/v1.0.0") ;;
        *) fail "lock source_url: $(lock_row "$W/p1" hello source_url)"; return 1 ;;
    esac
    [ -n "$(lock_row "$W/p1" hello resolved_commit)" ] || fail "no resolved_commit"
}

s_install_global_app() {
    v_step "$SHIM" --unattended install -g org.corp/tool || fail "install -g failed" || return 1
    local exe
    exe="$(find "$SETTINGS" "$INSTALL_ROOT" -path '*bin/corp-tool' -type f 2>/dev/null | head -n1)"
    [ -n "$exe" ] || fail "installed launcher payload not found" || return 1
    [ "$(cat "$exe")" = "$APP_PAYLOAD" ] || fail "payload differs: $(cat "$exe")"
}

s_artifacts_where() {
    v_step "$SHIM" --json artifacts where org.corp/tool || fail "where failed" || return 1
    [ "$(jget inputs.0.status <<< "$LAST")" = present ] || fail "payload not reported present" || return 1
    [ "$(jget inputs.0.file <<< "$LAST")" = corp-tool.txt ] || fail "wrong file name" || return 1
    [ "$(jget release.source <<< "$LAST")" = localdir ] || fail "release source not localdir"
}

s_init_publisher() {
    v_step "$SHIM" --unattended init --path "$W/pub" || fail "init failed" || return 1
    [ -f "$W/pub/vibe.toml" ] || fail "no vibe.toml"
}

s_publish_monorepo() {
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry publish "$W/internal" --registry corp || fail "publish failed" || return 1
    [ "$(jget action <<< "$LAST")" = created ] || fail "action not created" || return 1
    git --git-dir "$BARE" ls-tree -r --name-only main | grep -q '^org.corp/internal/v1.0.0/vibe.toml$' \
        || fail "version not on main of the monorepo" || return 1
    git --git-dir "$BARE" ls-tree -r --name-only main | grep -q '^_index/listing.txt$' || fail "no _index in the monorepo"
}

s_publish_monorepo_again() {
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry publish "$W/internal" --registry corp || fail "re-publish failed" || return 1
    [ "$(jget action <<< "$LAST")" = unchanged ] || fail "identical re-publish was not 'unchanged'" || return 1
    printf 'changed\n' > "$W/internal/payload"
    if in_dir "$W/pub" v_step "$SHIM" --unattended registry publish "$W/internal" --registry corp; then
        fail "a different 1.0.0 was accepted without --replace"; return 1
    fi
    printf '%s\n' "internal 1.0.0" > "$W/internal/payload"
}

s_publish_localdir() {
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry publish "$W/team" --registry shared || fail "publish failed" || return 1
    [ -f "$SITE/shared/org.corp/team/v1.0.0/vibe.toml" ] || fail "not in the shared folder" || return 1
    [ -f "$SITE/shared/_index/listing.txt" ] || fail "no _index in the shared folder"
}

s_install_published_monorepo() {
    project "$W/p2" "'org.corp/internal' = '=1.0.0'\n"
    in_dir "$W/p2" v_step "$SHIM" --unattended install || fail "install failed" || return 1
    [ "$(lock_row "$W/p2" internal registry)" = corp ] || fail "internal not from corp"
}

s_install_published_localdir() {
    # The site lists the monorepo first and the shared folder second;
    # org.corp/team exists only in the shared folder.
    project "$W/p2b" "'org.corp/team' = '=1.0.0'\n"
    in_dir "$W/p2b" v_step "$SHIM" --unattended install || fail "install failed" || return 1
    [ "$(lock_row "$W/p2b" team registry)" = shared ] || fail "team not from shared"
}

s_registry_add_upstream() {
    v_step "$SHIM" --unattended registry add upstream-user "file://$W/upstream" --type git --scope user || fail "add failed" || return 1
    v_step "$SHIM" --json registry list --scope user || fail "list failed" || return 1
    grep -q '"upstream-user"' <<< "$LAST" || fail "the user row is not listed" || return 1
    # The user layer outranks the site's: leave resolution to the site.
    v_step "$SHIM" --unattended registry remove registry upstream-user --scope user || fail "remove failed"
}

s_declare_upstream() {
    in_dir "$W/pub" v_step "$SHIM" --unattended registry add upstream "file://$W/upstream" --type git || fail "add failed" || return 1
    in_dir "$W/pub" v_step "$SHIM" --json registry list || fail "list failed" || return 1
    grep -q '"upstream"' <<< "$LAST" && grep -q '"site"' <<< "$LAST" || fail "list lacks the project or site rows"
}

s_mirror_dry_run() {
    local before
    before="$(git --git-dir "$BARE" rev-parse main)"
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry mirror upstream --into corp --package org.corp/mirrored --dry-run || fail "dry-run failed" || return 1
    [ "$(git --git-dir "$BARE" rev-parse main)" = "$before" ] || fail "dry-run pushed"
}

s_mirror_into_empty_bare() {
    # How an administrator creates a monorepo: an empty bare repository,
    # filled by the first mirror run (the branch is created by the push).
    git init -q --bare "$W/fresh.git" || return 1
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry mirror upstream --into "$W/fresh.git" --package org.corp/mirrored \
        || fail "mirror into an empty bare repository failed" || return 1
    git --git-dir "$W/fresh.git" ls-tree -r --name-only main | grep -q '^org.corp/mirrored/v1.0.0/vibe.toml$' || fail "nothing on main"
}

s_mirror() {
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry mirror upstream --into corp --package org.corp/mirrored || fail "mirror failed" || return 1
    git --git-dir "$BARE" ls-tree -r --name-only main | grep -q '^org.corp/mirrored/v1.1.0/vibe.toml$' \
        || fail "mirrored version missing on main" || return 1
    git --git-dir "$BARE" ls-tree -r --name-only main | grep -q '^_index/mirrors/upstream.toml$' || fail "no mirror record"
}

s_mirror_incremental() {
    local before
    before="$(git --git-dir "$BARE" rev-parse main)"
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry mirror upstream --into corp --package org.corp/mirrored || fail "rerun failed" || return 1
    [ "$(git --git-dir "$BARE" rev-parse main)" = "$before" ] || fail "a rerun with nothing new committed"
}

s_mirror_working_copy() {
    # A working copy of the monorepo: the mirror commits into it, never pushes.
    git clone -q "$BARE" "$W/wc" > /dev/null 2>&1 || fail "clone failed" || return 1
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry mirror upstream --into "$W/wc" --package org.corp/mirrored --keep-history \
        || fail "mirror into a working copy failed" || return 1
    ls "$W/wc/_index/mirrors/bundles/upstream/"*.bundle > /dev/null 2>&1 || fail "--keep-history wrote no bundle" || return 1
    [ "$(git -C "$W/wc" rev-parse HEAD)" != "$(git --git-dir "$BARE" rev-parse main)" ] || fail "nothing committed in the working copy" || return 1
    [ -z "$(git -C "$W/wc" status --porcelain)" ] || fail "working copy left dirty"
}

s_install_mirrored() {
    project "$W/p3" "'org.corp/mirrored' = '=1.1.0'\n"
    in_dir "$W/p3" v_step "$SHIM" --unattended install || fail "install failed" || return 1
    [ "$(lock_row "$W/p3" mirrored registry)" = corp ] || fail "mirrored not from corp"
}

s_sources_only() {
    # A project locked on the shared folder moves onto the monorepo.
    cp -r "$SITE/shared/org.corp/team" "$W/team-copy"
    in_dir "$W/pub" v_step "$SHIM" --json --unattended registry mirror shared --into corp --package org.corp/team || fail "mirror shared→corp failed" || return 1
    project "$W/p4" "'org.corp/team' = '=1.0.0'\n"
    printf "[[registry]]\nname = 'old'\ntype = 'localdir'\nurl = 'file://%s/shared'\n" "$SITE" >> "$W/p4/vibe.toml"
    in_dir "$W/p4" v_step "$SHIM" --unattended install || fail "install failed" || return 1
    [ "$(lock_row "$W/p4" team registry)" = old ] || fail "precondition: not locked on old" || return 1
    local hash
    hash="$(lock_row "$W/p4" team content_hash)"
    python3 -I - "$W/p4/vibe.toml" <<'PY'
import sys; p = sys.argv[1]; t = open(p).read(); open(p, "w").write(t[: t.index("[[registry]]")])
PY
    in_dir "$W/p4" v_step "$SHIM" --unattended update --sources-only || fail "update --sources-only failed" || return 1
    [ "$(lock_row "$W/p4" team registry)" = corp ] || fail "not moved onto corp: $(lock_row "$W/p4" team registry)" || return 1
    [ "$(lock_row "$W/p4" team content_hash)" = "$hash" ] || fail "content hash changed"
}

s_search() {
    # Outside any project: the machine's (site) registries are searched.
    in_dir "$W" v_step "$SHIM" --json search corpwidget || fail "search failed" || return 1
    grep -q 'org.corp' <<< "$LAST" && grep -q '"hello"\|org.corp/hello' <<< "$LAST" || fail "hello not found by keyword"
}

s_dl() {
    local want
    want="$(sha256sum "$W/files/blob.bin" | cut -d' ' -f1)"
    mkdir -p "$W/dl"
    v_step "$SHIM" dl "http://127.0.0.1:$HTTP_PORT/blob.bin" "$W/dl/" --sha256 "$want" || fail "dl failed" || return 1
    cmp -s "$W/files/blob.bin" "$W/dl/blob.bin" || fail "downloaded bytes differ" || return 1
    if v_step "$SHIM" dl "http://127.0.0.1:$HTTP_PORT/blob.bin" "$W/dl/again.bin" --sha256 "$(printf '0%.0s' $(seq 64))"; then
        fail "a wrong --sha256 was accepted"; return 1
    fi
    [ ! -e "$W/dl/again.bin" ] || fail "a mismatching body was kept"
}

s_self_update() {
    v_step "$SHIM" --unattended self update || fail "self update failed" || return 1
    local out
    out="$(machine_env "$SHIM" 2>&1)"
    grep -q 'vibe 1.0.13 fake' <<< "$out" || fail "the shim does not run 1.0.13: $out" || return 1
    # Back to the real binary, through the hand-placed one.
    v_step "$DOWNLOAD/vibe" --unattended self rollback || fail "rollback failed" || return 1
    v_step "$SHIM" --version || fail "shim broken after rollback" || return 1
    grep -q '^vibe 1\.0\.11' <<< "$LAST" || fail "rollback did not return to 1.0.11"
}

s_apply_remove() {
    v_step "$DOWNLOAD/vibe" --json --unattended self apply --remove || fail "remove failed" || return 1
    [ ! -e "$SETTINGS/site-profile.toml" ] || fail "profile still stored"
}

echo "workspace: $W (network: loopback only)"
step "fixtures: sshd monorepo, shared folder, drops, upstream, site profile" setup_fixtures || true
if [ -n "${ACCEPT_HOOK:-}" ]; then   # debugging: run a script against the live fixtures
    export W HOME_DIR SETTINGS MONOREPO_URL BARE SITE
    bash "$ACCEPT_HOOK"
    exit $?
fi
if [ "$FAILED" -ne 0 ]; then
    echo "VERDICT: FAIL (fixtures could not be built)"
    exit 1
fi
step "self apply --dry-run writes nothing"                    s_apply_dry_run
step "self apply corp.toml (downloaded binary imports itself)" s_apply
step "self apply again is a no-op"                             s_apply_again
step "self apply --show: site registries replace defaults"     s_show
step "self doctor"                                             s_doctor
step "self releases lists the release drop"                    s_releases
step "install project packages from the SSH monorepo"         s_install_project
step "install -g an app whose payload lives in the drop"      s_install_global_app
step "artifacts where org.corp/tool"                           s_artifacts_where
step "init a publisher project"                                s_init_publisher
step "registry publish into the monorepo"                      s_publish_monorepo
step "registry publish: identical is unchanged, different refused" s_publish_monorepo_again
step "registry publish into the shared folder"                 s_publish_localdir
step "install what was published into the monorepo"            s_install_published_monorepo
step "install what was published into the shared folder"       s_install_published_localdir
step "registry add --type git --scope user file://… (per-package upstream)" s_registry_add_upstream
step "registry add the per-package upstream to vibe.toml; registry list" s_declare_upstream
step "registry mirror --dry-run writes nothing"                s_mirror_dry_run
step "registry mirror into a new empty bare repository"       s_mirror_into_empty_bare
step "registry mirror upstream into the monorepo"              s_mirror
step "registry mirror rerun is incremental (no commit)"        s_mirror_incremental
step "registry mirror --keep-history into a working copy"     s_mirror_working_copy
step "install a mirrored package"                              s_install_mirrored
step "update --sources-only moves a project onto the monorepo" s_sources_only
step "search outside a project finds a package by keyword"     s_search
step "dl of a local file (loopback HTTP), --sha256"            s_dl
step "self update from the release drop, rollback"             s_self_update
step "self apply --remove"                                     s_apply_remove

echo "steps: $PASSED passed, $FAILED failed"
if [ "$FAILED" -eq 0 ]; then
    echo "VERDICT: PASS (zero non-loopback connection attempts)"
    exit 0
fi
echo "VERDICT: FAIL:$FAILED_STEPS" | sed 's/:  */: /'
exit 1
