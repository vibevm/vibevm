#!/usr/bin/env python3
"""B-031 host-as-package migration (cut W2): spec://vibevm -> spec://org.vibevm.core/vibevm.

A textual, byte-exact migrator. Two replacement rules, applied in order:

  R1 (prefix, with slash):
      spec://vibevm/         -> spec://org.vibevm.core/vibevm/
  R2 (bare; regex with negative lookahead — matches when the char after
      `spec://vibevm` is NOT one of `/ [A-Za-z0-9] - _`, i.e. end-of-string,
      quote, paren, backtick, whitespace, `#`, `.`, ...):
      spec://vibevm          -> spec://org.vibevm.core/vibevm

Idempotent: the migrated form `spec://org.vibevm.core/vibevm` does NOT contain
the substring `spec://vibevm` (after `spec://` comes `o`, never `v`), so neither
rule re-matches already-migrated text.

Modes:
  (no flag)  dry-run : per-area table (files touched / occurrences),
                       top-20 files by occurrences, unified diff of the first
                       three touched files. No writes.
  --wet              : apply the migration (rewrite a file only if it changed).
  --verify           : post-wet residue check — count remaining `spec://vibevm`
                       on live surfaces; exit 1 if any remain, else 0.

Stdlib only. Reads/writes BYTES (rb/wb); replacement is on byte literals, so
UTF-8 is never re-encoded, CRLF/LF and BOM are preserved exactly.
"""

import argparse
import difflib
import os
import re
import sys

# --- perimeter ------------------------------------------------------------

# Repo root is derived from this script's location so the migrator is
# independent of the current working directory:
#   <repo>/campaigns/packages-2026-09/tasks/migrate-b031.py
_HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(os.path.dirname(_HERE)))

ROOTS = ("spec", "crates", "campaigns", "packages", "xtask", "tools")
EXTS = (".rs", ".md", ".toml", ".sh", ".ps1", ".py")
ROOT_EXTS = (".md", ".toml")  # top-level repo files only
EXCLUDE_DIRS = {
    "vibedeps", ".vibe", "refs", "fixtures", "legacy-spec",
    "run", "target", ".git", "node_modules", ".wt",
}
# Exclude the migrator itself by basename (its own literals must not be
# rewritten, or a second run would no longer find the source form).
EXCLUDE_FILES = {"migrate-b031.py"}

# --- rules (byte literals) ------------------------------------------------

R1_FROM = b"spec://vibevm/"
R1_TO = b"spec://org.vibevm.core/vibevm/"
# Bare form: `spec://vibevm` not followed by `/`, alnum, `-` or `_`.
R2_RE = re.compile(rb"spec://vibevm(?![A-Za-z0-9/_-])")
R2_TO = b"spec://org.vibevm.core/vibevm"

AREA_ORDER = ("spec", "crates", "campaigns", "packages", "tools", "xtask", "root")


# --- core transforms ------------------------------------------------------

def migrate(data):
    """Apply R1 then R2 to byte content."""
    data = data.replace(R1_FROM, R1_TO)
    data = R2_RE.sub(R2_TO, data)
    return data


def occ(data):
    """Occurrences the rules would change (R1 and R2 are disjoint on source)."""
    return data.count(R1_FROM) + len(R2_RE.findall(data))


def residue(data):
    """Any remaining raw `spec://vibevm` substring (post-wet should be 0)."""
    return data.count(b"spec://vibevm")


# --- file discovery -------------------------------------------------------

def collect_files():
    out = []
    for root_dir in ROOTS:
        base = os.path.join(REPO, root_dir)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            # prune excluded directories at any depth (mutate in place)
            dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
            for fn in filenames:
                if fn in EXCLUDE_FILES:
                    continue
                if os.path.splitext(fn)[1] not in EXTS:
                    continue
                out.append(os.path.join(dirpath, fn))
    # top-level repo files (*.md, *.toml)
    for fn in os.listdir(REPO):
        full = os.path.join(REPO, fn)
        if not os.path.isfile(full):
            continue
        if fn in EXCLUDE_FILES:
            continue
        if os.path.splitext(fn)[1] in ROOT_EXTS:
            out.append(full)
    return sorted(set(out))


def area_of(path):
    rel = os.path.relpath(path, REPO).replace(os.sep, "/")
    for r in ROOTS:
        if rel == r or rel.startswith(r + "/"):
            return r
    return "root"


def read_bytes(path):
    with open(path, "rb") as fh:
        return fh.read()


def write_bytes(path, data):
    with open(path, "wb") as fh:
        fh.write(data)


# --- modes ----------------------------------------------------------------

def cmd_dryrun():
    files = collect_files()
    per_area = {a: {"files": 0, "occ": 0} for a in AREA_ORDER}
    touched = []  # (occ, path)
    for path in files:
        try:
            data = read_bytes(path)
        except OSError as e:
            print("WARN: cannot read %s: %s" % (path, e), file=sys.stderr)
            continue
        n = occ(data)
        if n <= 0:
            continue
        a = area_of(path)
        per_area[a]["files"] += 1
        per_area[a]["occ"] += n
        touched.append((n, path))

    total_files = sum(v["files"] for v in per_area.values())
    total_occ = sum(v["occ"] for v in per_area.values())

    print("=== B-031 migration DRY-RUN (spec://vibevm -> spec://org.vibevm.core/vibevm) ===")
    print("repo: %s" % REPO)
    print("rules:")
    print("  R1  spec://vibevm/  -> spec://org.vibevm.core/vibevm/   (prefix, with slash)")
    print("  R2  spec://vibevm   -> spec://org.vibevm.core/vibevm    (bare; not before / [A-Za-z0-9] - _)")
    print("")
    print("per-area (files touched / occurrences):")
    print("  %-12s %6s %8s" % ("area", "files", "occ"))
    for a in AREA_ORDER:
        v = per_area[a]
        print("  %-12s %6d %8d" % (a + "/", v["files"], v["occ"]))
    print("  %-12s %6d %8d" % ("TOTAL", total_files, total_occ))
    print("")
    print("live total occurrences: %d  (census non-json expectation ~1851)" % total_occ)
    print("")

    top = sorted(touched, key=lambda t: (-t[0], t[1]))[:20]
    print("top-20 files by occurrences:")
    for n, path in top:
        print("  %6d  %s" % (n, os.path.relpath(path, REPO).replace(os.sep, "/")))
    print("")

    # unified diff of the first three touched files (by path)
    first3 = sorted((p for _, p in touched), key=lambda p: p.replace(os.sep, "/"))[:3]
    print("unified diff (first %d touched files by path):" % len(first3))
    for path in first3:
        try:
            data = read_bytes(path)
        except OSError:
            continue
        new = migrate(data)
        rel = os.path.relpath(path, REPO).replace(os.sep, "/")
        a_lines = data.decode("utf-8", "replace").splitlines(keepends=True)
        b_lines = new.decode("utf-8", "replace").splitlines(keepends=True)
        diff = difflib.unified_diff(a_lines, b_lines,
                                    fromfile="a/" + rel, tofile="b/" + rel, n=1)
        print("".join(diff), end="")
        if not diff:
            print("(no diff)")
    return 0


def cmd_wet():
    files = collect_files()
    rewritten = 0
    changed_occ = 0
    for path in files:
        try:
            data = read_bytes(path)
        except OSError as e:
            print("WARN: cannot read %s: %s" % (path, e), file=sys.stderr)
            continue
        n = occ(data)
        if n <= 0:
            continue
        new = migrate(data)
        if new != data:
            write_bytes(path, new)
            rewritten += 1
            changed_occ += n
    print("=== B-031 migration WET ===")
    print("repo: %s" % REPO)
    print("files rewritten: %d" % rewritten)
    print("occurrences changed: %d" % changed_occ)
    return 0


def cmd_verify():
    files = collect_files()
    per_area = {a: {"files": 0, "res": 0} for a in AREA_ORDER}
    for path in files:
        try:
            data = read_bytes(path)
        except OSError as e:
            print("WARN: cannot read %s: %s" % (path, e), file=sys.stderr)
            continue
        r = residue(data)
        if r <= 0:
            continue
        a = area_of(path)
        per_area[a]["files"] += 1
        per_area[a]["res"] += r
    total = sum(v["res"] for v in per_area.values())
    print("=== B-031 migration VERIFY (residue `spec://vibevm`) ===")
    print("repo: %s" % REPO)
    print("per-area residue:")
    print("  %-12s %6s %8s" % ("area", "files", "residue"))
    for a in AREA_ORDER:
        v = per_area[a]
        print("  %-12s %6d %8d" % (a + "/", v["files"], v["res"]))
    print("  %-12s %6s %8d" % ("TOTAL", "", total))
    if total == 0:
        print("result: CLEAN (exit 0)")
        return 0
    print("result: RESIDUE REMAINS (exit 1)")
    return 1


def main(argv=None):
    # Force UTF-8 stdout/stderr so diff display of non-ASCII (em-dash, Cyrillic)
    # does not crash on a cp1252 console. Migration itself is byte-level.
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass
    p = argparse.ArgumentParser(description="B-031 spec://vibevm migrator")
    p.add_argument("--wet", action="store_true", help="apply the migration")
    p.add_argument("--verify", action="store_true",
                   help="post-wet residue check (exit 1 if any remain)")
    args = p.parse_args(argv)
    if args.wet and args.verify:
        p.error("--wet and --verify are mutually exclusive")
    if args.verify:
        return cmd_verify()
    if args.wet:
        return cmd_wet()
    return cmd_dryrun()


if __name__ == "__main__":
    sys.exit(main())
