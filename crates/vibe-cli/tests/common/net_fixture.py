#!/usr/bin/env python3
"""Local TLS / HTTP servers and test certificate authorities for the
vibe-cli network tests (PROP-068#proxy). Numeric loopback only.

  net_fixture.py ca <dir> <label>       -> CA + leaf in <dir>, prints JSON paths
  net_fixture.py https <cert> <key> <root> <log>
  net_fixture.py http <root> <log>

A server prints {"port": N} once it listens and stops when stdin closes.
Routes: /redirect/<path> -> 302 to /<path>; /status/<code> -> that status; /slow/<s> -> a body
trickled for <s> seconds;
/git/<repo>/... -> git http-backend over <root>/git (smart HTTP);
anything else -> the file under <root>.
"""
import functools
import http.server
import json
import os
import pathlib
import ssl
import subprocess
import sys
import threading
import urllib.parse

env = {"PATH": "/usr/bin:/bin", "LC_ALL": "C", "HOME": "/nonexistent",
       "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null"}


def make_ca(directory, label):
    d = pathlib.Path(directory)
    d.mkdir(parents=True, exist_ok=True)
    ca, ca_key = d / (label + "-ca.pem"), d / (label + "-ca-key.pem")
    key, csr, leaf, ext = d / (label + "-key.pem"), d / (label + ".csr"), d / (label + "-cert.pem"), d / (label + ".ext")
    ext.write_text(
        "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\n"
        "extendedKeyUsage=serverAuth\nsubjectAltName=DNS:vibe-fixture.test,DNS:localhost,IP:127.0.0.1\n")
    for argv in [
        ["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", str(ca_key), "-out", str(ca), "-days", "2",
         "-subj", "/CN=Vibe Fixture " + label + " CA/O=vibe-fixture", "-addext", "basicConstraints=critical,CA:TRUE",
         "-addext", "keyUsage=critical,keyCertSign,cRLSign"],
        ["req", "-newkey", "rsa:2048", "-nodes", "-keyout", str(key), "-out", str(csr), "-subj", "/CN=vibe-fixture.test"],
        ["x509", "-req", "-in", str(csr), "-CA", str(ca), "-CAkey", str(ca_key), "-CAcreateserial", "-out", str(leaf),
         "-days", "2", "-extfile", str(ext)],
    ]:
        p = subprocess.run(["openssl", *argv], env=env, capture_output=True, timeout=30)
        if p.returncode:
            raise SystemExit("openssl failed: " + p.stderr.decode())
    print(json.dumps({"ca_pem": str(ca), "cert": str(leaf), "key": str(key)}), flush=True)


class Handler(http.server.SimpleHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    # Fixed so a captured response head is the same on every machine.
    server_version = "vibe-fixture/1"
    sys_version = ""

    def date_time_string(self, timestamp=None):
        return "Thu, 01 Jan 2026 00:00:00 GMT"

    def log_message(self, *a):
        pass

    def record(self):
        with open(self.server.request_log, "a") as f:
            f.write(self.command + " " + self.path + " host=" + str(self.headers.get("Host")) + "\n")

    def git(self):
        parsed = urllib.parse.urlsplit(self.path)
        body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        child = dict(env)
        child.update({"GIT_PROJECT_ROOT": str(pathlib.Path(self.directory) / "git"), "GIT_HTTP_EXPORT_ALL": "1",
                      "PATH_INFO": parsed.path[len("/git"):], "QUERY_STRING": parsed.query,
                      "REQUEST_METHOD": self.command, "CONTENT_TYPE": self.headers.get("Content-Type", ""),
                      "CONTENT_LENGTH": str(len(body))})
        p = subprocess.run(["git", "http-backend"], env=child, input=body, capture_output=True, timeout=60)
        head, data = p.stdout.split(b"\r\n\r\n", 1) if b"\r\n\r\n" in p.stdout else (b"", p.stdout)
        fields = [l.decode().split(":", 1) for l in head.split(b"\r\n") if b":" in l]
        status = next((int(v.strip().split()[0]) for k, v in fields if k.lower() == "status"), 200)
        self.send_response(status)
        for k, v in fields:
            if k.lower() != "status":
                self.send_header(k, v.strip())
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def simple(self, code, location=None):
        self.send_response(code)
        if location:
            self.send_header("Location", location)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_POST(self):
        self.record()
        if self.path.startswith("/git/"):
            return self.git()
        self.simple(405)

    def do_GET(self):
        self.record()
        if self.path.startswith("/git/"):
            return self.git()
        if self.path.startswith("/redirect/"):
            return self.simple(302, "/" + self.path[len("/redirect/"):])
        if self.path.startswith("/status/"):
            return self.simple(int(self.path[len("/status/"):].split("?")[0]))
        if self.path.startswith("/slow/"):
            # /slow/<seconds>: 100 KB announced, 1 KB every 50 ms.
            seconds = float(self.path[len("/slow/"):])
            self.send_response(200)
            self.send_header("Content-Length", "102400")
            self.end_headers()
            import time
            deadline = time.time() + seconds
            sent = 0
            while sent < 102400 and time.time() < deadline:
                self.wfile.write(b"s" * 1024)
                self.wfile.flush()
                sent += 1024
                time.sleep(0.05)
            return
        return super().do_GET()

    def do_HEAD(self):
        self.record()
        return super().do_HEAD()

    def handle(self):
        try:
            super().handle()
        except (ConnectionResetError, BrokenPipeError, ssl.SSLError):
            pass


class Server(http.server.ThreadingHTTPServer):
    daemon_threads = True

    def handle_error(self, request, client_address):
        pass


def serve(root, log, tls=None):
    server = Server(("127.0.0.1", 0), functools.partial(Handler, directory=root))
    server.request_log = log
    if tls:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(tls[0], tls[1])
        server.socket = context.wrap_socket(server.socket, server_side=True, do_handshake_on_connect=False)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(json.dumps({"port": server.server_address[1]}), flush=True)
    try:
        sys.stdin.read()
    finally:
        server.shutdown()


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "ca":
        make_ca(sys.argv[2], sys.argv[3])
    elif mode == "https":
        serve(sys.argv[4], sys.argv[5], (sys.argv[2], sys.argv[3]))
    elif mode == "http":
        serve(sys.argv[2], sys.argv[3])
    else:
        raise SystemExit("unknown mode " + mode)
