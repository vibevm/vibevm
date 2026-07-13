# `flow:conventional-commits` — the commit message format

A vibevm `flow` package that installs the [Conventional Commits](https://www.conventionalcommits.org/)
message format: a typed header (`type(scope): subject`) and a body that explains *why* a change
was made, not *what* the diff already shows.

This is the message **format** only. The complementary discipline — **atomicity**, one commit =
one logical idea — is the separate `flow:atomic-commits` package. You can follow this format and
still write a non-atomic commit (`feat: add foo, bar, and baz`), and you can be atomic without
this format; the two run together, and each is its own package so a project can adopt either.

This package ships:

- `spec/flows/conventional-commits/conventional-commits.md` — the full format: header shape,
  the allowed-type table, scope convention, body structure, worked examples, and anti-patterns.
- `spec/boot/31-flow-conventional-commits.md` — the boot snippet loaded at session start.

## Install

```bash
vibe install flow:conventional-commits
```

## Composition

- Pairs with `flow:atomic-commits` (atomicity) — together they are the commit-message half of a
  `git-practices` posture.

## License

UPL-1.0 — see `LICENSE`.
