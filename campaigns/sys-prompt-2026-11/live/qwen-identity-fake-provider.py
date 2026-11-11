"""A loopback OpenAI-compatible provider that records every request body and
can drive a scripted sequence of tool calls for the MAIN session only.

A sibling of `fake-provider-subagent.py`, adapted to Qwen Code 0.27.2's tool
surface, where `agent` is a *deferred* tool: the model reviews it with
`tool_search` and invokes it through `tool_call`. So spawning a subagent
takes a two-step script rather than one tool call.

A request belongs to the main session when one of its user messages carries
MAIN_MARKER; step k of the script is emitted when such a request holds
exactly k tool-result messages. Every other request (the subagent's own, the
memory-extraction side agent, title generation, ...) gets plain text.

It answers Server-Sent Events when the request asks for `stream: true` and a
plain JSON completion otherwise. It binds 127.0.0.1 only, calls no model, and
drops the `authorization` header before writing anything.

env: CAPTURE_DIR   where request bodies are written (required)
     SCRIPT        JSON list of [tool-name, arguments-object] steps (optional)
     MAIN_MARKER   text that marks the main session's user turn (optional)
usage: fakeprov.py <port>
"""
import json
import os
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

OUT = os.environ["CAPTURE_DIR"]
SCRIPT = json.loads(os.environ.get("SCRIPT", "[]"))
MAIN_MARKER = os.environ.get("MAIN_MARKER", "")
LOCK = threading.Lock()
N = [0]
USAGE = {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}


def chunk(delta, finish=None, usage=None):
    body = {
        "id": "chatcmpl-fake",
        "object": "chat.completion.chunk",
        "created": 0,
        "model": "fake-1",
        "choices": [{"index": 0, "delta": delta, "finish_reason": finish}],
    }
    if usage:
        body["usage"] = usage
    return f"data: {json.dumps(body)}\n\n".encode()


def message_text(message):
    content = message.get("content")
    if content is None:
        return ""
    if isinstance(content, str):
        return content
    return "\n".join(p.get("text", "") if isinstance(p, dict) else str(p) for p in content)


class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def _read(self):
        n = int(self.headers.get("content-length", "0") or 0)
        return self.rfile.read(n)

    def do_POST(self):
        raw = self._read()
        with LOCK:
            N[0] += 1
            index = N[0]
        safe = {k: v for k, v in self.headers.items() if k.lower() != "authorization"}
        with open(os.path.join(OUT, f"req-{index:03d}.json"), "wb") as f:
            f.write(raw)
        with open(os.path.join(OUT, f"req-{index:03d}.headers.json"), "w") as f:
            json.dump({"path": self.path, "headers": safe}, f, indent=2)

        try:
            body = json.loads(raw)
        except ValueError:
            body = {}
        messages = body.get("messages", []) or []
        is_main = bool(MAIN_MARKER) and any(
            m.get("role") == "user" and MAIN_MARKER in message_text(m) for m in messages)
        tool_results = sum(1 for m in messages if m.get("role") == "tool")
        step = SCRIPT[tool_results] if is_main and tool_results < len(SCRIPT) else None

        tool_call = None
        if step:
            tool_call = {
                "index": 0,
                "id": f"call_vibe_{tool_results + 1}",
                "type": "function",
                "function": {"name": step[0], "arguments": json.dumps(step[1])},
            }

        if body.get("stream"):
            parts = [chunk({"role": "assistant"})]
            if tool_call:
                parts.append(chunk({"tool_calls": [tool_call]}))
                parts.append(chunk({}, "tool_calls", USAGE))
            else:
                parts.append(chunk({"content": "FAKE-OK"}))
                parts.append(chunk({}, "stop", USAGE))
            parts.append(b"data: [DONE]\n\n")
            out = b"".join(parts)
            ctype = "text/event-stream"
        else:
            message = {"role": "assistant", "content": "FAKE-OK"}
            finish = "stop"
            if tool_call:
                message = {"role": "assistant", "content": None, "tool_calls": [
                    {k: v for k, v in tool_call.items() if k != "index"}]}
                finish = "tool_calls"
            out = json.dumps({
                "id": "chatcmpl-fake", "object": "chat.completion", "created": 0,
                "model": "fake-1",
                "choices": [{"index": 0, "finish_reason": finish, "message": message}],
                "usage": USAGE,
            }).encode()
            ctype = "application/json"

        self.send_response(200)
        self.send_header("content-type", ctype)
        self.send_header("cache-control", "no-cache")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

    def do_GET(self):
        out = json.dumps(
            {"data": [{"id": "fake-1", "object": "model"}], "object": "list"}
        ).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

    def log_message(self, *a):
        pass


ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
