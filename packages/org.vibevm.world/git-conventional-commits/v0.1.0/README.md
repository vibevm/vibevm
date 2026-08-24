# `flow:git-conventional-commits` — the commit message format {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-CONVENTIONAL-COMMITS-MESSAGE-FORMAT A vibevm `flow` package that installs the [Conventional Commits](https://www.conventionalcommits.org/)
message format: a typed header (`type(scope): subject`) and a body that explains *why* a change
was made, not *what* the diff already shows. @status:impl/done

@fact:THIS-IS-THE-MESSAGE-FORMAT-ONLY This is the message **format** only. @status:impl/done

@fact:ATOMICITY-IS-THE-SEPARATE-ATOMIC-COMMITS-PACKAGE The complementary discipline — **atomicity**, one commit =
one logical idea — is the separate `flow:git-atomic-commits` package. @status:impl/done

@fact:EACH-DISCIPLINE-IS-ADOPTABLE-ON-ITS-OWN You can follow this format and
still write a non-atomic commit (`feat: add foo, bar, and baz`), and you can be atomic without
this format; the two run together, and each is its own package so a project can adopt either. @status:impl/done

@fact:package-contents-lead This package ships: @status:impl/done

- @fact:CONTENT-THE-FULL-FORMAT `spec/flows/conventional-commits/conventional-commits.xml` — the full format: header shape,
  the allowed-type table, scope convention, body structure, worked examples, and anti-patterns. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/31-flow-conventional-commits.xml` — the boot snippet loaded at session start. @status:impl/done

## Install {#install}

```bash
vibe install flow:git-conventional-commits
```

## Composition {#composition}

- @fact:COMPOSES-ATOMIC-COMMITS Pairs with `flow:git-atomic-commits` (atomicity) — together they are the commit-message half of a
  `git-practices` posture. @status:impl/done

## License {#license}

@fact:license-line UPL-1.0 — see `LICENSE`. @status:impl/done

