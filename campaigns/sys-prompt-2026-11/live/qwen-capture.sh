#!/usr/bin/env bash
# Measure, offline, what Qwen Code actually puts in a request — and whether
# its one additive per-launch slot can carry a real lane.
#
# It starts a loopback OpenAI-compatible provider that records every request
# body (`opencode-fake-provider.py`, shared with the OpenCode capture),
# points Qwen Code at it from an isolated `HOME` / `QWEN_HOME`, and runs the
# same prompt against the sample project three times:
#
#   `read`     plain, nothing passed — the shipped tier. The managed router
#              block must arrive (Qwen Code reads `AGENTS.md` itself) and no
#              lane byte with it.
#   `append`   `--append-system-prompt "$(cat <lane>)"`, the only ADDITIVE
#              per-launch slot Qwen Code has. It takes **text**, so this leg
#              says where that text lands and what else is around it.
#   `carrier`  the file-based slot: `QWEN_WRITE_SYSTEM_MD` has the installed
#              Qwen Code write its OWN base prompt to a file, the lane is
#              appended to that copy, and the result is handed back through
#              `QWEN_SYSTEM_MD` — which *replaces* the base prompt, so the
#              copy is what keeps Qwen's own instructions intact. The leg
#              checks the composition byte for byte against the `read` run:
#              Qwen's base, then the lane exactly once, then everything the
#              plain run had after its base, unchanged.
#   `size`     the additive slot at the lane's real size, as an `execve`
#              probe. One argument is capped at 128 KiB on Linux, so a real
#              lane cannot go through it, and the errno is the measurement
#              (PROP-076 `#SYS-QWEN`, `HARNESS-DELIVERY.md` §6).
#
# The `carrier` leg also records the dump's **interaction-mode variance**,
# which is what keeps the tier closed: `QWEN_WRITE_SYSTEM_MD` writes the
# variant of the run that produced it, and a run without a terminal can only
# ever be the headless one. The interactive variant is 2,823 bytes longer on
# 0.27.2 and cannot be obtained without allocating a pty, so a launcher
# cannot know which copy a given launch needs. That number is printed rather
# than argued.
#
# No model is ever called and no real provider is ever reached. The script
# re-execs itself under `bwrap --unshare-net` when bubblewrap is available,
# so the only reachable endpoint is `127.0.0.1` by construction rather than
# by configuration; set `QWEN_CAPTURE_SEALED=1` to skip that (and say so in
# whatever the capture is recorded in). The recorder drops the
# `authorization` header before writing, so a captured file can never carry
# a credential.
#
# usage: qwen-capture.sh <sample-project> <out-dir> [qwen]
#
#   <sample-project>  a project made by `make-sample.sh`
#   <out-dir>         created; holds `read/`, `append/` and `report.txt`
#   [qwen]            the Qwen Code binary (default: `qwen` on PATH)

set -eu

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# **The network namespace, taken before anything else runs.** A capture that
# proves "no model was called" by reading a configuration file proves it
# about the configuration; this proves it about the process.
if [ "${QWEN_CAPTURE_SEALED:-0}" != "1" ] && command -v bwrap >/dev/null 2>&1; then
    exec bwrap --unshare-net --dev-bind / / --proc /proc \
        env QWEN_CAPTURE_SEALED=1 "${BASH_SOURCE[0]}" "$@"
fi

PROJECT="$(cd "${1:?usage: qwen-capture.sh <sample-project> <out-dir> [qwen]}" && pwd)"
OUT="${2:?usage: qwen-capture.sh <sample-project> <out-dir> [qwen]}"
QWEN="${3:-qwen}"
LANE="$PROJECT/vibevm/vibespecs/boot/AGENT-LANE.md"

[ -e "$OUT" ] && { echo "refusing: $OUT exists" >&2; exit 2; }
[ -r "$LANE" ] || { echo "no lane at $LANE — run make-sample.sh first" >&2; exit 2; }
command -v "$QWEN" >/dev/null 2>&1 || { echo "no qwen: $QWEN" >&2; exit 2; }

mkdir -p "$OUT/read" "$OUT/append" "$OUT/carrier"
OUT="$(cd "$OUT" && pwd)"
HOMEDIR="$OUT/home"
mkdir -p "$HOMEDIR/.qwen" "$OUT/composed"

PORT="${PORT:-8751}"

run() {           # run <capture-dir> [extra qwen arguments…]
  local capture="$1"; shift
  CAPTURE_DIR="$capture" python3 -I "$HERE/opencode-fake-provider.py" "$PORT" &
  local server=$!
  sleep 1
  (
    cd "$PROJECT"
    # The fake provider answers JSON where Qwen Code asked for SSE, so the
    # run ends in an API error. That is expected and harmless: the claim
    # under test is the REQUEST, and the request is already on disk.
    env -i PATH="$PATH" HOME="$HOMEDIR" QWEN_HOME="$HOMEDIR/.qwen" \
        NO_COLOR=1 TERM=dumb \
      ${QWEN_SYSTEM_MD:+"QWEN_SYSTEM_MD=$QWEN_SYSTEM_MD"} \
      timeout 300 "$QWEN" \
        --auth-type openai --openai-api-key not-a-real-key \
        --openai-base-url "http://127.0.0.1:$PORT/v1" -m fake-1 \
        "$@" -p "say ok" >/dev/null 2>&1 || true
  )
  kill "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
}

run "$OUT/read"

# **The composed carrier.** The dump comes from a probe that reaches no
# provider at all: with no prompt and stdin closed, Qwen Code writes the
# base prompt and then refuses for want of input, so this costs nothing and
# calls nothing. `--safe-mode` keeps the probe from running hooks,
# extensions, skills or MCP servers — and the dump it writes is byte-identical
# to the one a plain run writes, which is why the probe may be safe-moded at
# all.
BASE="$OUT/composed/qwen-base.md"
(
  cd "$PROJECT"
  env -i PATH="$PATH" HOME="$HOMEDIR" QWEN_HOME="$HOMEDIR/.qwen" \
      NO_COLOR=1 TERM=dumb QWEN_WRITE_SYSTEM_MD="$BASE" \
    timeout 120 "$QWEN" --safe-mode \
      --auth-type openai --openai-api-key not-a-real-key \
      --openai-base-url "http://127.0.0.1:$PORT/v1" -m fake-1 \
      </dev/null >/dev/null 2>&1 || true
)
if [ -s "$BASE" ]; then
    COMPOSED="$OUT/composed/qwen-base-plus-lane.md"
    { cat "$BASE"; printf '\n\n'; cat "$LANE"; } > "$COMPOSED"
    QWEN_SYSTEM_MD="$COMPOSED" run "$OUT/carrier"
else
    echo "the base-prompt probe wrote nothing" >"$OUT/carrier/skipped"
fi
# The lane as TEXT on the command line, which is the only shape this slot
# takes. It works here only because `make-sample.sh` was called without a
# padding argument; the `size` leg below is the same slot at real size.
if [ "$(wc -c <"$LANE")" -lt 100000 ]; then
    run "$OUT/append" --append-system-prompt "$(cat "$LANE")"
else
    # Not attempted, and the reason is the finding rather than a gap: the
    # shell would fail with `Argument list too long` before Qwen Code
    # started, which the `execve` probe below reports as an errno.
    echo "lane too large for one argument" >"$OUT/append/skipped"
fi

# **The interactive variant, which is why the tier stays closed.** It needs a
# terminal: `--prompt-interactive` refuses piped stdin, and without a tty the
# run is headless whatever is asked of it. So this allocates a pty, waits for
# the dump and kills the session — which a launcher must not do at every
# launch, and which is exactly the finding.
python3 -I - "$OUT/composed/qwen-interactive.md" "$HOMEDIR" "$QWEN" "$PORT" <<'PTY' >/dev/null 2>&1 || true
import os, pty, signal, sys, time

target, home, qwen, port = sys.argv[1:5]
env = {
    "PATH": os.environ["PATH"], "HOME": home, "QWEN_HOME": home + "/.qwen",
    "TERM": "xterm-256color", "NO_COLOR": "1", "QWEN_WRITE_SYSTEM_MD": target,
}
pid, fd = pty.fork()
if pid == 0:
    os.execvpe(qwen, [qwen, "--safe-mode", "--auth-type", "openai",
                      "--openai-api-key", "not-a-real-key",
                      "--openai-base-url", f"http://127.0.0.1:{port}/v1",
                      "-m", "fake-1", "-i", "probe"], env)
deadline = time.time() + 90
while time.time() < deadline and not os.path.exists(target):
    try:
        os.read(fd, 65536)
    except OSError:
        break
    time.sleep(0.2)
try:
    os.kill(pid, signal.SIGKILL)
except ProcessLookupError:
    pass
PTY

python3 -I - "$LANE" "$OUT" <<'PY' | tee "$OUT/report.txt"
import glob, json, os, subprocess, sys

lane = open(sys.argv[1], encoding="utf-8").read()
out = sys.argv[2]


def bodies(directory):
    return [f for f in sorted(glob.glob(os.path.join(directory, "req-*.json")))
            if not f.endswith(".headers.json")]


def request(directory):
    files = bodies(directory)
    return None if not files else json.load(open(files[-1], encoding="utf-8"))


def texts(message):
    content = message["content"]
    pieces = content if isinstance(content, list) else [content]
    return [piece["text"] if isinstance(piece, dict) else piece for piece in pieces]


def blob(body, role):
    parts = []
    for message in body["messages"]:
        if message["role"] == role:
            parts.extend(texts(message))
    return "\n".join(parts)


def at(text, needle):
    """A BYTE offset. `str.find` answers in characters, and a record that
    mixes the two cannot be compared with `wc -c`."""
    index = text.find(needle)
    return -1 if index < 0 else len(text[:index].encode())


CANARIES = [
    "CANARY-LANE-HEAD-OBSIDIAN",
    "CANARY-LANE-TAIL-PUMICE",
    "CANARY-INDEX-CORE-BASALT",
    "CANARY-INDEX-USER-GNEISS",
    "CANARY-ENTRYPOINT-QUARTZ",
    "CANARY-TEMPLATE-SLATE",
]

composed = os.path.join(out, "composed")
base = os.path.join(composed, "qwen-base.md")
interactive = os.path.join(composed, "qwen-interactive.md")


def size(path):
    return os.path.getsize(path) if os.path.exists(path) else -1


print(f"lane file                : {len(lane.encode())} bytes")
print(f"Qwen base prompt, dumped : {size(base)} bytes (headless), "
      f"{size(interactive)} bytes (interactive)")
if size(base) > 0 and size(interactive) > 0:
    print(f"    the two variants differ by {size(interactive) - size(base)} bytes — a launcher "
          f"cannot know which one a launch needs without allocating a pty")
for name, directory in (("read, nothing passed", "read"),
                        ("append-system-prompt", "append"),
                        ("QWEN_SYSTEM_MD carrier", "carrier")):
    skipped = os.path.join(out, directory, "skipped")
    if os.path.exists(skipped):
        print(f"{name:24}: NOT ATTEMPTED — "
              f"{open(skipped, encoding='utf-8').read().strip()}")
        continue
    body = request(os.path.join(out, directory))
    if body is None:
        print(f"{name:24}: NO REQUEST CAPTURED")
        continue
    system = blob(body, "system")
    user = blob(body, "user")
    print(f"{name:24}: system {len(system.encode())} bytes "
          f"(x{len([m for m in body['messages'] if m['role'] == 'system'])}), "
          f"user turn {len(user.encode())} bytes, lane x{system.count(lane)}, "
          f"tools {len(json.dumps(body.get('tools', [])).encode())} bytes")
    for canary in CANARIES:
        print(f"    {canary:26} system {system.count(canary)}  "
              f"user {user.count(canary)}")
    print("    system-prompt byte offsets: "
          f"context-files {at(system, '--- Context from:')}, "
          f"router block {at(system, '# Session boot')}, "
          f"lane {at(system, lane[:200])}, "
          f"auto-memory {at(system, '# auto memory')}, "
          f"date {at(system, chr(39).join(('Today', 's date')))}")
    print("    user-turn byte offsets:     "
          f"date {at(user, chr(39).join(('Today', 's date')))}, "
          f"cwd {at(user, 'currently working in the directory')}")
    if directory == "carrier" and os.path.exists(base):
        # **The composition, byte for byte.** "The lane arrived" is not the
        # claim being made here — the claim is that Qwen Code's own prompt
        # arrived too, unchanged, with the lane as a suffix of it.
        own = open(base, encoding="utf-8").read().rstrip()
        plain = blob(request(os.path.join(out, "read")), "system")
        index = system.find(lane)
        print(f"    composition: starts with Qwen's own base prompt: "
              f"{system.startswith(own)}; "
              f"after the lane, byte-identical to the plain run after its base: "
              f"{system[index + len(lane):] == plain[len(own):]}")

# **The size question, asked of the kernel rather than of the documentation.**
# `execve` is what a launch does, and one argument of a real lane's size is
# `E2BIG` before any program starts.
try:
    subprocess.run(["/bin/true", "--append-system-prompt", lane], check=False)
    verdict = "accepted"
except OSError as error:
    verdict = f"{error.strerror} (errno {error.errno})"
print(f"execve with the lane as one argument: {verdict}")
PY
