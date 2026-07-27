# `flow:conventional-commits` — the commit message format {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-INSTALLS-THE-CONVENTIONAL-COMMITS-MESSAGE-FORMAT A vibevm `flow` package that installs the [Conventional Commits](https://www.conventionalcommits.org/)
message format: a typed header (`type(scope): subject`) and a body that explains *why* a change
was made, not *what* the diff already shows. @impl/done

##THIS-IS-THE-MESSAGE-FORMAT-ONLY This is the message **format** only. @impl/done

##ATOMICITY-IS-THE-SEPARATE-ATOMIC-COMMITS-PACKAGE The complementary discipline — **atomicity**, one commit =
one logical idea — is the separate `flow:atomic-commits` package. @impl/done

##EACH-DISCIPLINE-IS-ADOPTABLE-ON-ITS-OWN You can follow this format and
still write a non-atomic commit (`feat: add foo, bar, and baz`), and you can be atomic without
this format; the two run together, and each is its own package so a project can adopt either. @impl/done

##package-contents-lead This package ships: @impl/done

- ##CONTENT-THE-FULL-FORMAT `spec/flows/conventional-commits/conventional-commits.md` — the full format: header shape,
  the allowed-type table, scope convention, body structure, worked examples, and anti-patterns. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/31-flow-conventional-commits.md` — the boot snippet loaded at session start. @impl/done

## Install {#install}

```bash
vibe install flow:conventional-commits
```

## Composition {#composition}

- ##COMPOSES-ATOMIC-COMMITS Pairs with `flow:atomic-commits` (atomicity) — together they are the commit-message half of a
  `git-practices` posture. @impl/done

## License {#license}

##license-line UPL-1.0 — see `LICENSE`. @impl/done
