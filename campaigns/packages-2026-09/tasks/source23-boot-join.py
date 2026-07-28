#!/usr/bin/env python3
"""§3.1 sources 2 and 3 for the `world` cluster: does the host actually receive,
and actually carry, what each flow's boot snippet says it does?

Usage:
    python tasks/source23-boot-join.py

§3.1 orders three sources. Source 1 is the package agreeing with itself. Source 2
is «the host's observed conformance» — the consuming project either behaves as the
flow promises or does not. Source 3 is «the installed reality» — what a consumer
receives. For a boot-lane flow both are answerable mechanically and together,
because the host's boot lane is COMPILED from the installed packages and carries a
provenance marker for every contribution:

    <!-- vibe:static org.vibevm.world/wal — vibedeps/flow-wal/0.2.0/spec/boot/10-flow-wal.md -->

That marker is the join. For each contribution this resolves three things:

  INSTALLED   the `vibedeps/` path the marker names exists — source 3, and the
              honest substitute for `files_written`, which is `[]` for all 36
              packages in the lockfile (batch plan §2.3)
  SOURCED     the same snippet exists in `packages/org.vibevm.world/<name>/<ver>/`
              — the shipped artifact the installed copy came from
  SAME-WORDS  the compiled text in the host's boot lane carries the same word
              stream as the package's source — the unit a re-split cannot change

A mismatch at IDENTICAL is the interesting one: it means the host is running a
different rule from the one the package ships, which is exactly the drift §3.1
source 2 exists to catch and which no amount of reading the package would find.

Exit 0 when every contribution resolves and matches, 1 otherwise.
"""

import re, sys, pathlib, difflib

ROOT = pathlib.Path(__file__).resolve().parents[3]
STATIC = ROOT / "spec" / "boot" / "STATIC.md"
MARKER = re.compile(r"^<!--\s*vibe:static\s+(\S+)\s+[—-]+\s+(\S+)\s*-->\s*$", re.M)

# The boot lane is a RENDERING of the snippet, not a copy of it: the spec
# compiler strips the progress markup on the way in, so the host carries the
# prose and none of the campaign's metadata. Comparing raw bytes reported all
# thirty-one contributions as drifting, which was a fact about the comparison.
STATUS_EL = re.compile(r"^<status[^>]*/>[ \t]*\r?\n?", re.M)
ANCHOR = re.compile(r"##[A-Za-z][A-Za-z0-9_-]*[ \t]+")
MARK = re.compile(r"[ \t]*@[a-z]+/[a-z]+(?=\s|$)")
HEAD_ANCHOR = re.compile(r"[ \t]*\{#[A-Za-z0-9_.:-]+\}")
BLANKS = re.compile(r"(\r?\n){3,}")


WORD = re.compile(r"[^\W_]+", re.UNICODE)


def words(text):
    """The text as a stream of words — the unit a re-split cannot change."""
    return WORD.findall(text.lower())


def strip_markup(text):
    """The snippet as a reader receives it: no status elements, no fact anchors,
    no stage/state markers."""
    text = STATUS_EL.sub("", text)
    text = ANCHOR.sub("", text)
    text = MARK.sub("", text)
    return BLANKS.sub("\n\n", text).strip()


def main():
    if not STATIC.is_file():
        print(f"REFUSED: {STATIC} does not exist — nothing to join against")
        return 1
    text = STATIC.read_text(encoding="utf-8")
    hits = list(MARKER.finditer(text))
    if not hits:
        print("REFUSED: no `vibe:static` provenance markers in the boot lane — "
              "an empty join is not a clean one")
        return 1

    rows, bad = [], 0
    for i, m in enumerate(hits):
        pkg, dep_path = m.group(1), m.group(2)
        body_start = m.end()
        body_end = hits[i + 1].start() if i + 1 < len(hits) else len(text)
        compiled = text[body_start:body_end].strip()

        installed = (ROOT / dep_path)
        state = []
        state.append("INSTALLED" if installed.is_file() else "NOT-INSTALLED")

        # The package source the installed copy came from: same tail path under
        # packages/<group>/<name>/<version>/.
        # `vibedeps/<slot>/<version>/<tail>` — four leading components, not three.
        # Taking three left the version duplicated in the glob and reported all
        # thirty-one contributions unsourced, which is the same class of bug as the
        # empty sweep in `source1-join.py`, wearing a louder failure.
        group, name = pkg.split("/", 1) if "/" in pkg else ("?", pkg)
        tail = dep_path.split("/", 3)[-1] if dep_path.count("/") >= 3 else dep_path
        cands = sorted((ROOT / "packages" / group / name).glob("v*/" + tail)) if (ROOT / "packages" / group / name).is_dir() else []
        src = cands[-1] if cands else None
        state.append("SOURCED" if src else "NO-SOURCE")

        if src:
            source_text = strip_markup(src.read_text(encoding="utf-8"))
            # Compare WORD STREAMS, not lines. The installed lane predates Phase B,
            # whose sense-preserving re-splits moved sentence boundaries without
            # changing a word; a line-wise compare reports every one of those as
            # drift and buries a real change among them. If the word streams agree,
            # the host is running exactly the rule the package ships.
            same = words(source_text) == words(compiled)
            state.append("SAME-WORDS" if same else "WORDS-DIFFER")
            if not same:
                bad += 1
        else:
            source_text = ""
            bad += 1
        if not installed.is_file():
            bad += 1
        # The STRIPPED text, not the raw file: the report must diff what the
        # comparison compared. Storing the raw text here made the report claim 247
        # differing words where the decision had been taken on 6.
        rows.append((pkg, dep_path, src, state, compiled, source_text if src else ""))

    print(f"boot-lane join over {len(rows)} contribution(s) in spec/boot/STATIC.md")
    ok = sum(1 for r in rows if r[3] == ["INSTALLED", "SOURCED", "SAME-WORDS"])
    print(f"  installed, sourced, same word stream: {ok}")
    print(f"  problems: {len(rows) - ok}")
    for pkg, dep, src, state, compiled, source_text in rows:
        if state == ["INSTALLED", "SOURCED", "SAME-WORDS"]:
            continue
        print()
        print(f"  {pkg}  [{' '.join(state)}]")
        print(f"    installed: {dep}")
        print(f"    source   : {src.relative_to(ROOT).as_posix() if src else '<none found>'}")
        if "WORDS-DIFFER" in state:
            a, b = words(source_text), words(compiled)
            d = [l for l in difflib.unified_diff(a, b, lineterm="", n=0)
                 if l[:1] in "+-" and l[:3] not in ("---", "+++")]
            only_pkg = [l[1:] for l in d if l[0] == "-"]
            only_host = [l[1:] for l in d if l[0] == "+"]
            print(f"    package {len(a)} words, host {len(b)} — {len(d)} differ")
            if only_pkg:
                print(f"    only in the package: {' '.join(only_pkg[:20])}"
                      + (f" … (+{len(only_pkg) - 20})" if len(only_pkg) > 20 else ""))
            if only_host:
                print(f"    only in the host   : {' '.join(only_host[:20])}"
                      + (f" … (+{len(only_host) - 20})" if len(only_host) > 20 else ""))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
