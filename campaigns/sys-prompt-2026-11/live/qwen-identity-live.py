#!/usr/bin/env python3
"""qwen-identity-live.py --project DIR --out DIR --prompt TEXT [--launcher ARGV]
                       [--env K=V ...] [-- qwen args]

One LIVE interactive Qwen Code session under a pseudo-terminal, with the
caller's real environment and configuration. It waits for the session's own
transcript to show a finished assistant turn, then ends the session and prints
what the model answered and every tool it called.

Nothing secret is read or printed: the transcript is the session's own, and
only the assistant text, the model id and the tool-call names are shown.
"""
import argparse
import fcntl
import glob
import json
import os
import pty
import select
import shlex
import signal
import struct
import sys
import termios
import time


def project_key(cwd):
    return "".join(c if c.isalnum() else "-" for c in cwd)


def read_transcript(path):
    entries = []
    try:
        for line in open(path, encoding="utf-8"):
            try:
                entries.append(json.loads(line))
            except ValueError:
                pass
    except OSError:
        pass
    return entries


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--project", required=True)
    ap.add_argument("--launcher", default="qwen",
                    help="the argv prefix that starts the session, shell-split")
    ap.add_argument("--out", required=True)
    ap.add_argument("--prompt", required=True)
    ap.add_argument("--env", action="append", default=[])
    ap.add_argument("--timeout", type=float, default=420)
    ap.add_argument("--settle", type=float, default=4.0)
    ap.add_argument("rest", nargs=argparse.REMAINDER)
    a = ap.parse_args()
    extra = a.rest[1:] if a.rest[:1] == ["--"] else a.rest
    os.makedirs(a.out, exist_ok=True)
    project = os.path.realpath(a.project)

    env = dict(os.environ)
    env["TERM"] = "xterm-256color"
    for item in a.env:
        key, _, value = item.partition("=")
        env[key] = value
    home = os.path.join(env.get("QWEN_HOME") or os.path.join(env["HOME"], ".qwen"))
    chats = os.path.join(home, "projects", project_key(project), "chats")
    before = set(glob.glob(os.path.join(chats, "*.jsonl")))

    argv = shlex.split(a.launcher) + extra + ["-i", a.prompt]
    started = time.time()
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(project)
        os.execvpe(argv[0], argv, env)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
    log = open(os.path.join(a.out, "tty.log"), "wb")
    transcript = None
    finished_at = None
    status = "timeout"
    seen = b""
    confirm_from = 0
    while time.time() - started < a.timeout:
        r, _, _ = select.select([fd], [], [], 0.3)
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
            seen += data
            if b"Do you want to proceed?" in seen[confirm_from:]:
                # The probe asks for no tool at all; a dialog means the model
                # tried one anyway. Decline it and let the transcript say so.
                confirm_from = len(seen)
                os.write(fd, b"\x1b")
        if transcript is None:
            new = sorted(set(glob.glob(os.path.join(chats, "*.jsonl"))) - before)
            if new:
                transcript = new[-1]
        if transcript:
            entries = read_transcript(transcript)
            last_assistant = None
            for e in entries:
                if e.get("type") == "assistant":
                    last_assistant = e
            if last_assistant is not None:
                parts = (last_assistant.get("message") or {}).get("parts") or []
                text_only = any("text" in p and p["text"].strip() and not p.get("thought")
                                for p in parts) \
                    and not any("functionCall" in p for p in parts)
                mtime = os.path.getmtime(transcript)
                if text_only and time.time() - mtime > a.settle:
                    status = "answered"
                    break
    try:
        os.kill(pid, signal.SIGTERM)
        time.sleep(0.7)
        os.kill(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    try:
        os.waitpid(pid, 0)
    except ChildProcessError:
        pass
    log.close()

    print(f"[qlive] status={status} after {time.time() - started:.0f}s transcript={'yes' if transcript else 'none'}")
    if not transcript:
        return
    entries = read_transcript(transcript)
    with open(os.path.join(a.out, "transcript.jsonl"), "w", encoding="utf-8") as f:
        for e in entries:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    calls = []
    answer = []
    models = set()
    for e in entries:
        parts = (e.get("message") or {}).get("parts") or []
        if e.get("type") == "assistant":
            if e.get("model"):
                models.add(e["model"])
            for p in parts:
                if "functionCall" in p:
                    fc = p["functionCall"]
                    calls.append((fc.get("name"), json.dumps(fc.get("args"), ensure_ascii=False)[:160]))
                elif "text" in p and not p.get("thought"):
                    answer.append(p["text"])
    print(f"[qlive] model(s): {sorted(models)}   qwen version: {entries[0].get('version') if entries else '?'}")
    print(f"[qlive] tool calls: {calls if calls else 'NONE'}")
    print("[qlive] assistant text:")
    print("\n".join(answer))


if __name__ == "__main__":
    main()
