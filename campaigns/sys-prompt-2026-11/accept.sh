#!/usr/bin/env bash
# Whole-campaign acceptance for the sys-prompt campaign (PLAN.md §7).
#
# One scripted scenario in a network-isolated sandbox with a fresh HOME. The
# run re-executes itself inside `bwrap --unshare-net` (only loopback exists)
# and every `vibe` invocation runs under `strace -f -e trace=connect`: any
# connect(2) to a non-loopback address fails the run even though it could not
# have succeeded. **It never calls a paid API.** The live cache measurement is
# a separate, separately authorised exercise (PROP-076 `#SYS-CACHE-ECONOMICS`,
# `#SYS-OWNER-QUESTIONS` Q5) and is not part of this script.
#
# What it sets up, all under one temp directory removed at the end:
#   * an isolated HOME and VIBE_SETTINGS — nothing of the operator's is read;
#   * a directory registry holding one statically-linked boot package, so the
#     fixture project has a real compiled `STATIC.*` lane;
#   * a project that opts in by writing `SYSTEM-TEMPLATE.md`, with canaries in
#     the template, the entrypoint's authored prose and a user-local file the
#     authored boot text only mentions;
#   * a second checkout of the same project at a different path, for the
#     determinism step.
#
# usage: accept.sh [path/to/vibe]       (default: <repo>/target/debug/vibe)
# env:   ACCEPT_TMP=<dir>  where the temp directory is created (default /tmp)
#        ACCEPT_KEEP=1     keep the temp directory (logs/<n>.out, <n>.strace)
#        ACCEPT_SKIP_CARGO=1  skip the two steps that run `cargo test`, for a run
#                          that only exercises the shipped binary
#        CARGO_TARGET_DIR  honoured and passed through; set it, or the two
#                          cargo steps pay a cold build into <repo>/target
# exit:  0 when every step passed, 1 otherwise, 2 when a prerequisite is missing.

set -u

SCRIPT="$(readlink -f "${BASH_SOURCE[0]}")"
REPO="$(cd "$(dirname "$SCRIPT")/../.." && pwd)"

if [ -z "${ACCEPT_IN_SANDBOX:-}" ]; then
    VIBE_SRC="$(readlink -f "${1:-$REPO/target/debug/vibe}")"
    missing=""
    for tool in bwrap strace python3; do
        command -v "$tool" >/dev/null 2>&1 || missing="$missing $tool"
    done
    [ -x "$VIBE_SRC" ] || missing="$missing $VIBE_SRC (build it: cargo build -p vibe-cli)"
    if [ -n "$missing" ]; then
        echo "FAIL prerequisites: missing$missing"
        exit 2
    fi
    if ! bwrap --dev-bind / / --unshare-net true 2>/dev/null; then
        echo "FAIL prerequisites: bwrap --unshare-net is not permitted here"
        exit 2
    fi
    bwrap --dev-bind / / --proc /proc --unshare-net --unshare-pid --die-with-parent \
        env ACCEPT_IN_SANDBOX=1 ACCEPT_VIBE="$VIBE_SRC" bash "$SCRIPT"
    exit $?
fi

# ---------------------------------------------------------------- sandbox ---

VIBE="$ACCEPT_VIBE"
W="$(mktemp -d "${ACCEPT_TMP:-/tmp}/vibe-sysprompt-accept.XXXXXX")"
LOGS="$W/logs"
mkdir -p "$LOGS"
if [ -z "${ACCEPT_KEEP:-}" ]; then
    trap 'rm -rf "$W"' EXIT
else
    trap 'echo "kept: $W"' EXIT
fi

HOME_DIR="$W/home"
SETTINGS="$W/settings"
PROJECT="$W/project"
MIRROR="$W/mirror"
REGISTRY="$W/registry"
mkdir -p "$HOME_DIR" "$SETTINGS"

BOOT="vibevm/vibespecs/boot"
PASS=0
FAIL=0
STEP=0

# Canaries. Each one answers a different question, and none of them may end
# up anywhere the step that names it forbids.
TEMPLATE_CANARY="ACCEPT-TEMPLATE-CANARY-3f91"
ENTRY_CANARY="ACCEPT-ENTRYPOINT-CANARY-7b24"
LANE_CANARY="ACCEPT-LANE-CANARY-c508"
DYNAMIC_CANARY="ACCEPT-DYNAMIC-CANARY-a1d6"
USER_LOCAL_CANARY="ACCEPT-USER-LOCAL-CANARY-e73f"

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

# Run `vibe` under strace, with the isolated home. Any connect(2) outside
# loopback fails the run: the commands under test promise no network, and a
# promise is not a measurement.
vibe() {
    local out="$LOGS/$STEP.out" trace="$LOGS/$STEP.strace"
    env -i \
        HOME="$HOME_DIR" \
        PATH="/usr/bin:/bin" \
        TMPDIR="$W/tmp" \
        VIBE_SETTINGS="$SETTINGS" \
        VIBE_REGISTRY_CACHE="$W/cache" \
        VIBE_NO_DEFAULT_REGISTRY=1 \
        strace -f -o "$trace" -e trace=connect \
        "$VIBE" "$@" >"$out" 2>&1
    local code=$?
    if grep -E 'connect\(' "$trace" 2>/dev/null \
        | grep -v 'sin_addr=inet_addr("127\.' \
        | grep -v 'sun_path' \
        | grep -qv 'inet_pton(AF_INET6, "::1"'; then
        echo "non-loopback connect(2) attempt:" >&2
        grep -E 'connect\(' "$trace" >&2
        FAIL=$((FAIL + 1))
    fi
    return $code
}

out() { cat "$LOGS/$STEP.out"; }

# ------------------------------------------------------------- the fixture ---

mkdir -p "$W/tmp"
mkdir -p "$REGISTRY/org.accept/carrier/v1.0.0/boot"
cat >"$REGISTRY/org.accept/carrier/v1.0.0/vibe.toml" <<EOF
[package]
group = "org.accept"
name = "carrier"
kind = "flow"
version = "1.0.0"
format = "simple"

[boot_snippet]
source = "boot/main.md"
EOF
cat >"$REGISTRY/org.accept/carrier/v1.0.0/boot/main.md" <<EOF
# carrier {#root}

$LANE_CANARY
EOF

build_project() {
    local root="$1"
    mkdir -p "$root/$BOOT" "$root/home/.vibe/steward"
    cat >"$root/vibe.toml" <<EOF
[project]
group = "org.accept"
name = "demo"
version = "0.1.0"
spec_format = "markdown"

[requires.packages]
"org.accept/carrier" = { version = "=1.0.0", link = "static" }

[[registry]]
name = "local"
url = "file://$REGISTRY"
EOF
    cat >"$root/AGENTS.md" <<EOF
# demo

Authored prose: $ENTRY_CANARY
EOF
    cat >"$root/$BOOT/00-core.md" <<EOF
# core

Project conventions.
EOF
    cat >"$root/$BOOT/90-user.md" <<EOF
# user

$DYNAMIC_CANARY
EOF
    cat >"$root/$BOOT/50-mentions.md" <<'EOF'
# mentions

Resolve `~/.vibe/steward/contexts/*/plan.toml` and `~/.vibe/zai.api.token`
before starting.
EOF
    cat >"$root/$BOOT/SYSTEM-TEMPLATE.md" <<EOF
# My frame

$TEMPLATE_CANARY
EOF
    # The sentinels genuinely exist, so step 10 is not vacuous.
    printf '%s\n' "$USER_LOCAL_CANARY" >"$root/home/.vibe/steward/plan.toml"
    printf '%s\n' "$USER_LOCAL_CANARY" >"$root/home/.vibe/steward/custody.toml"
    printf '%s\n' "$USER_LOCAL_CANARY" >"$root/home/.vibe/zai.api.token"
}

build_project "$PROJECT"

# ----------------------------------------------------------------- the run ---

echo "sys-prompt acceptance — $W"

step "install publishes the whole generated set in one commit"
if vibe install --offline --assume-yes --path "$PROJECT"; then
    want="INDEX.md STATIC.md SYSTEM.md SYSTEM.meta.json AGENT-LANE.md .gitignore"
    present=1
    for name in $want; do
        [ -f "$PROJECT/$BOOT/$name" ] || { present=0; bad "$name is absent"; break; }
    done
    [ "$present" = 1 ] && ok
else
    bad "install failed: $(out | tail -3)"
fi

step "the install summary line names what happened and where"
grep -q "system prompt: " "$LOGS/1.out" && ok || bad "no summary line: $(cat "$LOGS/1.out" | tail -3)"

step "the ignore file covers the three artifacts and itself"
if grep -q '/SYSTEM.md' "$PROJECT/$BOOT/.gitignore" \
    && grep -q '/SYSTEM.meta.json' "$PROJECT/$BOOT/.gitignore" \
    && grep -q '/AGENT-LANE.md' "$PROJECT/$BOOT/.gitignore" \
    && grep -q '/.gitignore' "$PROJECT/$BOOT/.gitignore"; then
    ok
else
    bad "$(cat "$PROJECT/$BOOT/.gitignore")"
fi

step "check is equal, and writes nothing"
BEFORE="$(find "$PROJECT/$BOOT" -type f -exec sha256sum {} + | sort)"
if vibe prompt check --path "$PROJECT" && grep -q "system prompt: equal" "$LOGS/$STEP.out"; then
    AFTER="$(find "$PROJECT/$BOOT" -type f -exec sha256sum {} + | sort)"
    [ "$BEFORE" = "$AFTER" ] && ok || bad "a read-only verb wrote"
else
    bad "$(out | tail -3)"
fi

step "a second checkout at another path composes identical bytes"
build_project "$MIRROR"
if vibe install --offline --assume-yes --path "$MIRROR"; then
    same=1
    for name in SYSTEM.md SYSTEM.meta.json AGENT-LANE.md; do
        cmp -s "$PROJECT/$BOOT/$name" "$MIRROR/$BOOT/$name" || { same=0; bad "$name differs by checkout path"; break; }
    done
    [ "$same" = 1 ] && ok
else
    bad "the mirror install failed: $(out | tail -3)"
fi

step "an edit of a dynamic boot file leaves check equal"
printf '# user\n\n%s edited\n' "$DYNAMIC_CANARY" >"$PROJECT/$BOOT/90-user.md"
if vibe prompt check --path "$PROJECT" && grep -q "system prompt: equal" "$LOGS/$STEP.out"; then
    ok
else
    bad "a band-U edit moved the verdict: $(out | tail -3)"
fi

step "an edit of the entrypoint makes it stale, naming the entrypoint"
printf '\nOne more authored sentence.\n' >>"$PROJECT/AGENTS.md"
vibe prompt check --path "$PROJECT" --json >/dev/null
if grep -q '"verdict": *"stale"' "$LOGS/$STEP.out" && grep -q 'entrypoint' "$LOGS/$STEP.out"; then
    ok
else
    bad "$(out | tail -5)"
fi

step "prepare refuses a stale prefix rather than sending it"
if vibe prompt prepare --json --path "$PROJECT"; then
    bad "prepare served a stale prefix"
else
    grep -q "vibe prompt build" "$LOGS/$STEP.out" && ok || bad "no remedy named"
fi

step "an explicit build makes it current again, and a second writes nothing"
vibe prompt build --path "$PROJECT" >/dev/null
BEFORE="$(find "$PROJECT/$BOOT" -type f -printf '%p %T@ ' | sort)"
vibe prompt build --path "$PROJECT" >/dev/null
AFTER="$(find "$PROJECT/$BOOT" -type f -printf '%p %T@ ' | sort)"
if [ "$BEFORE" = "$AFTER" ] && grep -q "system prompt: unchanged" "$LOGS/$STEP.out"; then
    ok
else
    bad "an idempotent build rewrote something"
fi

step "prepare returns the bands separately, never concatenated"
vibe prompt prepare --json --path "$PROJECT" >/dev/null
if python3 -I -c '
import json,sys
d=json.load(open(sys.argv[1]))
assert d["prefix"]["len"] > 0, d
assert isinstance(d["unconditional"], list) and d["unconditional"], d
assert d["cache_boundaries"], d
print("bands", 1 + len(d["unconditional"]), "boundaries", len(d["cache_boundaries"]))
' "$LOGS/$STEP.out" >"$LOGS/$STEP.bands" 2>&1; then
    ok
else
    bad "$(cat "$LOGS/$STEP.bands")"
fi

step "the user-local sentinels are in no artifact and in no envelope"
vibe prompt capture --json --with-text --task "do the thing" --path "$PROJECT" >/dev/null
leaked=""
for name in SYSTEM.md SYSTEM.meta.json AGENT-LANE.md INDEX.md STATIC.md; do
    grep -q "$USER_LOCAL_CANARY" "$PROJECT/$BOOT/$name" && leaked="$leaked $name"
done
grep -q "$USER_LOCAL_CANARY" "$LOGS/$STEP.out" && leaked="$leaked envelope"
if [ -z "$leaked" ] && grep -q 'steward/contexts' "$PROJECT/$BOOT/AGENT-LANE.md"; then
    ok
else
    bad "leaked into:$leaked (or the authored mention was rewritten)"
fi

step "the entrypoint's text crosses the envelope exactly once"
count=$(python3 -I -c '
import json,sys
d=json.load(open(sys.argv[1]))
text=json.dumps(d)
print(text.count(sys.argv[2]))
' "$LOGS/$((STEP - 1)).out" "$ENTRY_CANARY")
[ "$count" = "1" ] && ok || bad "the entrypoint canary appears $count times"

step "the differential parity test passes"
if [ -n "${ACCEPT_SKIP_CARGO:-}" ]; then
    echo "skipped (ACCEPT_SKIP_CARGO)"
elif (cd "$REPO" && cargo test -p vibe-workspace --test system_prompt_parity --offline --locked -q >"$LOGS/$STEP.parity" 2>&1); then
    ok
else
    bad "$(tail -20 "$LOGS/$STEP.parity")"
fi

step "the matrix and the stale sweep pass"
if [ -n "${ACCEPT_SKIP_CARGO:-}" ]; then
    echo "skipped (ACCEPT_SKIP_CARGO)"
elif (cd "$REPO" && cargo test -p vibe-cli --test cli_prompt_matrix --test cli_prompt_stale \
    --offline --locked -q >"$LOGS/$STEP.matrix" 2>&1); then
    ok
else
    bad "$(tail -20 "$LOGS/$STEP.matrix")"
fi

step "a hand-edited prefix is stale and rebuilt, not blessed as current"
# PLAN.md §7 wrote this step as an ownership-conflict refusal. P4 retired
# that refusal when the owner made the artifacts untracked (`SP-FMT-010`):
# an untracked, fully derivable artifact whose bytes disagree with its
# sidecar is stale, and the honest answer is to rebuild it. The conflict
# that survives is the git-tracked one, which the Rust suite asserts.
printf '\nhand-edited\n' >>"$PROJECT/$BOOT/SYSTEM.md"
vibe prompt check --path "$PROJECT" --json >/dev/null
if grep -q '"verdict": *"stale"' "$LOGS/$STEP.out"; then
    vibe prompt build --path "$PROJECT" >/dev/null
    grep -q "hand-edited" "$PROJECT/$BOOT/SYSTEM.md" && bad "the hand edit survived a rebuild" || ok
else
    bad "a hand-edited prefix was not reported stale: $(out | tail -5)"
fi

step "opting out removes exactly the artifacts vibe owns"
rm "$PROJECT/$BOOT/SYSTEM-TEMPLATE.md"
if vibe install --offline --assume-yes --path "$PROJECT"; then
    gone=1
    for name in SYSTEM.md SYSTEM.meta.json AGENT-LANE.md .gitignore; do
        [ -e "$PROJECT/$BOOT/$name" ] && { gone=0; bad "$name survived the opt-out"; break; }
    done
    for name in INDEX.md STATIC.md 00-core.md 90-user.md; do
        [ -f "$PROJECT/$BOOT/$name" ] || { gone=0; bad "$name was removed and is not ours"; break; }
    done
    [ "$gone" = 1 ] && ok
else
    bad "the opt-out install failed: $(out | tail -3)"
fi

step "a project that never opted in boots exactly as before"
# The router alone must still direct a reader to STATIC first, then INDEX.
if grep -q 'read it from disk — `vibevm/vibespecs/boot/STATIC.md`' "$PROJECT/AGENTS.md" \
    && grep -q 'first, then INDEX order.' "$PROJECT/AGENTS.md"; then
    ok
else
    bad "the classic router does not point at STATIC then INDEX"
fi

echo
echo "steps: $PASS passed, $FAIL failed"
[ "$FAIL" = 0 ] || exit 1
exit 0
