"""A loopback OpenAI-compatible provider that records request bodies."""
import json, os, sys
from http.server import BaseHTTPRequestHandler, HTTPServer

OUT = os.environ["CAPTURE_DIR"]
N = [0]

class H(BaseHTTPRequestHandler):
    def _read(self):
        n = int(self.headers.get("content-length", "0") or 0)
        return self.rfile.read(n)

    def do_POST(self):
        body = self._read()
        N[0] += 1
        # Headers are written WITHOUT authorization: a captured auth header
        # must never reach a file that could be committed.
        safe = {k: v for k, v in self.headers.items() if k.lower() != "authorization"}
        with open(os.path.join(OUT, f"req-{N[0]:03d}.json"), "wb") as f:
            f.write(body)
        with open(os.path.join(OUT, f"req-{N[0]:03d}.headers.json"), "w") as f:
            json.dump({"path": self.path, "headers": safe}, f, indent=2)
        answer = {
            "id": "chatcmpl-fake", "object": "chat.completion", "created": 0,
            "model": "fake-1",
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": "FAKE-OK"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
        }
        out = json.dumps(answer).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

    def do_GET(self):
        out = json.dumps({"data": [{"id": "fake-1", "object": "model"}], "object": "list"}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

    def log_message(self, *a):
        pass

HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
