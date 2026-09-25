# Changelog of the VibeVM manual {#root}

What changed for a reader, by date. This package carries one number and is republished under it in place, so a date and not a number says which edition a line belongs to. Written by hand from `JOURNAL.md`; the product's own versions are named where a change follows them.

## 1.0.0 — 2026-09-25

- **The manual reads like a textbook.** The list beside a page follows a learning path: ten chapters from what VibeVM is and installing vibe, through everyday work, agents, the lifecycle and writing packages, to the appendices, with the glossary last. A switch above the list brings back the grouping by section, and every page ends with the previous and the next page on the path.

## 1.0.0 — 2026-09-14

What is new for a reader since the pages were first laid out:

- **The pages open in the dark, and a switch keeps the other theme.** The choice is remembered on the reader's own machine and is sent nowhere.
- **Search finds a page by a word from it.** The box reaches the title and the leading fact of every page and of every documentation on the site; it asks no search service, because the index is part of the site.
- **A card says what it stands for.** It now wears the mark of its package's kind, the hand that wrote its prose, and, for a bridge, the two authorships kept apart — where one drawing stood before that distinguished nothing.
- **Keeping `vibe` current is written down.** `self update` follows where the running copy came from, `self reinstall` fetches the running version again without changing which one it is, and `self rollback` goes back.
- **The manual reads in Russian.** The adaptation is a package of its own, `vibevm-docs-ru`, mirroring these pages block for block in its own sentences.

## 1.0.0 — the first edition

The first complete manual of VibeVM 1.0, forty-nine pages in English:

- **Start** — what VibeVM is, installing vibe, a first project, what a project contains.
- **The model** — the two trees, the boot lane, packages and their eight kinds, registries, versions, the lock file and the store, dependency visibility.
- **How-to** — setting up a workspace, working offline, private registries, publishing a package, the lifecycle from build to deploy.
- **Agents** — asking your agent to do the work, giving it the vibevm skill, how agents read this manual.
- **Authoring** — writing packages of every kind, flows, feats and stacks, language packages, tools and MCP servers, documentation and its adaptations, bridging a repository, facts and status markers, the specifications agents can cite.
- **Reference** — every command's help, the manifest and the lock file, machine formats, the dependency tree, the diagnostics and their fixes.
- **Architecture** — how vibe is built, traceability between specs and code, what the lifecycle epic delivered, how this manual is maintained.
- **Glossary and questions.**

Every task page opens with the request you give a coding agent; every example on these pages is run against the real binary before a release; every rule quotes the specification by its address.

Not in this edition: the Russian adaptation (it follows as its own package), images for the package card (placeholders stand in), and the third level of command help (`vibe doc build --help` and its siblings are described in prose).
