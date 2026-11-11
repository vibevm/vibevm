#!/usr/bin/env python3
"""qwen-identity-prove.py <plain-dir> <carrier-dir> <lane-file> [<second-carrier-dir>]

The COMBINED-request proof for Qwen Code's QWEN_SYSTEM_IDENTITY_MD carrier
(PROP-076 `#SYS-QWEN`), on the raw bytes the provider received. Requests are
paired by arrival order; both runs must be the same session shape (same
flags, model, cwd, terminal).

(1) every carrier request equals its plain counterpart except for ONE
    contiguous insertion (main-session requests) or not at all (the rest);
    the insertion's offset and length are reported for the raw body and for
    the decoded system message;
(2) the inserted bytes are AGENT-LANE.md byte for byte, followed by one LF;
(3) the seam on both sides is printed;
(4) with a second carrier run: the two launches' first requests are
    byte-identical from byte 0 through the end of the lane;
(5) a non-main request (subagent, classifier, memory extractor) is
    byte-identical to its plain counterpart and carries no lane byte.
"""
import glob
import hashlib
import json
import os
import sys

LANE_CANARIES = [
    b"CANARY-LANE-HEAD-OBSIDIAN",
    b"CANARY-LANE-TAIL-PUMICE",
    b"CANARY-INDEX-CORE-BASALT",
    b"CANARY-INDEX-USER-GNEISS",
]


def load(directory):
    return [(os.path.basename(f), open(f, "rb").read())
            for f in sorted(glob.glob(os.path.join(directory, "req-*.json")))
            if not f.endswith(".headers.json")]


def system_of(body):
    parts = []
    for m in body.get("messages", []):
        if m.get("role") == "system":
            c = m.get("content")
            pieces = c if isinstance(c, list) else [c]
            parts.extend(p.get("text", "") if isinstance(p, dict) else p for p in pieces)
    return "\n".join(parts)


def single_insertion(a, b):
    """If b is a with one contiguous run inserted, return (offset, inserted)."""
    if len(b) < len(a):
        return None
    prefix = 0
    limit = len(a)
    while prefix < limit and a[prefix] == b[prefix]:
        prefix += 1
    suffix = 0
    while suffix < len(a) - prefix and a[len(a) - 1 - suffix] == b[len(b) - 1 - suffix]:
        suffix += 1
    if prefix + suffix != len(a):
        return None
    return prefix, b[prefix:len(b) - suffix]


def sha(data):
    return hashlib.sha256(data).hexdigest()[:16]


def show(data):
    return repr(data.decode("utf-8", "replace"))


def main():
    plain_dir, carrier_dir, lane_file = sys.argv[1:4]
    second_dir = sys.argv[4] if len(sys.argv) > 4 else None
    lane = open(lane_file, "rb").read()
    lane_text = lane.decode("utf-8")
    assert lane.endswith(b"\n") and not lane.endswith(b"\n\n") and lane == lane.strip() + b"\n", \
        "the lane must end with exactly one LF and carry no other edge whitespace"
    plain, carrier = load(plain_dir), load(carrier_dir)
    print(f"AGENT-LANE.md: {len(lane)} bytes, sha256 {sha(lane)}")
    print(f"plain run: {len(plain)} requests   carrier run: {len(carrier)} requests")
    ok = len(plain) == len(carrier) and len(plain) > 0
    mains = others = 0
    first_main = None
    for (name, praw), (_, craw) in zip(plain, carrier):
        p, c = json.loads(praw), json.loads(craw)
        ps, cs = system_of(p).encode(), system_of(c).encode()
        head = ps.split(b"\n", 1)[0][:58].decode("utf-8", "replace")
        if praw == craw:
            others += 1
            leaked = [k.decode() for k in LANE_CANARIES if k in craw]
            # The lane's own first and last lines. (The managed block in
            # AGENTS.md quotes the marker FORMAT in prose, so the bare words
            # `vibe:lane-begin` occur in a plain run too and prove nothing.)
            marker = (craw.count(lane.split(b"\n", 1)[0])
                      + craw.count(lane.rstrip(b"\n").rsplit(b"\n", 1)[-1]))
            print(f"\n{name}  NOT MAIN  [{head}…]")
            print(f"  (5) raw body {len(craw)} B, sha {sha(craw)}: BYTE-IDENTICAL to the plain run's; "
                  f"lane canaries: {leaked or 'none'}; lane marker lines: {marker}")
            ok = ok and not leaked and marker == 0
            continue
        mains += 1
        raw = single_insertion(praw, craw)
        dec = single_insertion(ps, cs)
        print(f"\n{name}  MAIN  [{head}…]")
        if raw is None or dec is None:
            print("  FAIL: the carrier request is not the plain request plus one contiguous insertion")
            ok = False
            continue
        raw_off, raw_ins = raw
        dec_off, dec_ins = dec
        rest_equal = (json.dumps({**p, "messages": [m for m in p["messages"] if m["role"] != "system"]},
                                 sort_keys=True)
                      == json.dumps({**c, "messages": [m for m in c["messages"] if m["role"] != "system"]},
                                    sort_keys=True))
        tools_equal = json.dumps(p.get("tools")) == json.dumps(c.get("tools"))
        print(f"  (1) raw body {len(praw)} B -> {len(craw)} B: ONE contiguous insertion of "
              f"{len(raw_ins)} raw bytes at raw offset {raw_off}; nothing else differs")
        print(f"      system message {len(ps)} B -> {len(cs)} B: one insertion of {len(dec_ins)} bytes "
              f"at system offset {dec_off}")
        print(f"      tools array identical: {tools_equal} ({len(p.get('tools') or [])} tools); "
              f"every non-system message and every parameter identical: {rest_equal}")
        exact = dec_ins == lane + b"\n"
        try:
            raw_exact = json.loads(b'"' + raw_ins + b'"').encode() == lane + b"\n"
        except ValueError:
            raw_exact = False
        print(f"  (2) inserted bytes == AGENT-LANE.md ({len(lane)} B) + one LF: {exact} "
              f"(raw insertion JSON-decodes to the same bytes: {raw_exact}); "
              f"lane occurs x{cs.count(lane.rstrip())} in the system message, "
              f"x{craw.count(json.dumps(lane_text.rstrip(), ensure_ascii=False)[1:-1].encode())} in the raw body")
        first_line = lane.split(b"\n", 1)[0]
        last_line = lane.rstrip(b"\n").rsplit(b"\n", 1)[-1]
        print(f"      first marker line intact: {cs.count(first_line) == 1}   "
              f"last marker line intact: {cs.count(last_line) == 1}")
        end = dec_off + len(dec_ins)
        print(f"  (3) seam before: {show(cs[dec_off - 70:dec_off])}")
        print(f"                   | {show(cs[dec_off:dec_off + 60])}")
        print(f"      seam after:  {show(cs[end - 62:end])}")
        print(f"                   | {show(cs[end:end + 40])}")
        ok = ok and exact and raw_exact and rest_equal and tools_equal and cs.count(lane.rstrip()) == 1
        if first_main is None:
            first_main = (name, craw, raw_off + len(raw_ins))
    print(f"\nmain-session requests: {mains}; other requests: {others}")
    if second_dir:
        second = load(second_dir)
        name, craw, lane_end = first_main
        sraw = dict(second)[name]
        common = 0
        while common < min(len(craw), len(sraw)) and craw[common] == sraw[common]:
            common += 1
        through = common >= lane_end
        print(f"\n(4) two consecutive launches, {name}: bodies {len(craw)} B and {len(sraw)} B; "
              f"byte-identical for the first {common} bytes; the lane ends at raw byte {lane_end}; "
              f"identical through the end of the lane: {through}"
              + ("; the WHOLE request is identical" if craw == sraw else
                 f"; first difference at {common}: {show(craw[common - 30:common + 50])} vs "
                 f"{show(sraw[common - 30:common + 50])}"))
        s1 = system_of(json.loads(craw)).encode()
        s2 = system_of(json.loads(sraw)).encode()
        print(f"    system message identical across the two launches: {s1 == s2} "
              f"(sha {sha(s1)} / {sha(s2)}); tools identical: "
              f"{json.loads(craw).get('tools') == json.loads(sraw).get('tools')}")
        ok = ok and through
    print("\nVERDICT:", "PASS" if ok and mains > 0 else "FAIL")
    sys.exit(0 if ok and mains > 0 else 1)


if __name__ == "__main__":
    main()
