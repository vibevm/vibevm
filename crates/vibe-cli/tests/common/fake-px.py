#!/usr/bin/env python3
"""A fake px and the CONNECT-proxy fixture of the vibe-cli tests (PROP-068#proxy).

One forwarding proxy on numeric loopback: CONNECT tunnels and absolute-form
HTTP requests. Every requested host is mapped to 127.0.0.1, so
`vibe-fixture.test:<port>` reaches a local test server while a direct
connection to that name fails DNS -- which is how a test proves the traffic
went through the proxy.

px-compatible flags: --listen=, --port= (0 = pick one and print
{"port": N} on stdout), --test=<url> (exit 0, or 1 under FAKE_PX_TEST_FAIL=1);
--foreground, --pac, --proxy, --noproxy and anything else are accepted and
ignored. Fixture-only flags: --require-auth (answer 407 to everything),
--request-log=<file> (append each request line).

Environment: FAKE_PX_LOG=<file> gets `start <pid>` / `exit <pid>` lines;
FAKE_PX_REQUESTS=<file> is a default for --request-log; FAKE_PX_FAIL=1 makes
it exit 3 at once with a message on stderr.
"""
import json
import os
import select
import signal
import socket
import sys
import threading

args = sys.argv[1:]


def flag(name, default=None):
    for arg in args:
        if arg == name:
            return ""
        if arg.startswith(name + "="):
            return arg.split("=", 1)[1]
    return default


def note(line):
    path = os.environ.get("FAKE_PX_LOG")
    if path:
        with open(path, "a") as f:
            f.write(line + "\n")


if os.environ.get("FAKE_PX_FAIL") == "1":
    sys.stderr.write("fake px: configured to fail (FAKE_PX_FAIL=1)\n")
    sys.exit(3)

test = flag("--test")
if test is not None:
    if os.environ.get("FAKE_PX_TEST_FAIL") == "1":
        print("fake px: test of " + test + " failed")
        sys.exit(1)
    print("fake px: test of " + test + " passed")
    sys.exit(0)

listen = flag("--listen", "127.0.0.1")
port = int(flag("--port", "3128"))
require_auth = flag("--require-auth") is not None
request_log = flag("--request-log", os.environ.get("FAKE_PX_REQUESTS"))
lock = threading.Lock()


def log_request(line):
    if request_log:
        with lock, open(request_log, "a") as f:
            f.write(line + "\n")


def pump(a, b):
    sockets = [a, b]
    try:
        while True:
            ready, _, _ = select.select(sockets, [], [], 30)
            if not ready:
                return
            for s in ready:
                data = s.recv(65536)
                if not data:
                    return
                (b if s is a else a).sendall(data)
    except OSError:
        return


def read_head(conn):
    data = b""
    while b"\r\n\r\n" not in data:
        chunk = conn.recv(4096)
        if not chunk:
            return None, b""
        data += chunk
        if len(data) > 65536:
            return None, b""
    head, rest = data.split(b"\r\n\r\n", 1)
    return head.decode("latin-1"), rest


def handle(conn):
    try:
        head, rest = read_head(conn)
        if head is None:
            return
        lines = head.split("\r\n")
        request = lines[0]
        log_request(request)
        if require_auth:
            conn.sendall(
                b"HTTP/1.1 407 Proxy Authentication Required\r\n"
                b"Proxy-Authenticate: Negotiate\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            return
        method, target, version = request.split(" ", 2)
        if method == "CONNECT":
            host, _, tport = target.rpartition(":")
            try:
                upstream = socket.create_connection(("127.0.0.1", int(tport)), timeout=10)
            except OSError:
                conn.sendall(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n")
                return
            conn.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
            if rest:
                upstream.sendall(rest)
            pump(conn, upstream)
            upstream.close()
            return
        # Absolute-form request: http://host:port/path
        if not target.startswith("http://"):
            conn.sendall(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            return
        authority, _, path = target[len("http://"):].partition("/")
        _, _, tport = authority.rpartition(":")
        tport = int(tport) if tport.isdigit() else 80
        upstream = socket.create_connection(("127.0.0.1", tport), timeout=10)
        forwarded = [method + " /" + path + " " + version]
        forwarded += [l for l in lines[1:] if not l.lower().startswith("proxy-")]
        upstream.sendall(("\r\n".join(forwarded) + "\r\n\r\n").encode("latin-1") + rest)
        pump(conn, upstream)
        upstream.close()
    except (OSError, ValueError):
        pass
    finally:
        try:
            conn.close()
        except OSError:
            pass


def main():
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind((listen, port))
    server.listen(64)
    pid = os.getpid()
    note("start " + str(pid))

    def stop(*_):
        note("exit " + str(pid))
        os._exit(0)

    signal.signal(signal.SIGTERM, stop)
    if port == 0:
        print(json.dumps({"port": server.getsockname()[1]}), flush=True)
    while True:
        conn, _ = server.accept()
        threading.Thread(target=handle, args=(conn,), daemon=True).start()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        note("exit " + str(os.getpid()))
