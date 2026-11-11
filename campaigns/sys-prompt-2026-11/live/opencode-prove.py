"""Judge the five captures of `opencode-prove.sh` against conditions (0) and
(i)-(v) of `PROP-076#SYS-SUPPLIED-BASE`.

Every answer is a comparison over bytes, printed with its numbers, and the
last line is a verdict a tool can read. Nothing here is a heuristic: a
condition that cannot be evaluated FAILS rather than being skipped, because a
proof that quietly proves four things out of five is how `verified = true`
comes to mean less than it says.
"""
import glob
import hashlib
import json
import os
import sys

LANE_PATH, OUT = sys.argv[1], sys.argv[2]
LANE = open(LANE_PATH, encoding="utf-8").read()
CARRIER = json.load(open(os.path.join(OUT, "carrier.json"), encoding="utf-8"))

# The members a carrier launch must not change. `messages` is compared
# separately, because exactly one of them — the system one — is where the
# whole claim lives.
COMPARED = ["model", "stream", "tools", "max_completion_tokens", "stream_options",
            "temperature", "top_p", "response_format", "tool_choice"]

# The prompt the fake subagent provider hands the spawning tool, which is how
# a child request is identified by what it IS rather than by its position.
CHILD_MARK = "whether you can see a line containing vibe:lane-begin"

failures = []


def check(condition, name, detail):
    print(f"  [{'PASS' if condition else 'FAIL'}] {name}: {detail}")
    if not condition:
        failures.append(name)
    return condition


def bodies(leg):
    """Every recorded request body of one leg, in capture order."""
    files = [f for f in sorted(glob.glob(os.path.join(OUT, leg, "req-*.json")))
             if not f.endswith(".headers.json")]
    return [json.load(open(f, encoding="utf-8")) for f in files]


def texts(message):
    content = message["content"]
    pieces = content if isinstance(content, list) else [content]
    return [piece["text"] if isinstance(piece, dict) else piece for piece in pieces
            if not isinstance(piece, dict) or "text" in piece]


def blob(body, role="system"):
    """One role's whole text, joined the way the capture reports it."""
    parts = []
    for message in body["messages"]:
        if message["role"] == role:
            parts.extend(str(part) for part in texts(message))
    return "\n".join(parts)


def first(leg, role="system"):
    """The leg's last request — the one carrying the whole prompt; the first
    is OpenCode's own title call, which has a prompt of its own."""
    found = bodies(leg)
    return found[-1] if found else None


def insertion(plain, carried):
    """The one contiguous span `carried` has and `plain` does not, as (offset,
    length) in BYTES — or None when the difference is not one insertion.

    **Stated as an equation, not as a diff.** The offset is the two blobs'
    longest common prefix and the length is their size difference; the claim
    is then the identity

        carried[:offset] + carried[offset + length:] == plain

    which is the only reading under which "one contiguous insertion and
    nothing else changed" is a fact rather than an interpretation. A
    line-oriented diff would place the boundary wherever its heuristics
    preferred and the answer would be a matter of taste.
    """
    pb, cb = plain.encode(), carried.encode()
    if len(cb) <= len(pb):
        return None
    offset = 0
    while offset < len(pb) and pb[offset] == cb[offset]:
        offset += 1
    length = len(cb) - len(pb)
    if cb[:offset] + cb[offset + length:] != pb:
        return None
    return offset, length


def base_from_carrier():
    """The base vibe put in front of the lane, and the separator between
    them, out of the carrier document.

    Read back from the document rather than recomposed, so what is checked is
    what was actually handed over.
    """
    document = json.loads(CARRIER["value"])
    agents = document["agents"]
    agent = agents[next(iter(agents))]
    system = agent["system"]
    head, _, _ = system.rpartition("{file:")
    base = head.rstrip("\n")
    return base, head[len(base):]


def markers():
    lines = LANE.splitlines()
    # The marker lines are HTML comments in a Markdown lane, so the token is
    # inside the line rather than at its start.
    begin = next(line for line in lines if "vibe:lane-begin" in line)
    end = [line for line in lines if "vibe:lane-end" in line][-1]
    return begin, end


print(f"lane                : {LANE_PATH}")
print(f"lane bytes          : {len(LANE.encode())}")
print(f"carrier env         : {CARRIER['env']}")
print(f"carrier flags       : {' '.join(CARRIER['flags'])}")
print(f"carrier doc bytes   : {len(CARRIER['value'].encode())}")

plain_body, carried_body = first("plain"), first("carrier")
again_body = first("carrier2")
if plain_body is None or carried_body is None or again_body is None:
    print("\nNO REQUEST CAPTURED for one of plain/carrier/carrier2 — nothing is proved")
    print("VERDICT: FAIL")
    raise SystemExit(1)

plain = blob(plain_body)
carried = blob(carried_body)
again = blob(again_body)
base, separator = base_from_carrier()

print(f"\nplain system bytes  : {len(plain.encode())}")
print(f"carrier system bytes: {len(carried.encode())}")
print(f"supplied base bytes : {len(base.encode())}  sha256 {hashlib.sha256(base.encode()).hexdigest()}")

# ---------------------------------------------------------------------------
print("\n(0) the reproduced base is the installed program's own")
# The plain run's prompt begins with OpenCode's own base. If the base vibe
# supplied is not byte-equal to it, the carrier launch is running on
# instructions OpenCode did not write.
check(plain.encode().startswith(base.encode()),
      "base equals the installed program's own first bytes",
      f"plain run's first {len(base.encode())} bytes "
      f"{'match' if plain.encode().startswith(base.encode()) else 'DIFFER'}")
check(carried.encode().startswith(base.encode()),
      "the carrier request begins with that same base",
      "the supplied base is at offset 0")

# ---------------------------------------------------------------------------
print("\n(i) one contiguous insertion and nothing else changed")
span = insertion(plain, carried)
if not check(span is not None, "the difference is ONE contiguous insertion",
             "carried[:offset] + carried[offset+length:] == plain"
             if span else
             "the identity carried[:offset] + carried[offset+length:] == plain does NOT hold, "
             "so something outside the insertion changed"):
    offset, length = 0, 0
    inserted = ""
else:
    offset, length = span
    inserted = carried.encode()[offset:offset + length].decode("utf-8", "replace")
    print(f"       insertion offset {offset}, length {length}, "
          f"plain bytes in that window 0")
    check(offset == len(base.encode()) + len(separator.encode()),
          "the insertion begins where the supplied base and the recorded separator end",
          f"offset {offset} = base {len(base.encode())} + separator "
          f"{len(separator.encode())} ({separator!r})")
for member in COMPARED:
    if member in plain_body or member in carried_body:
        check(plain_body.get(member) == carried_body.get(member),
              f"`{member}` identical", "equal" if plain_body.get(member) == carried_body.get(member)
              else f"{plain_body.get(member)!r} vs {carried_body.get(member)!r}")
others_plain = [m for m in plain_body["messages"] if m["role"] != "system"]
others_carried = [m for m in carried_body["messages"] if m["role"] != "system"]
check(others_plain == others_carried, "every non-system message identical",
      f"{len(others_plain)} message(s), "
      f"{'equal' if others_plain == others_carried else 'DIFFERENT'}")
check(sorted(plain_body) == sorted(carried_body), "the same request members are present",
      f"{len(plain_body)} members")

# ---------------------------------------------------------------------------
print("\n(ii) the inserted bytes are the lane under the documented transform only")
# Two named transformations act on the lane and they cancel: `{file:}` trims
# the inserted content's leading and trailing whitespace, and the harness's
# own join between the agent's `system` part and the next part puts a newline
# back. Either outcome is admissible and the report says WHICH, because
# "the inserted bytes equal the lane" and "equal the lane minus its trailing
# newline" are different facts about the same mechanism.
if inserted == LANE:
    check(True, "inserted == AGENT-LANE.md exactly",
          f"all {len(LANE.encode())} bytes — `{{file:}}` trims the lane's "
          f"{len(LANE.encode()) - len(LANE.strip().encode())} trailing byte(s) and the "
          f"harness's own part join restores as many, so the delivered window is the file "
          f"byte for byte")
else:
    check(inserted == LANE.strip(),
          "inserted == the lane with leading/trailing whitespace trimmed",
          f"{len(inserted.encode())} of {len(LANE.encode())} bytes; "
          f"{len(LANE.encode()) - len(inserted.encode())} dropped at the ends "
          f"({'the documented `{file:}` trim' if inserted == LANE.strip() else 'MISMATCH'})")
begin_marker, end_marker = markers()
check(inserted.count(begin_marker) == 1, "the first marker line arrives intact",
      f"{begin_marker[:60]}… x{inserted.count(begin_marker)}")
check(inserted.count(end_marker) == 1, "the last marker line arrives intact",
      f"{end_marker[:60]}… x{inserted.count(end_marker)}")
check(carried.count(LANE.strip()) == 1, "the lane arrives exactly once", "x1")

print("\n(iii) the seam, in both directions")
at = carried.find(begin_marker)
after = carried.find(end_marker) + len(end_marker)
before_seam = carried[max(0, at - 60):at]
after_seam = carried[after:after + 60]
print(f"       before the first marker: {before_seam!r}")
print(f"       after the last marker  : {after_seam!r}")
check(before_seam.endswith("\n\n"), "a blank line separates the base from the first marker",
      "the base's closing sentence is not glued to a marker line")
check(after_seam.startswith("\n"), "the last marker line ends at a newline",
      "what follows the lane starts on its own line")

# ---------------------------------------------------------------------------
print("\n(iv) two consecutive launches are byte-identical through the end of the lane")
lane_end_offset = len(carried[:after].encode())
same_prefix = carried.encode()[:lane_end_offset] == again.encode()[:lane_end_offset]
check(same_prefix, "the prefix up to the end of the lane is byte-identical",
      f"{lane_end_offset} bytes, "
      f"sha256 {hashlib.sha256(carried.encode()[:lane_end_offset]).hexdigest()[:16]} vs "
      f"{hashlib.sha256(again.encode()[:lane_end_offset]).hexdigest()[:16]}")
differs_at = next((i for i in range(min(len(carried.encode()), len(again.encode())))
                   if carried.encode()[i] != again.encode()[i]), None)
check(differs_at is None or differs_at >= lane_end_offset,
      "every byte that differs between the two launches sits AFTER the lane",
      "the two launches are byte-identical throughout" if differs_at is None
      else f"first difference at byte {differs_at}, lane ends at {lane_end_offset}")

# ---------------------------------------------------------------------------
print("\n(v) a subagent carries the lane zero times and is otherwise its own control")
def child(leg):
    for body in bodies(leg):
        if CHILD_MARK in blob(body, "user"):
            return body
    return None

child_with, child_plain = child("child-with"), child("child-plain")
if not check(child_with is not None and child_plain is not None,
             "a real child session was created in both legs",
             f"with-carrier {'yes' if child_with else 'NO'}, "
             f"control {'yes' if child_plain else 'NO'}"):
    pass
else:
    with_blob, plain_blob = blob(child_with), blob(child_plain)
    check(with_blob.count(LANE.strip()) == 0, "the child carries the lane ZERO times",
          f"x{with_blob.count(LANE.strip())}, {len(with_blob.encode())} bytes")
    check(with_blob == plain_blob,
          "the child's request is byte-identical to its own plain counterpart",
          f"{len(with_blob.encode())} vs {len(plain_blob.encode())} bytes, "
          f"{'equal' if with_blob == plain_blob else 'DIFFERENT'}")
    for member in COMPARED:
        if member in child_with or member in child_plain:
            check(child_with.get(member) == child_plain.get(member),
                  f"child `{member}` identical", "equal")

# ---------------------------------------------------------------------------
proof = hashlib.sha256(
    (hashlib.sha256(carried.encode()).hexdigest()
     + hashlib.sha256(plain.encode()).hexdigest()
     + hashlib.sha256(LANE.encode()).hexdigest()).encode()
).hexdigest()[:16]
print(f"\nproof id            : opencode-whole-request-{proof}")
print(f"failed conditions   : {len(failures)}"
      + (f" — {', '.join(failures)}" if failures else ""))
print(f"VERDICT: {'PASS' if not failures else 'FAIL'}")
raise SystemExit(0 if not failures else 1)
