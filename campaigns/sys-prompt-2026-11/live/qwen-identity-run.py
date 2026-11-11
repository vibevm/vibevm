#!/usr/bin/env python3
"""Run Qwen Code once against the loopback fake provider and keep the requests.

One leg of the `QWEN_SYSTEM_IDENTITY_MD` carrier proof (PROP-076
`#SYS-QWEN`). It starts the program a `--launcher` prefix names — `qwen`
itself for a plain run, `<vibe> agent qwen` for a carrier run, so what the
proof measures is the shipped launcher rather than a hand-made file.

It is meant to be started inside `bwrap --unshare-net` (which
`qwen-identity-prove.sh` does for every leg), so
127.0.0.1 is the only reachable endpoint by construction.

  --mode headless   `qwen ... -p PROMPT` with stdin closed (no terminal)
  --mode tty        the real interactive TUI under a pseudo-terminal; the
                    prompt is passed with `-i` (default) or typed into the
                    TUI after it settles (`--type-prompt`)

The Qwen process gets a minimal environment (PATH, HOME, QWEN_HOME, TERM,
NO_COLOR) plus every `--env K=V`; nothing of the caller's environment leaks
in. The fake provider's captures land in <out>/req-NNN.json.
"""
import argparse
import fcntl
import glob
import os
import pty
import select
import shlex
import signal
import struct
import subprocess
import sys
import termios
import time

HERE = os.path.dirname(os.path.abspath(__file__))


def request_count(out):
    return len([f for f in glob.glob(os.path.join(out, "req-*.json"))
                if not f.endswith(".headers.json")])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--project", required=True)
    ap.add_argument("--home", required=True)
    ap.add_argument("--mode", choices=["headless", "tty"], required=True)
    ap.add_argument("--prompt", default="say ok MAINPROMPT-7731")
    ap.add_argument("--type-prompt", action="store_true")
    ap.add_argument("--no-prompt", action="store_true",
                    help="tty mode: start the TUI and send nothing")
    ap.add_argument("--env", action="append", default=[])
    ap.add_argument("--spawn", action="store_true",
                    help="have the provider answer the first turn with an agent tool call")
    ap.add_argument("--spawn-type", default="general-purpose")
    ap.add_argument("--spawn-direct", action="store_true")
    ap.add_argument("--marker", default="MAINPROMPT-7731")
    ap.add_argument("--script", default=None,
                    help="JSON list of [tool, args] steps for the main session (overrides --spawn)")
    ap.add_argument("--want", type=int, default=1, help="requests to wait for")
    ap.add_argument("--timeout", type=float, default=120)
    ap.add_argument("--linger", type=float, default=2.0)
    ap.add_argument("--port", type=int, default=8761)
    ap.add_argument("--launcher", default="qwen",
                    help="the argv prefix that starts the session, shell-split; "
                         "`qwen` for a plain run, `<vibe> agent qwen` for a carrier run")
    ap.add_argument("--model", default="fake-1")
    ap.add_argument("--no-auth-flags", action="store_true")
    ap.add_argument("--wait-file", default=None,
                    help="also stop as soon as this file exists and is non-empty")
    ap.add_argument("rest", nargs=argparse.REMAINDER)
    a = ap.parse_args()
    extra = a.rest[1:] if a.rest[:1] == ["--"] else a.rest

    os.makedirs(a.out, exist_ok=True)
    os.makedirs(os.path.join(a.home, ".qwen"), exist_ok=True)

    penv = dict(os.environ, CAPTURE_DIR=a.out)
    if a.spawn:
        import json
        agent_args = {
            "subagent_type": a.spawn_type,
            "description": "canary check",
            "prompt": ("Report whether you can see any word of the form CANARY-... "
                       "anywhere in your context, and whether you can see a line "
                       "containing vibe:lane-begin. Answer in one sentence and read no file."),
            "run_in_background": False,
        }
        if a.spawn_type == "fork":
            agent_args.pop("run_in_background")
        # `agent` is a deferred tool on 0.27.2: review it, then call it through
        # the bridge. `--spawn-direct` calls it by name instead (headless/yolo
        # shapes where it is declared).
        if a.spawn_direct:
            script = [["agent", agent_args]]
        else:
            script = [["tool_search", {"query": "select:agent"}],
                      ["tool_call", {"name": "agent", "arguments": agent_args}]]
        penv["SCRIPT"] = json.dumps(script)
        penv["MAIN_MARKER"] = a.marker
    if a.script:
        penv["SCRIPT"] = a.script
        penv["MAIN_MARKER"] = a.marker
    provider = subprocess.Popen(
        [sys.executable, "-I", os.path.join(HERE, "qwen-identity-fake-provider.py"), str(a.port)], env=penv)
    time.sleep(0.8)

    env = {
        "PATH": os.environ["PATH"],
        "HOME": a.home,
        "QWEN_HOME": os.path.join(a.home, ".qwen"),
        "TERM": "xterm-256color" if a.mode == "tty" else "dumb",
        "NO_COLOR": "1",
    }
    for item in a.env:
        key, _, value = item.partition("=")
        env[key] = value

    argv = shlex.split(a.launcher)
    if not a.no_auth_flags:
        argv += ["--auth-type", "openai", "--openai-api-key", "not-a-real-key",
                 "--openai-base-url", f"http://127.0.0.1:{a.port}/v1", "-m", a.model]
    argv += extra

    log = open(os.path.join(a.out, "tty.log"), "wb")
    deadline = time.time() + a.timeout
    status = "timeout"

    def done():
        if a.wait_file and os.path.exists(a.wait_file) and os.path.getsize(a.wait_file) > 0:
            return True
        return request_count(a.out) >= a.want

    if a.mode == "headless":
        argv += ["-p", a.prompt]
        with open(os.devnull) as devnull:
            proc = subprocess.Popen(argv, cwd=a.project, env=env, stdin=devnull,
                                    stdout=log, stderr=subprocess.STDOUT)
        while time.time() < deadline:
            if proc.poll() is not None:
                status = f"exit {proc.returncode}"
                break
            if done():
                # let the run finish on its own if it can; the answer is harmless
                try:
                    proc.wait(timeout=a.linger + 20)
                    status = f"exit {proc.returncode}"
                except subprocess.TimeoutExpired:
                    status = "captured (still running)"
                break
            time.sleep(0.2)
        if proc.poll() is None:
            proc.kill()
    else:
        if not a.type_prompt and not a.no_prompt:
            argv += ["-i", a.prompt]
        pid, fd = pty.fork()
        if pid == 0:
            os.chdir(a.project)
            os.execvpe(argv[0], argv, env)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
        typed = False
        seen = b""
        confirm_from = 0
        confirm_at = None
        confirms = 0
        last_output = time.time()
        started = time.time()
        finished_at = None
        while time.time() < deadline:
            r, _, _ = select.select([fd], [], [], 0.2)
            if r:
                try:
                    data = os.read(fd, 65536)
                except OSError:
                    status = "pty closed"
                    break
                if not data:
                    status = "pty eof"
                    break
                log.write(data)
                log.flush()
                last_output = time.time()
                seen += data
                # An approval dialog: accept the preselected "Yes, allow once".
                if b"Do you want to proceed?" in seen[confirm_from:]:
                    confirm_from = len(seen)
                    confirm_at = time.time() + 1.0
            if confirm_at and time.time() >= confirm_at:
                os.write(fd, b"\r")
                confirm_at = None
                confirms += 1
            if (a.type_prompt and not typed and time.time() - started > 4
                    and time.time() - last_output > 2.0):
                os.write(fd, a.prompt.encode())
                time.sleep(0.5)
                os.write(fd, b"\r")
                typed = True
            if done():
                if finished_at is None:
                    finished_at = time.time()
                elif time.time() - finished_at > a.linger:
                    status = "captured"
                    break
        try:
            os.kill(pid, signal.SIGTERM)
            time.sleep(0.5)
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        try:
            os.waitpid(pid, 0)
        except ChildProcessError:
            pass

    log.close()
    provider.terminate()
    try:
        provider.wait(timeout=5)
    except subprocess.TimeoutExpired:
        provider.kill()
    print(f"[qrun] launcher={a.launcher!r} mode={a.mode} status={status} "
          f"requests={request_count(a.out)} out={a.out}")


if __name__ == "__main__":
    main()
