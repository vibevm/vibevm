"""The tracked campaign state: pure transforms, and one all-or-nothing write."""

import json
import os
import pathlib


def byte_sorted(keys):
    """Sorted the way a Rust `BTreeMap<String, _>` serialises: by UTF-8 bytes."""
    return sorted(keys, key=lambda s: s.encode("utf-8"))


def dumps(obj):
    """The byte-for-byte spelling `serde_json::to_string_pretty` produces:
    two-space indent, `": "` after a key, raw UTF-8, no trailing newline.
    Verified against all three tracked artifacts before the first write —
    re-serialising them unchanged reproduces them byte for byte."""
    return json.dumps(obj, indent=2, ensure_ascii=False).encode("utf-8")


def migrate_cache(cache, mapping):
    files = cache["files"]
    moved = {mapping[old]: files[old] for old in files}
    return dict(cache, files={k: moved[k] for k in byte_sorted(moved)})


def migrate_corpus_state(corpus, mapping):
    """`run/state/corpus.json` is derived from the cache and re-emitted by the
    engine on its next write; it is migrated anyway, so that `--apply` alone
    leaves no tracked file naming a path that does not exist."""
    rows = [dict(row, path=mapping[row["path"]]) for row in corpus["files"]]
    rows.sort(key=lambda r: r["path"].encode("utf-8"))
    return dict(corpus, files=rows)


def publish(writes):
    """Write every artifact or none of them.

    Two phases: stage every body beside its target and fsync it, then rename. A
    failure while staging leaves the tree untouched; a failure while renaming
    restores the bytes read before the first rename. Staging files are removed on
    every path out, including the failing ones.
    """
    before = {path: path.read_bytes() for path in writes if path.exists()}
    staged = []
    try:
        for path, body in writes.items():
            tmp = path.with_suffix(path.suffix + ".migrate~")
            with open(tmp, "wb") as fh:
                fh.write(body)
                fh.flush()
                os.fsync(fh.fileno())
            staged.append((tmp, path))
        done = []
        try:
            for tmp, path in staged:
                os.replace(tmp, path)
                done.append(path)
        except OSError:
            for path in done:
                if path in before:
                    path.write_bytes(before[path])
            raise
    finally:
        for tmp, _ in staged:
            if tmp.exists():
                tmp.unlink()


def zone_paths(root, zone):
    """The five artifacts a campaign zone carries that this package reads."""
    root, zone = pathlib.Path(root), pathlib.Path(zone)
    return {
        "root": root,
        "zone": zone,
        "cache": zone / "run" / "cache.json",
        "baseline": zone / "baseline.json",
        "corpus": zone / "run" / "state" / "corpus.json",
        "journal": zone / "run" / "journal.jsonl",
        "mirror": zone / "run" / "mirror",
    }
