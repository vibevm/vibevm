# Flow: Conventional Commits {#root}

Every commit message follows the [Conventional Commits](https://www.conventionalcommits.org/)
specification: a **typed header** and a body that explains *why*.

## Header

```
type(scope): short imperative subject line
```

- Subject **≤ 60 characters** (hard limit 72), imperative mood, lowercase after the prefix.
- `type` is one of `feat` `fix` `chore` `docs` `build` `test` `refactor` `perf` `style`
  `ci` `revert`.
- `scope` names the **narrowest accurate** subsystem (a crate, package, module, or area).

## Body

A blank line after the subject, then a free-form body that answers *why*, not *what* — the
diff already shows what changed. Cite `spec://…` URIs where relevant. Full format — the
allowed-type table, scope rules, body structure, worked examples, and anti-patterns — is in
[`spec/flows/conventional-commits/conventional-commits.md`](../flows/conventional-commits/conventional-commits.md).

## Never

- Never write a subject that summarises *what* changed — write *why*.
- Never capitalise the first word after the `type(scope):` prefix, and never omit the type.

## Note — format is not atomicity

Conventional Commits is the message **format**; it does not by itself enforce **atomicity**
(one commit = one logical idea). A `feat: add foo, bar, and baz` message is valid Conventional
Commits *and* a violation of the atomic rule. Atomicity is the separate `atomic-commits` flow;
the two run together.
