"""Staged PTY driver for attended `vibe ai setup`; synthetic inputs only.

argv: <vibe> <json list of [expected, answer]>. Each answer is typed once
its expected text appears after the previous answer. Prints one JSON object.
"""
import json
import os
import pty
import select
import signal
import subprocess
import sys
import termios
import time

cli = sys.argv[1]
steps = json.loads(sys.argv[2])
master, slave = pty.openpty()
original = termios.tcgetattr(slave)


def child_terminal():
    os.setsid()
    import fcntl
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


process = subprocess.Popen([cli, 'ai', 'setup'], stdin=slave, stdout=slave, stderr=slave, preexec_fn=child_terminal, close_fds=True)
seen = bytearray()
mark = 0
sent = 0
started = time.monotonic()
while process.poll() is None:
    if time.monotonic() - started > 90:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        break
    readable, _, _ = select.select([master], [], [], 0.1)
    if not readable:
        continue
    try:
        data = os.read(master, 16384)
    except OSError:
        break
    seen.extend(data)
    if sent < len(steps) and steps[sent][0].encode() in seen[mark:]:
        time.sleep(0.3)
        os.write(master, steps[sent][1].encode())
        mark = len(seen)
        sent += 1
process.wait(timeout=5)
while select.select([master], [], [], 0)[0]:
    try:
        data = os.read(master, 16384)
    except OSError:
        break
    if not data:
        break
    seen.extend(data)
restored = termios.tcgetattr(slave)
os.close(master)
os.close(slave)
print(json.dumps({'status': process.returncode, 'sent': sent, 'terminal_restored': restored == original, 'transcript': seen.decode('utf-8', 'replace')}))
