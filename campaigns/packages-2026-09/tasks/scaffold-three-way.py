#!/usr/bin/env python3
"""C4's instrument: the nine scaffold cards, three languages, one diff.

Usage:
    python tasks/scaffold-three-way.py [--anchors|--words] [<letter> …]

The three `-lang` stacks each ship `spec/cards/scaffold-{a..i}-*.md`, and eight of
the nine carry **exactly 13 anchors in every language** — 467 anchors, 17 % of the
ai-native cluster, in three near-copies. Reading them one language at a time asks a
reviewer to hold the other two in their head; diffing them asks nothing.

The premise, and the reason this is evidence rather than tidiness: **these files
are a projection of one language-neutral scaffold**. Where they agree, one reading
covers three. Where they diverge WITHOUT a language reason, the divergence is the
finding — a rule that exists in Rust and not in Go is either a Go gap or a Rust
overreach, and only the cards can say which.

Two views:

  --anchors  which anchors each language's card carries. A missing anchor is a
             missing claim, and it is the loudest signal here.
  --words    for the anchors all three share, the word-level difference, with the
             three languages' own names filtered out — so `syn` vs `go/parser` does
             not read as drift while a changed rule does.
"""

import re, sys, pathlib, collections, difflib

ROOT = pathlib.Path(__file__).resolve().parents[3]
STACKS = {
    "rust": "packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/cards",
    "ts": "packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/cards",
    "go": "packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/cards",
}
# Names that legitimately differ per projection. Filtered from the word view so a
# language's own vocabulary cannot read as a divergence.
LANG_WORDS = {
    "rust", "cargo", "clippy", "syn", "rustc", "rustdoc", "crate", "crates", "rs",
    "typescript", "ts", "tsc", "eslint", "prettier", "node", "npm", "zod", "valibot",
    "vitest", "twoslash", "js", "jsdoc", "swc",
    "go", "golang", "gofmt", "vet", "staticcheck", "gopls", "exhaustive", "goroutine",
    "ai", "native", "lang", "v0", "1", "6", "7",
}
WORD = re.compile(r"[^\W_]+", re.UNICODE)
ANCHOR = re.compile(r"##([A-Za-z][A-Za-z0-9_-]*)")


# C7's trio is the same shape one directory over: three near-identical MCP briefs,
# one per language, and F-116 was filed from reading them side by side. The same
# instrument answers it, so it is named here rather than re-implemented.
TRIOS = {
    "discipline-mcp": {
        "rust": "packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/spec/tools/discipline-mcp-rust.md",
        "ts": "packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/spec/tools/discipline-mcp-typescript.md",
        "go": "packages/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/spec/tools/discipline-mcp-go.md",
    },
}


def cards(letter):
    if letter in TRIOS:
        return {lang: ROOT / p for lang, p in TRIOS[letter].items() if (ROOT / p).is_file()}
    out = {}
    for lang, d in STACKS.items():
        hits = sorted((ROOT / d).glob(f"scaffold-{letter}-*.md"))
        if hits:
            out[lang] = hits[0]
    return out


def anchors_of(path):
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"(?ms)^([`~]{3,}).*?^\1\s*$", "", text)     # fenced code is not markup
    seen, order = set(), []
    for m in ANCHOR.finditer(text):
        if m.group(1) not in seen:
            seen.add(m.group(1))
            order.append(m.group(1))
    return order, text


def body(text, anchor):
    """An anchor's own text: from its `##ID` to the next `##ID` or the block end."""
    blocks = re.split(r"\r?\n\s*\r?\n", text)
    for b in blocks:
        pos = [(m.start(), m.group(1)) for m in ANCHOR.finditer(b)]
        for i, (s, name) in enumerate(pos):
            if name == anchor:
                e = pos[i + 1][0] if i + 1 < len(pos) else len(b)
                return b[s:e]
    return ""


def content_words(s):
    return [w for w in WORD.findall(s.lower()) if w not in LANG_WORDS]


def main():
    view = "--words" if "--words" in sys.argv else "--anchors"
    letters = [a for a in sys.argv[1:] if not a.startswith("--")] or list("abcdefghi")
    total_missing = total_diverged = 0
    print(f"three-way scaffold diff — {len(letters)} card(s) × {len(STACKS)} languages, view {view}\n")
    for letter in letters:
        c = cards(letter)
        if len(c) < len(STACKS):
            print(f"scaffold-{letter}: MISSING in {sorted(set(STACKS) - set(c))}")
            continue
        parsed = {lang: anchors_of(p) for lang, p in c.items()}
        sets = {lang: set(a) for lang, (a, _) in parsed.items()}
        union = set().union(*sets.values())
        shared = set.intersection(*sets.values())
        counts = ", ".join(f"{lang}={len(sets[lang])}" for lang in STACKS)
        odd = {a: sorted(l for l in STACKS if a in sets[l]) for a in sorted(union - shared)}
        print(f"scaffold-{letter}: {counts}; shared {len(shared)}, divergent {len(odd)}")
        for a, langs in odd.items():
            missing = sorted(set(STACKS) - set(langs))
            print(f"    only in {'+'.join(langs):16} missing from {'+'.join(missing):12} — {a}")
            total_missing += 1
        if view == "--words":
            for a in sorted(shared):
                streams = {lang: content_words(body(parsed[lang][1], a)) for lang in STACKS}
                if len({tuple(s) for s in streams.values()}) == 1:
                    continue
                total_diverged += 1
                base = streams["rust"]
                print(f"    WORDS DIFFER  {a}")
                for lang in ("ts", "go"):
                    d = [l for l in difflib.unified_diff(base, streams[lang], lineterm="", n=0)
                         if l[:1] in "+-" and l[:3] not in ("---", "+++")]
                    only_r = " ".join(l[1:] for l in d if l[0] == "-")[:110]
                    only_l = " ".join(l[1:] for l in d if l[0] == "+")[:110]
                    print(f"        rust-only vs {lang}: {only_r or '(none)'}")
                    print(f"        {lang}-only        : {only_l or '(none)'}")
        print()
    print(f"anchors present in some languages and not others: {total_missing}")
    if view == "--words":
        print(f"shared anchors whose content words differ:        {total_diverged}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
