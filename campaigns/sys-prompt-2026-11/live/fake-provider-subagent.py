"""A loopback OpenAI-compatible provider that records requests AND drives one
subagent, so a child session really exists offline.

`opencode-fake-provider.py` answers one non-streaming JSON body, which is
enough to capture a primary request and nothing more: the question "does a
subagent inherit the lane" cannot be asked of a harness that never spawns
one. This one speaks Server-Sent Events, and on the first request that
offers a subagent-spawning tool it answers with a **tool call** instead of
text. The harness then creates the child session and sends its request, which
lands here as the next capture file.

Every request body is written in arrival order, so the primary and the child
can be compared: the lane must appear exactly once in the primary and zero
times in the child (`PROP-076#SYS-WORKER-EXCLUSION`).

It reaches nothing but the loopback address it is bound to, calls no model,
and drops the `authorization` header before writing, so a capture can never
carry a credential.

env: CAPTURE_DIR   where the request bodies are written (required)
     SPAWN_TOOL    the tool to call once, if the request offers it
                   (default `subagent`; OpenCode 2.2.7's name — it was
                   `task` in earlier versions)
     SPAWN_AGENT   the value of that tool's `agent` argument
                   (default `general`)
     SPAWN_PROMPT  the task handed to the child
usage: fake-provider-subagent.py <port>
"""
import json, os, sys
from http.server import BaseHTTPRequestHandler, HTTPServer

OUT = os.environ["CAPTURE_DIR"]
SPAWN_TOOL = os.environ.get("SPAWN_TOOL", "subagent")
SPAWN_AGENT = os.environ.get("SPAWN_AGENT", "general")
SPAWN_PROMPT = os.environ.get(
    "SPAWN_PROMPT",
    "Report whether you can see any word of the form CANARY-... anywhere in "
    "your context, and whether you can see a line containing vibe:lane-begin. "
    "Answer in one sentence and read no file.",
)
N = [0]
SPAWNED = [False]


def chunk(delta, finish=None):
    body = {
        "id": "chatcmpl-fake",
        "object": "chat.completion.chunk",
        "created": 0,
        "model": "fake-1",
        "choices": [{"index": 0, "delta": delta, "finish_reason": finish}],
    }
    return f"data: {json.dumps(body)}\n\n".encode()


class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def _read(self):
        n = int(self.headers.get("content-length", "0") or 0)
        return self.rfile.read(n)

    def do_POST(self):
        raw = self._read()
        N[0] += 1
        index = N[0]
        # Headers are written WITHOUT authorization: a captured auth header
        # must never reach a file that could be committed.
        safe = {k: v for k, v in self.headers.items() if k.lower() != "authorization"}
        with open(os.path.join(OUT, f"req-{index:03d}.json"), "wb") as f:
            f.write(raw)
        with open(os.path.join(OUT, f"req-{index:03d}.headers.json"), "w") as f:
            json.dump({"path": self.path, "headers": safe}, f, indent=2)

        try:
            body = json.loads(raw)
        except ValueError:
            body = {}
        offered = {
            tool.get("function", {}).get("name")
            for tool in body.get("tools", [])
            if isinstance(tool, dict)
        }
        # **One spawn, on the first request that can make one.** A second
        # would recurse: the child's own request offers the same tool.
        spawn = (
            not SPAWNED[0]
            and SPAWN_TOOL in offered
            and not any(message.get("role") == "tool" for message in body.get("messages", []))
        )

        parts = [chunk({"role": "assistant"})]
        if spawn:
            SPAWNED[0] = True
            arguments = json.dumps(
                {"agent": SPAWN_AGENT, "description": "canary check", "prompt": SPAWN_PROMPT}
            )
            parts.append(
                chunk(
                    {
                        "tool_calls": [
                            {
                                "index": 0,
                                "id": "call_vibe_1",
                                "type": "function",
                                "function": {"name": SPAWN_TOOL, "arguments": arguments},
                            }
                        ]
                    }
                )
            )
            parts.append(chunk({}, "tool_calls"))
        else:
            parts.append(chunk({"content": "FAKE-OK"}))
            parts.append(chunk({}, "stop"))
        parts.append(b"data: [DONE]\n\n")
        out = b"".join(parts)

        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
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


HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
